#!/usr/bin/env python3
"""
Простой экспорт чата с пользователем по username
"""

import asyncio
import os
import sys
from datetime import datetime
from telethon import TelegramClient
from dotenv import load_dotenv

load_dotenv()

API_ID = int(os.getenv("TELEGRAM_API_ID", "0"))
API_HASH = os.getenv("TELEGRAM_API_HASH", "")
SESSION_FILE = os.getenv("TELEGRAM_SESSION_NAME", "telegram_session") + ".session"


async def export_user_chat(username: str, limit: int = 1000):
    """Export chat with user by username."""
    async with TelegramClient(SESSION_FILE, API_ID, API_HASH) as client:
        # Remove @ if present
        username = username.lstrip("@")

        print(f"Connecting to @{username}...")

        try:
            entity = await client.get_entity(username)
            display_name = f"{getattr(entity, 'first_name', '')} {getattr(entity, 'last_name', '')}".strip()
            if not display_name:
                display_name = username

            print(f"✅ Found: {display_name} (@{username})")
        except Exception as e:
            print(f"❌ Error: {e}")
            return None

        print(f"Экспортирую последние {limit} сообщений...")

        messages = []
        async for message in client.iter_messages(entity, limit=limit):
            if message.message:
                sender_name = "Unknown"
                if message.sender:
                    if hasattr(message.sender, "first_name"):
                        sender_name = f"{message.sender.first_name or ''} {message.sender.last_name or ''}".strip()
                    elif hasattr(message.sender, "title"):
                        sender_name = message.sender.title

                date_str = message.date.strftime("%d.%m.%Y %H:%M:%S")
                messages.append(f"[{date_str}] {sender_name}: {message.message}")

        # Save to file
        output_file = f"chats/{username}.txt"
        os.makedirs("chats", exist_ok=True)

        with open(output_file, "w", encoding="utf-8") as f:
            f.write(f"# Chat with @{username} ({display_name})\n")
            f.write(f"# Exported: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
            f.write(f"# Messages: {len(messages)}\n\n")
            f.write("\n".join(reversed(messages)))

        print(f"\n✅ Exported {len(messages)} messages → {output_file}")
        return output_file


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python export_user_simple.py <username> [limit]")
        sys.exit(1)

    username = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 1000

    asyncio.run(export_user_chat(username, limit))
