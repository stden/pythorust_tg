import os
import sys

from dotenv import load_dotenv

from telethon.tl.types import PeerChannel, PeerChat
from telegram_session import get_client, known_senders, SessionLock
from chat_export_utils import (
    DEFAULT_TIMESTAMP_WITH_SECONDS,
    DEFAULT_UNKNOWN_SENDER,
    build_message_text,
    collect_reactions_summary,
    format_timestamp,
    load_chats_from_config,
    resolve_sender_name,
)

load_dotenv()

# Получаем клиент с SQLite сессией
client = get_client()

# Кэш для хранения sender_id и sender_name
known = known_senders.copy()


def _parse_int_env(name: str):
    raw = os.getenv(name)
    if raw is None or raw.strip() == "":
        return None
    try:
        return int(raw)
    except ValueError as exc:
        raise ValueError(f"{name} must be an integer") from exc


def _parse_int_list_env(name: str):
    raw = os.getenv(name)
    if raw is None or raw.strip() == "":
        return set()
    values = set()
    for chunk in raw.split(","):
        chunk = chunk.strip()
        if not chunk:
            continue
        try:
            values.add(int(chunk))
        except ValueError as exc:
            raise ValueError(f"{name} must contain comma-separated integers") from exc
    return values


MY_ID = _parse_int_env("MY_ID") or _parse_int_env("USER_ID") or _parse_int_env("MY_USER_ID")
MY_NAME = (os.getenv("MY_NAME") or os.getenv("USER_NAME") or "").strip()
ZOOM_SENDER_ID = _parse_int_env("ZOOM_SENDER_ID") or MY_ID
ZOOM_URL_PATTERN = os.getenv("ZOOM_URL_PATTERN", "zoom.us")
LOW_ENGAGEMENT_SENDER_IDS = _parse_int_list_env("LOW_ENGAGEMENT_SENDER_IDS")
if MY_ID:
    LOW_ENGAGEMENT_SENDER_IDS.add(MY_ID)
if MY_ID and MY_NAME:
    known[MY_ID] = MY_NAME


def is_github_actions():
    return os.getenv("GITHUB_ACTIONS") == "true"


LIMIT = 3000
if is_github_actions():
    LIMIT = 1000
print(f"LIMIT={LIMIT}")

MEDIA_REACTION_LIMIT = int(os.getenv("MEDIA_REACTION_LIMIT", "100000"))


async def read_chat(chat_entity, chat_name: str, limit=LIMIT):
    assert isinstance(chat_entity, (PeerChannel, PeerChat, str, int)), "PeerChannel, PeerChat, str или int"
    assert isinstance(chat_name, str), "Имя для сохранения"
    assert isinstance(limit, int), "Количество сообщений"

    # Чтение последних сообщений из чата
    messages = await client.get_messages(await client.get_entity(chat_entity), limit=limit)
    replied = set()
    for m in messages:
        if m.reply_to_msg_id:  # Это сообщение - ответ на другое сообщение
            # Добавляем id сообщения, на которое ответили, в множество
            replied.add(m.reply_to_msg_id)
    # Вывод сообщений
    with open(f'{chat_name}.md', 'w', encoding='utf-8') as md:
        md.write(
            f"""Что интересно людям в чате. Напиши отчёт с юмором и эмодзи. Вот чат:\n""")
        # Переворачиваем список сообщений для правильного порядка (сначала старые)
        for m in messages[::-1]:
            sender_name = await resolve_sender_name(m, known, unknown=DEFAULT_UNKNOWN_SENDER)
            timestamp = format_timestamp(m.date, DEFAULT_TIMESTAMP_WITH_SECONDS)
            raw_text = m.text or ""
            text = build_message_text(m)
            reactions, emojis = collect_reactions_summary(m)
            if ZOOM_SENDER_ID and m.sender_id == ZOOM_SENDER_ID and raw_text and ZOOM_URL_PATTERN in raw_text:
                print(f"!!!DEL-ZOOM!!! {timestamp} {sender_name}: {raw_text} {reactions}")
                await client.delete_messages(m.chat_id, m.id)
            if LOW_ENGAGEMENT_SENDER_IDS and m.sender_id in LOW_ENGAGEMENT_SENDER_IDS and reactions == 0:
                if m.id not in replied: # Если это не ответ на сообщение
                    print(f"! Неинтересное сообщение, удаляю: {timestamp} {sender_name}: {text}")
                    await client.delete_messages(m.chat_id, m.id)
            if m.media:
                if reactions >= MEDIA_REACTION_LIMIT and not is_github_actions():
                    os.makedirs(chat_name, exist_ok=True)
                    file_path = await m.download_media(file=(chat_name))
                    md.write(f"{timestamp} {sender_name}: {text} {emojis} {file_path}\n")
                else:
                    pass # Пропускаем медиа
                    # md.write(f"{timestamp} {sender_name}: {m.text} {emojis} [Media]\n")
            else:
                md.write(f"{sender_name}: {text} {emojis}\n")


CONFIG = load_chats_from_config()
if not CONFIG:
    print("config.yml не найден или пустой, CONFIG пустой")

print(sys.argv)
default_chat = os.getenv("DEFAULT_CHAT")
chat = sys.argv[1] if len(sys.argv) == 2 else default_chat

if not chat:
    raise ValueError("Не указан чат: передайте имя аргументом или задайте DEFAULT_CHAT в .env")
if chat not in CONFIG:
    raise KeyError(f"Чат '{chat}' не найден в CONFIG (config.yml).")

with SessionLock():  # Защита от параллельного запуска
    with client:
        client.loop.run_until_complete(read_chat(CONFIG[chat], chat))
