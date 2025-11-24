import os
import re
import sys
from typing import Optional, Set

from dotenv import load_dotenv
from telethon import events

from linear_client import LinearClient, LinearError
from telegram_session import SessionLock, get_client

load_dotenv()

COMMAND_PREFIX = (os.getenv("LINEAR_COMMAND_PREFIX") or "!linear").strip() or "!linear"
TEAM_KEY = os.getenv("LINEAR_TEAM_KEY")
PROJECT_ID = os.getenv("LINEAR_PROJECT_ID")


def _parse_default_priority() -> int:
    raw = os.getenv("LINEAR_DEFAULT_PRIORITY", "1")
    try:
        return int(raw)
    except ValueError:
        return 1


DEFAULT_PRIORITY = _parse_default_priority()


def _parse_allowed_senders() -> Set[int]:
    raw = os.getenv("LINEAR_ALLOWED_SENDERS", "")
    allowed: Set[int] = set()
    for chunk in raw.split(","):
        chunk = chunk.strip()
        if not chunk:
            continue
        try:
            allowed.add(int(chunk))
        except ValueError:
            print(f"Пропускаю некорректный LINEAR_ALLOWED_SENDERS id: {chunk}")
    return allowed


ALLOWED_SENDER_IDS = _parse_allowed_senders()

try:
    linear = LinearClient()
except LinearError as exc:
    print(f"Не удалось инициализировать Linear клиент: {exc}")
    sys.exit(1)

client = get_client()
LINEAR_COMMAND_PATTERN = rf"^{re.escape(COMMAND_PREFIX)}\s+(.+)"


def _split_payload(payload: str) -> tuple[str, Optional[str]]:
    if "|" in payload:
        title, description = payload.split("|", 1)
        return title.strip(), description.strip() or None
    return payload.strip(), None


def _merge_description(description: Optional[str], reply_text: Optional[str]) -> Optional[str]:
    reply_text = reply_text.strip() if reply_text else ""
    description = description.strip() if description else ""

    if description and reply_text:
        return f"{description}\n\nИсходное сообщение из Telegram:\n{reply_text}"
    if reply_text:
        return f"Исходное сообщение из Telegram:\n{reply_text}"
    return description or None


@client.on(events.NewMessage(pattern=LINEAR_COMMAND_PATTERN, incoming=True))
async def handle_linear_command(event):
    sender_id = event.sender_id
    if ALLOWED_SENDER_IDS and sender_id not in ALLOWED_SENDER_IDS:
        return

    if not TEAM_KEY:
        await event.reply("LINEAR_TEAM_KEY не задан (добавьте в .env).")
        return

    payload = event.pattern_match.group(1).strip()
    if not payload:
        await event.reply(f"Использование: {COMMAND_PREFIX} <заголовок> | <описание>")
        return

    title, description = _split_payload(payload)
    if not title:
        await event.reply("Нужно указать заголовок задачи после команды.")
        return

    reply_text = None
    if event.is_reply:
        reply = await event.get_reply_message()
        reply_text = reply.raw_text if reply else None

    full_description = _merge_description(description, reply_text)

    try:
        issue = linear.create_issue(
            team_key=TEAM_KEY,
            title=title,
            description=full_description,
            priority=DEFAULT_PRIORITY,
            project_id=PROJECT_ID,
        )
    except LinearError as exc:
        await event.reply(f"Linear вернул ошибку: {exc}")
        return
    except Exception as exc:  # noqa: BLE001
        await event.reply("Не удалось создать задачу, см. логи.")
        print(f"Не удалось создать задачу в Linear: {exc}")
        return

    identifier = issue.get("identifier")
    url = issue.get("url")
    await event.reply(f"Создана задача {identifier or ''} {url or ''}".strip())


async def main():
    allowed = ", ".join(str(v) for v in sorted(ALLOWED_SENDER_IDS)) or "все"
    print(
        f"Linear bot запущен. Команда {COMMAND_PREFIX}, команда: {TEAM_KEY or 'не задана'}, "
        f"разрешённые отправители: {allowed}"
    )
    await client.run_until_disconnected()


if __name__ == "__main__":
    with SessionLock():
        with client:
            client.loop.run_until_complete(main())
