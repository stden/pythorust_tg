#!/usr/bin/env python3
"""
Export Telegram chats list to MySQL database.
"""

import asyncio
import os
import pymysql
from telethon import TelegramClient
from telethon.tl.types import User, Chat, Channel
from dotenv import load_dotenv

load_dotenv()

# Telegram credentials
API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
SESSION_FILE = "telegram_session"

# MySQL credentials
MYSQL_CONFIG = {
    "host": os.getenv("MYSQL_HOST", "localhost"),
    "port": int(os.getenv("MYSQL_PORT", 3306)),
    "database": os.getenv("MYSQL_DATABASE", "pythorust_tg"),
    "user": os.getenv("MYSQL_USER", "pythorust_tg"),
    "password": os.getenv("MYSQL_PASSWORD"),
    "charset": "utf8mb4",
    "cursorclass": pymysql.cursors.DictCursor,
}


def get_chat_type(entity) -> str:
    """Determine chat type from entity."""
    if isinstance(entity, User):
        if entity.bot:
            return "bot"
        return "user"
    elif isinstance(entity, Chat):
        return "group"
    elif isinstance(entity, Channel):
        if entity.megagroup:
            return "supergroup"
        return "channel"
    return "user"


def get_mysql_connection():
    """Create MySQL connection."""
    return pymysql.connect(**MYSQL_CONFIG)


async def export_chats():
    """Export all chats to MySQL."""
    os.getenv("TELEGRAM_PHONE")
    client = TelegramClient(SESSION_FILE, API_ID, API_HASH)

    # Connect without starting auth flow - use existing session
    await client.connect()

    if not await client.is_user_authorized():
        print("ERROR: Not authorized. Please run `cargo run -- init-session` first.")
        return

    print("Connected to Telegram")

    # Get all dialogs
    dialogs = await client.get_dialogs()
    print(f"Found {len(dialogs)} dialogs")

    # Connect to MySQL
    conn = get_mysql_connection()
    cursor = conn.cursor()

    # Prepare insert query
    insert_query = """
        INSERT INTO telegram_chats
        (id, title, username, chat_type, members_count, unread_count,
         last_message_date, is_verified, is_restricted, is_creator, is_admin)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON DUPLICATE KEY UPDATE
        title = VALUES(title),
        username = VALUES(username),
        members_count = VALUES(members_count),
        unread_count = VALUES(unread_count),
        last_message_date = VALUES(last_message_date),
        is_verified = VALUES(is_verified),
        is_restricted = VALUES(is_restricted),
        is_creator = VALUES(is_creator),
        is_admin = VALUES(is_admin),
        updated_at = CURRENT_TIMESTAMP
    """

    inserted = 0
    updated = 0
    errors = 0

    for dialog in dialogs:
        try:
            entity = dialog.entity
            chat_id = dialog.id
            title = dialog.title or dialog.name or "Unknown"
            username = getattr(entity, "username", None)
            chat_type = get_chat_type(entity)

            # Get members count
            members_count = None
            if hasattr(entity, "participants_count"):
                members_count = entity.participants_count

            # Get unread count
            unread_count = dialog.unread_count

            # Get last message date
            last_message_date = None
            if dialog.message:
                last_message_date = dialog.message.date

            # Get flags
            is_verified = getattr(entity, "verified", False)
            is_restricted = getattr(entity, "restricted", False)
            is_creator = getattr(entity, "creator", False)
            is_admin = getattr(entity, "admin_rights", None) is not None

            # Insert or update
            cursor.execute(
                insert_query,
                (
                    chat_id,
                    title,
                    username,
                    chat_type,
                    members_count,
                    unread_count,
                    last_message_date,
                    is_verified,
                    is_restricted,
                    is_creator,
                    is_admin,
                ),
            )

            if cursor.rowcount == 1:
                inserted += 1
            else:
                updated += 1

            print(f"  [{chat_type:10}] {title[:40]:<40} (ID: {chat_id})")

        except Exception as e:
            errors += 1
            print(f"  ERROR: {dialog.title or dialog.name}: {e}")

    conn.commit()
    cursor.close()
    conn.close()

    await client.disconnect()

    print(f"\n{'=' * 50}")
    print("Export complete!")
    print(f"  Inserted: {inserted}")
    print(f"  Updated:  {updated}")
    print(f"  Errors:   {errors}")
    print(f"  Total:    {len(dialogs)}")


if __name__ == "__main__":
    asyncio.run(export_chats())
