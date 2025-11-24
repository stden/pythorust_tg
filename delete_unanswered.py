#!/usr/bin/env python3
"""Delete my unanswered messages without reactions from all chats."""
import os

from dotenv import load_dotenv
from telegram_session import get_client, SessionLock

load_dotenv()


def _get_my_id() -> int:
    raw = os.getenv("MY_ID") or os.getenv("USER_ID")
    if raw is None or raw.strip() == "":
        raise ValueError("MY_ID или USER_ID должно быть задано в .env")
    try:
        return int(raw)
    except ValueError as exc:
        raise ValueError("MY_ID/USER_ID must be an integer") from exc


MY_ID = _get_my_id()

client = get_client()


async def delete_unanswered_in_chat(dialog):
    """Delete my unanswered messages without reactions in a specific chat."""
    chat_name = dialog.title or dialog.name or str(dialog.id)

    # Get messages
    messages = await client.get_messages(dialog.entity, limit=500)

    # Find which messages have replies
    replied_to = set()
    for m in messages:
        if m.reply_to_msg_id:
            replied_to.add(m.reply_to_msg_id)

    deleted_count = 0
    for m in messages:
        # Skip if not my message
        if not m.out and m.sender_id != MY_ID:
            continue

        # Skip if message has reactions
        if hasattr(m, 'reactions') and m.reactions:
            reactions = sum(r.count for r in m.reactions.results)
            if reactions > 0:
                continue

        # Skip if someone replied to this message
        if m.id in replied_to:
            continue

        # Delete the message
        timestamp = m.date.strftime('%d.%m.%Y %H:%M')
        text_preview = (m.text or '[media]')[:50].replace('\n', ' ')
        print(f"  DEL: {timestamp} - {text_preview}...")

        try:
            await client.delete_messages(dialog.entity, m.id, revoke=True)
            deleted_count += 1
        except Exception as e:
            print(f"    Error: {e}")

    return deleted_count


async def main():
    """Delete unanswered messages without reactions from all dialogs."""
    dialogs = await client.get_dialogs(limit=100)

    total_deleted = 0
    for dialog in dialogs:
        chat_name = dialog.title or dialog.name or str(dialog.id)

        try:
            deleted = await delete_unanswered_in_chat(dialog)
            if deleted > 0:
                print(f"\n{chat_name}: deleted {deleted} messages")
                total_deleted += deleted
        except Exception as e:
            print(f"Error in {chat_name}: {e}")

    print(f"\n=== Total deleted: {total_deleted} messages ===")


if __name__ == '__main__':
    with SessionLock():
        with client:
            client.loop.run_until_complete(main())
