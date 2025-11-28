#!/usr/bin/env python3
"""Read vibecod3rs chat."""

from chat_export_utils import build_message_text, format_timestamp, resolve_sender_name
from telegram_session import SessionLock, get_client, known_senders

CHAT_URL = "https://t.me/vibecod3rs"
DEFAULT_LIMIT = 150

client = get_client()
known = known_senders.copy()


async def read_vibecod3rs(limit: int = DEFAULT_LIMIT):
    """Print last messages from vibecod3rs chat."""
    async with client:
        if not await client.is_user_authorized():
            print("Not authorized")
            return

        entity = await client.get_entity(CHAT_URL)
        messages = await client.get_messages(entity, limit=limit)
        chat_title = getattr(entity, "title", CHAT_URL)

        print(f"=== Последние {limit} сообщений из {chat_title} ===\n")

        for message in reversed(messages):
            text = build_message_text(message)
            if not text.strip():
                continue

            sender_name = await resolve_sender_name(message, known)
            date = format_timestamp(getattr(message, "date", None), "%d.%m.%Y %H:%M")

            print(f"[{date}] {sender_name}:")
            print(text)
            print("-" * 80)
            print()


def main():
    with SessionLock():
        client.loop.run_until_complete(read_vibecod3rs())


if __name__ == "__main__":
    main()
