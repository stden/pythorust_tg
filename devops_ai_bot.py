#!/usr/bin/env python3
"""DevOps AI Assistant Telegram bot with monitoring and quick commands."""

import asyncio
import logging
import os
import shlex
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional

import aiohttp
import yaml
from dotenv import load_dotenv
from telethon import Button, TelegramClient, events
from telethon.tl.custom import Message

from integrations.openai_client import chat_completion

load_dotenv()

API_ID_ENV = os.getenv("TELEGRAM_API_ID")
API_HASH = os.getenv("TELEGRAM_API_HASH")
if not API_ID_ENV or not API_HASH:
    raise RuntimeError("TELEGRAM_API_ID and TELEGRAM_API_HASH are required for DevOps AI bot.")

API_ID = int(API_ID_ENV)
BOT_TOKEN = os.getenv("DEVOPS_BOT_TOKEN") or os.getenv("TASK_ASSISTANT_BOT_TOKEN") or os.getenv("TELEGRAM_BOT_TOKEN")
CONFIG_PATH = Path(os.getenv("DEVOPS_BOT_CONFIG", "devops_bot.yml"))

DEFAULT_SYSTEM_PROMPT = (
    "Ты — DevOps/Backend ассистент. Отвечай коротко и по шагам. "
    "Всегда предлагай безопасные команды, проверяй статус сервисов, помни про логи и порты. "
    "Не придумывай данные, если их нет."
)

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("DevOpsAIBot")


@dataclass
class ServiceConfig:
    name: str
    kind: str = "http"  # http | tcp | systemd | command
    url: Optional[str] = None
    expected_status: int = 200
    timeout: float = 10.0
    restart_command: Optional[str] = None
    log_command: Optional[str] = None
    status_command: Optional[str] = None
    tcp_host: Optional[str] = None
    tcp_port: Optional[int] = None


def _expand_env(value: Any) -> Any:
    """Recursively expand environment variables inside config."""
    if isinstance(value, dict):
        return {k: _expand_env(v) for k, v in value.items()}
    if isinstance(value, list):
        return [_expand_env(v) for v in value]
    if isinstance(value, str):
        return os.path.expandvars(value)
    return value


