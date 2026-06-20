#!/usr/bin/env python3
"""
Load messages from Telegram chats into a MySQL database.

Usage:
    python load_messages_to_db.py                   # load from all chats (100 messages each)
    python load_messages_to_db.py --limit 500      # load 500 messages from each chat
    python load_messages_to_db.py --chat-id CHAT_ID  # load from a specific chat
    python load_messages_to_db.py --days 7         # only messages from the last 7 days
"""

import asyncio
import json
import os
import sys
import argparse
from datetime import datetime, timedelta
from typing import Optional

import mysql.connector
from mysql.connector import Error as MySQLError

# Add the project path.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from telegram_session import get_client, known_senders, SessionLock
from chat_export_utils import resolve_sender_name, collect_reactions_summary


def get_mysql_connection():
    """Connect to MySQL."""
    return mysql.connector.connect(
        host=os.getenv("MYSQL_HOST", "localhost"),
        port=int(os.getenv("MYSQL_PORT", "3306")),
        database=os.getenv("MYSQL_DATABASE", "pythorust_tg"),
        user=os.getenv("MYSQL_USER"),
        password=os.getenv("MYSQL_PASSWORD"),
        charset="utf8mb4",
        collation="utf8mb4_unicode_ci",
    )


def get_chats_from_db(conn, chat_id: Optional[int] = None) -> list:
    """Get the chat list from the database."""
    cursor = conn.cursor(dictionary=True)
    if chat_id:
        cursor.execute("SELECT id, title FROM telegram_chats WHERE id = %s", (chat_id,))
    else:
        # Take active chats with messages from the last month.
        cursor.execute("""
            SELECT id, title FROM telegram_chats
            WHERE last_message_date > DATE_SUB(NOW(), INTERVAL 30 DAY)
            AND chat_type IN ('supergroup', 'channel', 'group')
            ORDER BY members_count DESC
            LIMIT 50
        """)
    chats = cursor.fetchall()
    cursor.close()
    return chats


def insert_message(cursor, msg_data: dict) -> bool:
    """Insert a message into the database (with ON DUPLICATE KEY UPDATE)."""
    sql = """
        INSERT INTO telegram_messages
        (id, chat_id, sender_id, sender_name, message_text, date,
         reply_to_msg_id, forward_from_id, views, forwards,
         reactions_count, reactions_json, media_type)
        VALUES (%(id)s, %(chat_id)s, %(sender_id)s, %(sender_name)s, %(message_text)s, %(date)s,
                %(reply_to_msg_id)s, %(forward_from_id)s, %(views)s, %(forwards)s,
                %(reactions_count)s, %(reactions_json)s, %(media_type)s)
        ON DUPLICATE KEY UPDATE
            message_text = VALUES(message_text),
            views = VALUES(views),
            forwards = VALUES(forwards),
            reactions_count = VALUES(reactions_count),
            reactions_json = VALUES(reactions_json)
    """
    try:
        cursor.execute(sql, msg_data)
        return True
    except MySQLError as e:
        print(f"  ⚠️ Ошибка вставки сообщения {msg_data['id']}: {e}")
        return False


async def load_messages_from_chat(
    client,
    conn,
    chat_id: int,
    chat_title: str,
    limit: int = 100,
    min_date: Optional[datetime] = None,
    entity=None,
):
    """Load messages from one chat."""
    print(f"\n📥 Загружаю сообщения из: {chat_title} (id={chat_id})")

    known = known_senders.copy()
    cursor = conn.cursor()

    if entity is None:
        try:
            entity = await client.get_entity(chat_id)
        except Exception as e:
            print(f"  ❌ Не удалось получить чат: {e}")
            return 0

    # Get messages.
    messages = await client.get_messages(entity, limit=limit)

    inserted = 0
    skipped = 0

    for m in messages:
        # Skip messages older than min_date.
        if min_date and m.date.replace(tzinfo=None) < min_date:
            skipped += 1
            continue

        # Get sender name.
        sender_name = await resolve_sender_name(m, known, unknown="Unknown")

        # Collect reactions.
        reactions_count, reactions_list = collect_reactions_summary(m)
        reactions_json = json.dumps(reactions_list, ensure_ascii=False) if reactions_list else None

        # Detect media type.
        media_type = None
        if m.photo:
            media_type = "photo"
        elif m.video:
            media_type = "video"
        elif m.document:
            media_type = "document"
        elif m.audio:
            media_type = "audio"
        elif m.voice:
            media_type = "voice"
        elif m.sticker:
            media_type = "sticker"

        # Build data for insertion.
        msg_data = {
            "id": m.id,
            "chat_id": chat_id,
            "sender_id": m.sender_id,
            "sender_name": sender_name[:255] if sender_name else None,
            "message_text": m.text[:65535] if m.text else None,
            "date": m.date.replace(tzinfo=None),
            "reply_to_msg_id": m.reply_to_msg_id if hasattr(m, "reply_to_msg_id") else None,
            "forward_from_id": m.forward.from_id.user_id
            if m.forward and hasattr(m.forward.from_id, "user_id")
            else None,
            "views": m.views,
            "forwards": m.forwards,
            "reactions_count": reactions_count if isinstance(reactions_count, int) else 0,
            "reactions_json": reactions_json,
            "media_type": media_type,
        }

        if insert_message(cursor, msg_data):
            inserted += 1

    conn.commit()
    cursor.close()

    print(f"  ✅ Загружено: {inserted} сообщений (пропущено: {skipped})")
    return inserted


async def main():
    parser = argparse.ArgumentParser(description="Загрузка сообщений из Telegram в MySQL")
    parser.add_argument("--limit", type=int, default=100, help="Количество сообщений на чат")
    parser.add_argument("--chat-id", type=int, help="ID конкретного чата")
    parser.add_argument("--days", type=int, help="Загружать только сообщения за N дней")
    parser.add_argument("--max-chats", type=int, default=50, help="Максимум чатов для загрузки")
    args = parser.parse_args()

    # Date filter.
    min_date = None
    if args.days:
        min_date = datetime.now() - timedelta(days=args.days)
        print(f"📅 Фильтр: сообщения с {min_date.strftime('%Y-%m-%d')}")

    # Connect to MySQL.
    conn = get_mysql_connection()
    print("✅ Подключено к MySQL")

    # Connect to Telegram.
    client = get_client()

    with SessionLock():
        await client.start()
        print("✅ Подключено к Telegram")

        # Get all dialogs; they are the data source.
        print("📋 Загружаю диалоги Telegram...")
        dialogs = await client.get_dialogs()
        print(f"  ✅ Загружено {len(dialogs)} диалогов")

        total_inserted = 0
        processed = 0

        for dialog in dialogs:
            # Skip private chats and bots; take only groups/channels.
            if dialog.is_user:
                continue

            # Filter by a specific chat.
            if args.chat_id and dialog.id != args.chat_id:
                continue

            # Chat limit.
            if processed >= args.max_chats:
                break

            try:
                inserted = await load_messages_from_chat(
                    client,
                    conn,
                    dialog.id,
                    dialog.title or "Unknown",
                    limit=args.limit,
                    min_date=min_date,
                    entity=dialog.entity,
                )
                total_inserted += inserted
                processed += 1
            except Exception as e:
                print(f"  ❌ Ошибка: {e}")
                continue

        await client.disconnect()

    conn.close()

    print(f"\n🎉 Готово! Всего загружено: {total_inserted} сообщений из {processed} чатов")


if __name__ == "__main__":
    from dotenv import load_dotenv

    load_dotenv()

    asyncio.run(main())
