import os
import sys
from pathlib import Path

import yaml
from telethon.tl.types import PeerChannel, PeerChat, ReactionEmoji
from telegram_session import get_client, SessionLock, known_senders

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
            # Определяем имя отправителя
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
                known[m.sender_id] = sender_name
            timestamp = m.date.strftime('%d.%m.%Y %H:%M:%S')
            # Подсчет реакций
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
            # Обработка медиа
            if m.media:
                if reactions >= 1000:
                    os.makedirs(chat_name, exist_ok=True)
                    print(f"{timestamp} {sender_name}: {m.text} {emojis} --->\n")
                    file_path = await m.download_media(file=(chat_name))
                    print(f"{timestamp} {sender_name}: {m.text} {emojis} {file_path}\n")
                    md.write(f"{timestamp} {sender_name}: {m.text} {emojis} {file_path}\n")
                else:
                    md.write(f"{timestamp} {sender_name}: {m.text} {emojis} [Media]\n")
            else:
                md.write(f"{sender_name}: {m.text} {emojis}\n")


def load_chats_from_config(config_path: str = "config.yml") -> dict:
    path = Path(config_path)
    if not path.is_file():
        path = Path(__file__).with_name(config_path)
    with path.open("r", encoding="utf-8") as fh:
        data = yaml.safe_load(fh) or {}
    chats_cfg = data.get("chats", {})

    def to_entity(cfg: dict):
        ctype = cfg.get("type")
        if ctype == "channel":
            return PeerChannel(int(cfg["id"]))
        if ctype == "group":
            return PeerChat(int(cfg["id"]))
        if ctype == "user":
            return int(cfg["id"])
        if ctype == "username":
            username = cfg["username"]
            return username if username.startswith("@") else f"@{username}"
        raise ValueError(f"Unknown chat type: {ctype}")

    return {name: to_entity(cfg) for name, cfg in chats_cfg.items()}


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