class DevOpsAIBot:
    """Telegram bot that monitors services and answers DevOps questions."""

    def __init__(self):
        if not BOT_TOKEN:
            raise RuntimeError("Set DEVOPS_BOT_TOKEN (or TELEGRAM_BOT_TOKEN) in the environment.")

        self.config = self._load_config()
        self.allowed_users = self._load_allowed_users(self.config)

        monitor_cfg = self.config.get("monitor", {})
        self.monitor_interval = int(monitor_cfg.get("interval_seconds") or os.getenv("DEVOPS_CHECK_INTERVAL", 300))
        self.cooldown = int(monitor_cfg.get("cooldown_seconds") or os.getenv("DEVOPS_ALERT_COOLDOWN", 300))

        alert_chat = (
            monitor_cfg.get("alert_chat_id") or os.getenv("DEVOPS_ALERT_CHAT_ID") or os.getenv("TELEGRAM_CHAT_ID")
        )
        self.alert_chat_id = int(alert_chat) if alert_chat else None
        self.monitor_enabled = monitor_cfg.get("enabled", True)

        self.ai_model = os.getenv("OPENAI_MODEL")
        self.services = self._load_services(self.config)
        self.last_status: Dict[str, bool] = {}
        self.last_alert_at: Dict[str, float] = {}

        self.client = TelegramClient("devops_ai_bot", API_ID, API_HASH)

    def _load_config(self) -> Dict[str, Any]:
        """Load YAML config if present."""
        if CONFIG_PATH.exists():
            raw = yaml.safe_load(CONFIG_PATH.read_text()) or {}
            return _expand_env(raw)
        return {}

    def _load_allowed_users(self, config: Dict[str, Any]) -> set[int]:
        ids: set[int] = set()

        cfg_users = config.get("bot", {}).get("allowed_users") or []
        for item in cfg_users:
            try:
                ids.add(int(item))
            except (TypeError, ValueError):
                continue

        env_value = os.getenv("DEVOPS_ALLOWED_USERS")
        if env_value:
            for chunk in env_value.split(","):
                chunk = chunk.strip()
                if not chunk:
                    continue
                try:
                    ids.add(int(chunk))
                except ValueError:
                    logger.warning("Cannot parse user id from DEVOPS_ALLOWED_USERS: %s", chunk)

        return ids

    def _load_services(self, config: Dict[str, Any]) -> Dict[str, ServiceConfig]:
        services_cfg = config.get("services") or {}
        services: Dict[str, ServiceConfig] = {}

        for name, raw in services_cfg.items():
            data = raw or {}
            kind = data.get("kind") or ("http" if data.get("url") else "systemd")
            url = data.get("url") or os.getenv(f"{name.upper()}_URL")
            tcp_host = data.get("tcp_host") or data.get("host")
            tcp_port = data.get("tcp_port")
            tcp_port = int(tcp_port) if tcp_port is not None else None

            status_command = data.get("status_command")
            if not status_command and kind == "systemd":
                service_name = data.get("service_name") or name
                status_command = f"systemctl is-active {shlex.quote(service_name)}"

            services[name] = ServiceConfig(
                name=name,
                kind=kind,
                url=url or None,
                expected_status=int(data.get("expected_status", 200)),
                timeout=float(data.get("timeout", 10.0)),
                restart_command=data.get("restart_command"),
                log_command=data.get("log_command"),
                status_command=status_command,
                tcp_host=tcp_host,
                tcp_port=tcp_port,
            )

        # Minimal fallback to avoid empty config
        if not services and os.getenv("N8N_URL"):
            services["n8n"] = ServiceConfig(
                name="n8n",
                kind="http",
                url=os.getenv("N8N_URL"),
                expected_status=200,
                timeout=float(os.getenv("DEVOPS_HTTP_TIMEOUT", "10")),
                restart_command=os.getenv("N8N_RESTART_COMMAND"),
                log_command=os.getenv("N8N_LOG_COMMAND", "journalctl -u n8n -n 100 --no-pager"),
            )

        return services

    def _is_allowed(self, user_id: Optional[int]) -> bool:
        if not self.allowed_users:
            return True
        if user_id is None:
            return False
        return user_id in self.allowed_users

    async def _send_alert(self, text: str) -> None:
        if not self.alert_chat_id:
            return
        try:
            await self.client.send_message(int(self.alert_chat_id), text)
        except Exception as exc:
            logger.error("Failed to send alert: %s", exc)

    async def _check_http(self, svc: ServiceConfig, session: aiohttp.ClientSession) -> Dict[str, Any]:
        start = time.perf_counter()
        try:
            async with session.get(svc.url, timeout=aiohttp.ClientTimeout(total=svc.timeout), ssl=False) as resp:
                latency_ms = int((time.perf_counter() - start) * 1000)
                ok = resp.status == svc.expected_status
                return {
                    "name": svc.name,
                    "ok": ok,
                    "detail": f"HTTP {resp.status}",
                    "latency_ms": latency_ms,
                }
        except Exception as exc:
            return {"name": svc.name, "ok": False, "detail": str(exc)}

    async def _check_tcp(self, svc: ServiceConfig) -> Dict[str, Any]:
        host = svc.tcp_host or "localhost"
        port = svc.tcp_port
        if port is None:
            return {"name": svc.name, "ok": False, "detail": "TCP port is not configured"}

        try:
            start = time.perf_counter()
            reader, writer = await asyncio.wait_for(asyncio.open_connection(host, port), timeout=svc.timeout)
            latency_ms = int((time.perf_counter() - start) * 1000)
            writer.close()
            await writer.wait_closed()
            return {"name": svc.name, "ok": True, "detail": f"TCP {host}:{port}", "latency_ms": latency_ms}
        except Exception as exc:
            return {"name": svc.name, "ok": False, "detail": str(exc)}

    async def _check_status_command(self, svc: ServiceConfig) -> Dict[str, Any]:
        if not svc.status_command:
            return {"name": svc.name, "ok": False, "detail": "Status command not configured"}

        code, stdout, stderr = await self._run_command(svc.status_command, timeout=svc.timeout)
        output = stdout.strip() or stderr.strip()
        ok = code == 0 and ("active" in output or "running" in output)
        return {"name": svc.name, "ok": ok, "detail": output or f"exit {code}"}

    async def check_service(
        self, svc: ServiceConfig, session: Optional[aiohttp.ClientSession] = None
    ) -> Dict[str, Any]:
        if svc.kind == "http":
            if not svc.url:
                return {"name": svc.name, "ok": False, "detail": "URL is not set"}
            if session:
                return await self._check_http(svc, session)
            async with aiohttp.ClientSession() as local_session:
                return await self._check_http(svc, local_session)

        if svc.kind == "tcp":
            return await self._check_tcp(svc)

        if svc.kind in {"systemd", "command"}:
            return await self._check_status_command(svc)

        return {"name": svc.name, "ok": False, "detail": "Unknown service kind"}

    async def _monitor_loop(self) -> None:
        if not self.services:
            logger.info("No services configured, monitor loop skipped.")
            return

        logger.info("Starting monitor loop for %d services", len(self.services))
        while True:
            try:
                async with aiohttp.ClientSession() as session:
                    for svc in self.services.values():
                        result = await self.check_service(svc, session=session)
                        prev = self.last_status.get(svc.name)
                        self.last_status[svc.name] = result["ok"]

                        now = time.time()
                        if not result["ok"]:
                            last_alert = self.last_alert_at.get(svc.name, 0)
                            if prev is None or prev is True or (now - last_alert) >= self.cooldown:
                                self.last_alert_at[svc.name] = now
                                await self._send_alert(f"❌ {svc.name}: {result['detail']}")
                        elif prev is False:
                            await self._send_alert(f"✅ {svc.name} восстановился ({result['detail']})")

                await asyncio.sleep(self.monitor_interval)
            except asyncio.CancelledError:
                raise
            except Exception as exc:
                logger.error("Monitor loop error: %s", exc)
                await asyncio.sleep(self.monitor_interval)

    async def _run_command(self, command: str, timeout: float = 30.0) -> tuple[int, str, str]:
        proc = await asyncio.create_subprocess_shell(
            command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
        except asyncio.TimeoutError:
            proc.kill()
            return -1, "", f"Timeout after {timeout}s"

        return proc.returncode, stdout.decode(errors="ignore"), stderr.decode(errors="ignore")

    async def _handle_status(self, event: Message, target: Optional[str]) -> None:
        if target and target not in self.services:
            await event.respond(f"❔ Сервис '{target}' не найден. Доступно: {', '.join(self.services.keys()) or 'нет'}")
            return

        names = [target] if target else list(self.services.keys())
        if not names:
            await event.respond("Нет настроенных сервисов. Проверьте devops_bot.yml.")
            return

        await event.respond("⏳ Проверяю...")
        lines: List[str] = []
        async with aiohttp.ClientSession() as session:
            for name in names:
                svc = self.services[name]
                result = await self.check_service(svc, session=session)
                emoji = "✅" if result["ok"] else "❌"
                detail = result.get("detail", "")
                latency = result.get("latency_ms")
                extra = f" ({latency}ms)" if latency is not None else ""
                lines.append(f"{emoji} {name}: {detail}{extra}")

        await event.respond("\n".join(lines))

    async def _handle_logs(self, event: Message, parts: List[str]) -> None:
        if len(parts) < 2:
            await event.respond("Использование: /logs <service> [filter]")
            return

        svc_name = parts[1]
        if svc_name not in self.services:
            await event.respond(f"Сервис '{svc_name}' не найден.")
            return

        svc = self.services[svc_name]
        if not svc.log_command:
            await event.respond("Для сервиса не настроена команда логов.")
            return

        grep_term = parts[2] if len(parts) > 2 else ""
        command = svc.log_command
        if grep_term:
            command = f"{command} | grep -i {shlex.quote(grep_term)}"

        await event.respond(f"🪵 Читаю логи {svc_name}...")
        code, stdout, stderr = await self._run_command(command, timeout=svc.timeout)
        output = stdout.strip() or stderr.strip() or f"empty (exit {code})"
        if len(output) > 3500:
            output = output[-3500:]

        await event.respond(f"```\n{output}\n```")

    async def _handle_restart(self, event: Message, parts: List[str]) -> None:
        if len(parts) < 2:
            await event.respond("Использование: /restart <service>")
            return

        svc_name = parts[1]
        if svc_name not in self.services:
            await event.respond(f"Сервис '{svc_name}' не найден.")
            return

        svc = self.services[svc_name]
        if not svc.restart_command:
            await event.respond("Для сервиса не настроена команда перезапуска.")
            return

        buttons = [
            [
                Button.inline("✅ Подтвердить", f"restart:{svc_name}:yes".encode()),
                Button.inline("❌ Отмена", f"restart:{svc_name}:no".encode()),
            ]
        ]
        await event.respond(
            f"Перезапустить {svc_name}? Команда:\n`{svc.restart_command}`",
            buttons=buttons,
        )

    async def _handle_ask(self, event: Message, question: str) -> None:
        if not question.strip():
            await event.respond("Использование: /ask <вопрос>")
            return

        await event.respond("🤔 Думаю...")
        try:
            answer = await chat_completion(
                messages=[
                    {"role": "system", "content": DEFAULT_SYSTEM_PROMPT},
                    {"role": "user", "content": question},
                ],
                model=self.ai_model,
                temperature=0.2,
                max_tokens=500,
            )
            await event.respond(answer.strip())
        except Exception as exc:
            logger.error("AI error: %s", exc)
            await event.respond(f"❌ Ошибка AI: {exc}")

    async def on_start(self, event: Message) -> None:
        await event.respond(
            "🤖 DevOps AI бот готов.\n"
            "/status [name] — статус сервисов\n"
            "/logs <name> [filter] — последние логи\n"
            "/restart <name> — перезапуск (с подтверждением)\n"
            "/ask <вопрос> — вопрос по DevOps\n"
            "/help — показать команды"
        )

    async def on_message(self, event: Message) -> None:
        if not self._is_allowed(event.sender_id):
            await event.respond("⛔ Доступ запрещен.")
            return

        text = event.raw_text.strip()
        parts = text.split()
        command = parts[0].lower() if parts else ""

        if command in {"/start", "/help"}:
            await self.on_start(event)
        elif command == "/status":
            target = parts[1] if len(parts) > 1 else None
            await self._handle_status(event, target)
        elif command == "/logs":
            await self._handle_logs(event, parts)
        elif command == "/restart":
            await self._handle_restart(event, parts)
        elif command == "/ask":
            question = text[len("/ask") :].strip()
            await self._handle_ask(event, question)

    async def on_callback(self, event) -> None:
        if not self._is_allowed(event.sender_id):
            await event.answer("⛔ Нет доступа", alert=True)
            return

        data = event.data.decode()
        if not data.startswith("restart:"):
            return

        _, svc_name, action = data.split(":")
        svc = self.services.get(svc_name)
        if not svc or not svc.restart_command:
            await event.answer("Сервис недоступен или нет команды.")
            return

        if action == "no":
            await event.edit(f"Перезапуск {svc_name} отменен.")
            return

        await event.edit(f"🔄 Перезапускаю {svc_name}...")
        code, stdout, stderr = await self._run_command(svc.restart_command, timeout=max(30.0, svc.timeout))
        result = stdout.strip() or stderr.strip() or f"exit {code}"

        status_line = ""
        try:
            check = await self.check_service(svc)
            emoji = "✅" if check["ok"] else "❌"
            detail = check.get("detail", "")
            status_line = f"\n{emoji} После перезапуска: {detail}"
        except Exception:
            status_line = ""

        if len(result) > 1500:
            result = result[-1500:]

        await event.edit(f"Результат перезапуска {svc_name}:\n```\n{result}\n```{status_line}")

    async def run(self) -> None:
        await self.client.start(bot_token=BOT_TOKEN)
        logger.info("DevOps AI bot started.")

        if self.monitor_enabled:
            asyncio.create_task(self._monitor_loop())

        self.client.add_event_handler(self.on_message, events.NewMessage)
        self.client.add_event_handler(self.on_callback, events.CallbackQuery)

        await self.client.run_until_disconnected()


async def main() -> None:
    bot = DevOpsAIBot()
    await bot.run()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n👋 Stopped")
