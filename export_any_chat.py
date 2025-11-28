#!/usr/bin/env python3
"""
Экспорт произвольного чата Telegram по имени.
"""

import asyncio
import os
import sys
from datetime import datetime
from telethon import TelegramClient
from dotenv import load_dotenv

load_dotenv()

API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
SESSION_FILE = os.getenv("TELEGRAM_SESSION_FILE", "telegram_session")


async def export_chat(chat_name: str, limit: int = 500):
    """Export chat by name."""
    async with TelegramClient(SESSION_FILE, API_ID, API_HASH) as client:
        # Get all dialogs
        dialogs = await client.get_dialogs()

        # Find chat by name
        target_dialog = None
        for dialog in dialogs:
            if dialog.title and chat_name.lower() in dialog.title.lower():
                target_dialog = dialog
                break

        if not target_dialog:
            print(f"❌ Чат '{chat_name}' не найден")
            return

        print(f"✅ Найден чат: {target_dialog.title}")
        print(f"Экспортирую последние {limit} сообщений...")

        messages = []
        async for message in client.iter_messages(target_dialog.entity, limit=limit):
            if message.message:
                sender_name = "Unknown"
                if message.sender:
                    if hasattr(message.sender, 'first_name'):
                        sender_name = f"{message.sender.first_name or ''} {message.sender.last_name or ''}".strip()
                    elif hasattr(message.sender, 'title'):
                        sender_name = message.sender.title

                date_str = message.date.strftime('%d.%m.%Y %H:%M:%S')
                messages.append(f"[{date_str}] {sender_name}: {message.message}")

        # Save to file
        output_file = f"chats/{chat_name.replace(' ', '_')}.txt"
        os.makedirs("chats", exist_ok=True)

        with open(output_file, 'w', encoding='utf-8') as f:
            f.write(f"# Чат: {target_dialog.title}\n")
            f.write(f"# Экспортировано: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
            f.write(f"# Сообщений: {len(messages)}\n\n")
            f.write('\n'.join(reversed(messages)))

        print(f"\n✅ Экспортировано {len(messages)} сообщений → {output_file}")
        return output_file


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Использование: python export_any_chat.py <chat_name> [limit]")
        sys.exit(1)

    chat_name = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 500

    asyncio.run(export_chat(chat_name, limit))
