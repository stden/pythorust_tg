#!/usr/bin/env python3
"""Delete all Zoom links from chat history that were sent by me."""
from telegram_session import get_client, SessionLock

# Получаем клиент с SQLite сессией
client = get_client()


async def delete_zoom_in_chat(entity, chat_name):
    """Delete Zoom links in a specific chat."""
    print(f"\nПоиск сообщений с Zoom ссылками в чате {chat_name}")

    deleted_count = 0
    async for m in client.iter_messages(entity, limit=3000):
        if m.out and m.text and 'zoom.us/' in m.text:
            timestamp = m.date.strftime('%d.%m.%Y %H:%M:%S')
            preview = m.text[:50].replace('\n', ' ')
            print(f"Удаляю: {timestamp}: {preview}...")

            # Удаляем сообщение для обоих пользователей (revoke=True)
            await client.delete_messages(entity, m.id, revoke=True)
            deleted_count += 1

    print(f"Удалено {deleted_count} сообщений с Zoom ссылками из {chat_name}")
    return deleted_count


async def main():
    """Delete Zoom links from multiple chats."""
    import os
    # Список чатов для очистки из переменной окружения ZOOM_CLEANUP_CHATS
    # Формат: @username1,@username2,@username3
    chats_str = os.getenv("ZOOM_CLEANUP_CHATS", "")
    chats = [c.strip() for c in chats_str.split(",") if c.strip()]

    if not chats:
        print("ZOOM_CLEANUP_CHATS не задан. Укажите чаты через запятую: @user1,@user2")
        return

    total_deleted = 0
    for chat in chats:
        try:
            entity = await client.get_entity(chat)
            deleted = await delete_zoom_in_chat(entity, chat)
            total_deleted += deleted
        except Exception as e:
            print(f"Ошибка при обработке {chat}: {e}")

    print(f"\n=== Итого удалено {total_deleted} сообщений с Zoom ссылками ===")


if __name__ == '__main__':
    try:
        with SessionLock():  # Защита от параллельного запуска
            with client:
                client.loop.run_until_complete(main())
    except Exception as e:
        print(f"Ошибка: {e}")
        import traceback
        traceback.print_exc()
