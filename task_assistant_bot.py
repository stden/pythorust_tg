#!/usr/bin/env python3
"""
Task Assistant Telegram Bot
Телеграм-бот помощник для автоматизации задач Даши
"""

import asyncio
import logging
import os
import sys
from datetime import datetime
from pathlib import Path

from dotenv import load_dotenv
from telethon import Button, TelegramClient, events

# Load environment variables
load_dotenv()

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent))

from integrations.openai_client import chat_completion

# Configuration from .env
API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
BOT_TOKEN = os.getenv("TASK_ASSISTANT_BOT_TOKEN")
ALLOWED_USERS = [int(x) for x in os.getenv("ALLOWED_USERS", "").split(",") if x] if os.getenv("ALLOWED_USERS") else []
OPENAI_MODEL = os.getenv("OPENAI_MODEL")

logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s")
logger = logging.getLogger(__name__)


class TaskAssistantBot:
    """Task assistant bot for automation."""

    def __init__(self):
        self.client = TelegramClient("task_assistant_bot", API_ID, API_HASH)
        self.pending_commands = {}

    async def check_access(self, user_id: int) -> bool:
        """Check if user has access."""
        if not ALLOWED_USERS:
            return True  # No restrictions
        return user_id in ALLOWED_USERS

    async def start_handler(self, event):
        """Handle /start command."""
        if not await self.check_access(event.sender_id):
            await event.respond("❌ Доступ запрещён")
            return

        buttons = [
            [Button.inline("🔍 Проверить N8N", b"check_n8n")],
            [Button.inline("🔄 Перезапустить N8N", b"restart_n8n")],
            [Button.inline("💾 Создать бэкап", b"create_backup")],
            [Button.inline("📋 Список бэкапов", b"list_backups")],
            [Button.inline("🤖 ИИ-консультант", b"ai_consultant")],
            [Button.inline("📊 Статус серверов", b"server_status")],
        ]

        await event.respond(
            "👋 **Привет! Я твой помощник по автоматизации.**\n\n"
            "Могу помочь с:\n"
            "• Мониторинг и управление N8N\n"
            "• Бэкапы конфигураций\n"
            "• ИИ-консультации по проектам\n"
            "• Проверка статуса серверов\n\n"
            "Выбери действие:",
            buttons=buttons,
        )

    async def check_n8n_health(self) -> dict:
        """Check N8N health."""
        try:
            import aiohttp

            async with aiohttp.ClientSession() as session:
                async with session.get(
                    "https://n8n.vier-pfoten.club/healthz", timeout=aiohttp.ClientTimeout(total=10), ssl=False
                ) as response:
                    return {"status": "✅ Работает" if response.status == 200 else "❌ Ошибка", "code": response.status}
        except Exception as e:
            return {"status": "❌ Недоступен", "error": str(e)}

    async def restart_n8n_service(self) -> dict:
        """Restart N8N service."""
        try:
            process = await asyncio.create_subprocess_shell(
                "systemctl restart n8n", stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await process.communicate()

            if process.returncode == 0:
                # Wait and check
                await asyncio.sleep(5)
                health = await self.check_n8n_health()
                return {"success": True, "health": health}
            else:
                return {"success": False, "error": stderr.decode()}
        except Exception as e:
            return {"success": False, "error": str(e)}

    async def create_n8n_backup(self) -> dict:
        """Create N8N backup."""
        try:
            # Get project root from environment or use current directory
            project_root = os.getenv("PROJECT_ROOT", os.getcwd())
            process = await asyncio.create_subprocess_shell(
                f"cd {project_root} && .venv/bin/python n8n_backup.py backup",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await process.communicate()

            if process.returncode == 0:
                return {"success": True, "output": stdout.decode()}
            else:
                return {"success": False, "error": stderr.decode()}
        except Exception as e:
            return {"success": False, "error": str(e)}

    async def list_n8n_backups(self) -> list:
        """List N8N backups."""
        try:
            backup_dir = Path("/srv/backups/n8n")
            if not backup_dir.exists():
                return []

            backups = sorted(backup_dir.glob("n8n_backup_*.tar.gz"), key=lambda p: p.stat().st_mtime, reverse=True)

            result = []
            for backup in backups[:10]:  # Last 10
                stat = backup.stat()
                size_mb = stat.st_size / (1024 * 1024)
                mtime = datetime.fromtimestamp(stat.st_mtime)
                result.append({"name": backup.name, "size_mb": size_mb, "date": mtime.strftime("%d.%m.%Y %H:%M")})

            return result
        except Exception as e:
            logger.error(f"Error listing backups: {e}")
            return []

    async def get_server_status(self) -> dict:
        """Get server status."""
        try:
            # CPU usage
            cpu_cmd = "top -bn1 | grep 'Cpu(s)' | sed 's/.*, *\\([0-9.]*\\)%* id.*/\\1/' | awk '{print 100 - $1}'"
            cpu_process = await asyncio.create_subprocess_shell(cpu_cmd, stdout=asyncio.subprocess.PIPE)
            cpu_out, _ = await cpu_process.communicate()
            cpu_usage = float(cpu_out.decode().strip())

            # Memory usage
            mem_cmd = "free | grep Mem | awk '{print ($3/$2) * 100.0}'"
            mem_process = await asyncio.create_subprocess_shell(mem_cmd, stdout=asyncio.subprocess.PIPE)
            mem_out, _ = await mem_process.communicate()
            mem_usage = float(mem_out.decode().strip())

            # Disk usage
            disk_cmd = "df -h / | tail -1 | awk '{print $5}' | sed 's/%//'"
            disk_process = await asyncio.create_subprocess_shell(disk_cmd, stdout=asyncio.subprocess.PIPE)
            disk_out, _ = await disk_process.communicate()
            disk_usage = float(disk_out.decode().strip())

            return {"cpu": cpu_usage, "memory": mem_usage, "disk": disk_usage}
        except Exception as e:
            logger.error(f"Error getting server status: {e}")
            return {"error": str(e)}

    async def callback_handler(self, event):
        """Handle callback queries."""
        data = event.data.decode()

        if data == "check_n8n":
            await event.answer("Проверяю N8N...")
            health = await self.check_n8n_health()
            await event.respond(
                f"**N8N Health Check**\n\n"
                f"Статус: {health['status']}\n"
                f"HTTP Code: {health.get('code', 'N/A')}\n"
                f"Ошибка: {health.get('error', 'Нет')}"
            )

        elif data == "restart_n8n":
            await event.answer("Перезапускаю N8N...")
            result = await self.restart_n8n_service()
            if result["success"]:
                await event.respond(f"✅ **N8N перезапущен**\n\nСтатус: {result['health']['status']}")
            else:
                await event.respond(f"❌ **Ошибка перезапуска**\n\nОшибка: {result['error']}")

        elif data == "create_backup":
            await event.answer("Создаю бэкап...")
            result = await self.create_n8n_backup()
            if result["success"]:
                await event.respond("✅ Бэкап создан успешно")
            else:
                await event.respond(f"❌ Ошибка: {result['error']}")

        elif data == "list_backups":
            await event.answer("Получаю список бэкапов...")
            backups = await self.list_n8n_backups()
            if backups:
                text = "📋 **Последние бэкапы N8N:**\n\n"
                for backup in backups:
                    text += f"• {backup['name']}\n"
                    text += f"  {backup['date']} ({backup['size_mb']:.1f} MB)\n\n"
                await event.respond(text)
            else:
                await event.respond("📋 Бэкапы не найдены")

        elif data == "ai_consultant":
            await event.respond(
                "🤖 **ИИ-консультант**\n\n"
                "Просто напиши свой вопрос, и я помогу!\n\n"
                "Примеры:\n"
                "• Как настроить Caddy для N8N?\n"
                "• Почему приложение недоступно извне?\n"
                "• Напиши скрипт для мониторинга"
            )

        elif data == "server_status":
            await event.answer("Получаю статус сервера...")
            status = await self.get_server_status()
            if "error" not in status:
                cpu_emoji = "🟢" if status["cpu"] < 70 else "🟡" if status["cpu"] < 90 else "🔴"
                mem_emoji = "🟢" if status["memory"] < 70 else "🟡" if status["memory"] < 90 else "🔴"
                disk_emoji = "🟢" if status["disk"] < 70 else "🟡" if status["disk"] < 90 else "🔴"

                await event.respond(
                    f"📊 **Статус сервера**\n\n"
                    f"{cpu_emoji} CPU: {status['cpu']:.1f}%\n"
                    f"{mem_emoji} RAM: {status['memory']:.1f}%\n"
                    f"{disk_emoji} Disk: {status['disk']:.1f}%"
                )
            else:
                await event.respond(f"❌ Ошибка: {status['error']}")

    async def message_handler(self, event):
        """Handle regular messages."""
        if event.message.text.startswith("/"):
            return  # Skip commands

        if not await self.check_access(event.sender_id):
            return

        # AI consultant mode
        message_text = event.message.text

        await event.respond("🤔 Думаю...")

        try:
            response = await chat_completion(
                messages=[
                    {
                        "role": "system",
                        "content": "Ты - технический помощник. Помогаешь с вопросами по N8N, серверам, автоматизации. Отвечай кратко и по делу, с примерами если нужно.",
                    },
                    {"role": "user", "content": message_text},
                ],
                model=OPENAI_MODEL,
                temperature=0.3,
            )

            await event.respond(response)

        except Exception as e:
            await event.respond(f"❌ Ошибка: {e}")

    async def run(self):
        """Start the bot."""
        await self.client.start(bot_token=BOT_TOKEN)

        logger.info("✅ Task Assistant Bot started")

        # Register handlers
        self.client.add_event_handler(self.start_handler, events.NewMessage(pattern="/start"))
        self.client.add_event_handler(self.callback_handler, events.CallbackQuery)
        self.client.add_event_handler(self.message_handler, events.NewMessage)

        logger.info("📱 Bot is running...")
        await self.client.run_until_disconnected()


async def main():
    """Entry point."""
    bot = TaskAssistantBot()
    await bot.run()


if __name__ == "__main__":
    asyncio.run(main())
