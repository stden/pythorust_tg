#!/usr/bin/env python3
"""Delete all my outgoing messages from a chat using Telethon"""

import asyncio
import sys
from telethon import TelegramClient
import os
from dotenv import load_dotenv

load_dotenv()

API_ID = int(os.getenv("TELEGRAM_API_ID", "0"))
API_HASH = os.getenv("TELEGRAM_API_HASH", "")
SESSION_FILE = "+79117117850"


async def delete_my_messages(chat_id: int, limit: int = 0, dry_run: bool = False):
    """Delete all outgoing messages from a chat"""
    client = TelegramClient(SESSION_FILE, API_ID, API_HASH)
    await client.start()

    try:
        # Get the chat entity
        entity = await client.get_entity(chat_id)
        chat_name = getattr(entity, "title", str(chat_id))
        print(f"Found chat: {chat_name}")

        # Collect messages to delete
        messages_to_delete = []
        count = 0

        print(f"Scanning messages in {chat_name}...")

        async for message in client.iter_messages(entity, limit=limit if limit > 0 else None):
            count += 1
            if message.out:  # Outgoing message
                messages_to_delete.append(message)

        print(f"Found {len(messages_to_delete)} outgoing messages out of {count} total")

        # Delete messages
        deleted = 0
        for msg in messages_to_delete:
            text_preview = (msg.text or "[media]")[:50].replace("\n", " ")
            date_str = msg.date.strftime("%d.%m.%Y %H:%M")

            if dry_run:
                print(f"  WOULD DELETE: {date_str} - {text_preview}")
            else:
                print(f"  DEL: {date_str} - {text_preview}")
                try:
                    await msg.delete()
                    deleted += 1
                except Exception as e:
                    print(f"    Error: {e}")

        print(f"\n=== Deleted {deleted} messages ===")

    finally:
        await client.disconnect()


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python delete_my_msgs.py <chat_id> [limit] [--dry-run]")
        sys.exit(1)

    chat_id = int(sys.argv[1])
    limit = int(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2] != "--dry-run" else 0
    dry_run = "--dry-run" in sys.argv

    asyncio.run(delete_my_messages(chat_id, limit, dry_run))
