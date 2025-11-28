import os
import sys

from telethon.tl.types import PeerChannel, PeerChat
from telegram_session import get_client, SessionLock, known_senders
from chat_export_utils import (
    DEFAULT_TIMESTAMP_WITH_SECONDS,
    DEFAULT_UNKNOWN_SENDER,
    build_message_text,
    collect_reactions_summary,
    format_timestamp,
    load_chats_from_config,
    resolve_sender_name,
)

# Получаем клиент с SQLite сессией
client = get_client()

# Кэш для хранения sender_id и sender_name
known = known_senders.copy()


async def read_chat(chat_entity, chat_name: str, limit=200):
    assert isinstance(chat_entity, (PeerChannel, PeerChat, str, int)), "PeerChannel, PeerChat, str или int"
    assert isinstance(chat_name, str), "Имя для сохранения"
    assert isinstance(limit, int), "Количество сообщений"

    # Чтение последних сообщений из чата
    messages = await client.get_messages(await client.get_entity(chat_entity), limit=limit)
    # Вывод сообщений
    with open(f'{chat_name}.md', 'w', encoding='utf-8') as md:
        md.write("Напиши сообщение которое соберёт максимум лайков (сердечек). Используй эмоджи:\n")
        # Переворачиваем список сообщений для правильного порядка (сначала старые)
        for m in messages[::-1]:
            sender_name = await resolve_sender_name(m, known, unknown=DEFAULT_UNKNOWN_SENDER)
            timestamp = format_timestamp(m.date, DEFAULT_TIMESTAMP_WITH_SECONDS)
            text = build_message_text(m)
            reactions, emojis = collect_reactions_summary(m)
            # Обработка медиа
            if m.media:
                if reactions >= 1000:
                    os.makedirs(chat_name, exist_ok=True)
                    print(f"{timestamp} {sender_name}: {text} {emojis} --->\n")
                    file_path = await m.download_media(file=(chat_name))
                    print(f"{timestamp} {sender_name}: {text} {emojis} {file_path}\n")
                    md.write(f"{timestamp} {sender_name}: {text} {emojis} {file_path}\n")
                else:
                    md.write(f"{timestamp} {sender_name}: {text} {emojis}\n")
            else:
                md.write(f"{sender_name}: {text} {emojis}\n")


CONFIG = load_chats_from_config()

print(sys.argv)
chat = sys.argv[1] if len(sys.argv) >= 2 else list(CONFIG.keys())[0] if CONFIG else None
if not chat or chat not in CONFIG:
    print(f"Использование: python tg.py <chat_name>")
    print(f"Доступные чаты: {', '.join(CONFIG.keys())}")
    sys.exit(1)

with SessionLock():  # Защита от параллельного запуска
    with client:
        client.loop.run_until_complete(read_chat(CONFIG[chat], chat))
