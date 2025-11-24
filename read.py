import os
import sys

from dotenv import load_dotenv

from telethon.tl.types import PeerChannel, PeerChat, ReactionEmoji
from telegram_session import get_client, known_senders, SessionLock

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
    assert isinstance(chat_entity, (PeerChannel, PeerChat, str)), "PeerChannel или PeerChat"
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
            # Добавляем проверку, что сообщение от вас
            if m.sender_id in known:
                sender_name = known[m.sender_id]
            else:
                sender = await m.get_sender()  # Получение объекта отправителя
                if sender:  # Имя отправителя
                    try:
                        sender_name = f"{sender.first_name or ''} {sender.last_name or ''}".strip()
                    except:
                        sender_name = f"@{sender.username}"
                else:
                    sender_name = "Неизвестный отправитель"
                known[m.sender_id] = sender_name  # Сохранение имени
            timestamp = m.date.strftime('%d.%m.%Y %H:%M:%S')
            # Реакции
            if hasattr(m, 'reactions') and m.reactions:
                reactions = sum(r.count for r in m.reactions.results)
                emojis_list = []
                for r in m.reactions.results:
                    # Проверяем, является ли r.reaction объектом ReactionEmoji
                    if isinstance(r.reaction, ReactionEmoji):
                        emojis_list.append(r.reaction.emoticon)  # Достаём собственно эмодзи
                    else:
                        emojis_list.append(str(r.reaction))  # Иначе просто приводим к строке
                emojis = ''.join(emojis_list)
            else:
                reactions = 0
                emojis = ''
            if ZOOM_SENDER_ID and m.sender_id == ZOOM_SENDER_ID and m.text and ZOOM_URL_PATTERN in m.text:
                print(f"!!!DEL-ZOOM!!! {timestamp} {sender_name}: {m.text} {reactions}")
                await client.delete_messages(m.chat_id, m.id)
            if LOW_ENGAGEMENT_SENDER_IDS and m.sender_id in LOW_ENGAGEMENT_SENDER_IDS and reactions == 0:
                if m.id not in replied: # Если это не ответ на сообщение
                    print(f"! Неинтересное сообщение, удаляю: {timestamp} {sender_name}: {m.text}")
                    await client.delete_messages(m.chat_id, m.id)
            if m.media:
                if reactions >= MEDIA_REACTION_LIMIT and not is_github_actions():
                    os.makedirs(chat_name, exist_ok=True)
                    file_path = await m.download_media(file=(chat_name))
                    md.write(f"{timestamp} {sender_name}: {m.text} {emojis} {file_path}\n")
                else:
                    pass # Пропускаем медиа
                    # md.write(f"{timestamp} {sender_name}: {m.text} {emojis} [Media]\n")
            else:
                md.write(f"{sender_name}: {m.text} {emojis}\n")


# CONFIG загружается из config.yml
# Примеры форматов:
#   'channel_name': PeerChannel(123456789),
#   'group_name': PeerChat(123456789),
#   'username': '@username',
#   'user_id': 123456789,
CONFIG = {}

# Загрузка конфигурации из config.yml
import yaml
try:
    with open('config.yml', 'r') as f:
        config_data = yaml.safe_load(f)
        if config_data and 'chats' in config_data:
            for name, chat_cfg in config_data['chats'].items():
                if chat_cfg.get('type') == 'channel':
                    CONFIG[name] = PeerChannel(chat_cfg['id'])
                elif chat_cfg.get('type') == 'group':
                    CONFIG[name] = PeerChat(chat_cfg['id'])
                elif chat_cfg.get('type') == 'username':
                    CONFIG[name] = '@' + chat_cfg['username']
                elif chat_cfg.get('type') == 'user':
                    CONFIG[name] = chat_cfg['id']
except FileNotFoundError:
    print("config.yml не найден, CONFIG пустой")

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
