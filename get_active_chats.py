from telegram_session import get_client, SessionLock

# Получаем клиент с SQLite сессией
client = get_client()


async def get_active_chats():
    dialogs = await client.get_dialogs(limit=50)

    chat_activity = []
    for dialog in dialogs:
        if dialog.is_channel or dialog.is_group:
            try:
                messages = await client.get_messages(dialog.entity, limit=10)
                if messages:
                    latest_date = messages[0].date
                    chat_activity.append({
                        'title': dialog.title,
                        'id': dialog.entity.id,
                        'last_message': latest_date,
                        'unread': dialog.unread_count,
                        'type': 'channel' if dialog.is_channel else 'group'
                    })
            except Exception as e:
                pass

    # Сортируем по дате последнего сообщения
    chat_activity.sort(key=lambda x: x['last_message'], reverse=True)

    print('Наиболее активные чаты:\n')
    for i, chat in enumerate(chat_activity[:20], 1):
        print(f'{i}. {chat["title"]}')
        print(f'   ID: {chat["id"]} | Тип: {chat["type"]} | Непрочитано: {chat["unread"]}')
        print(f'   Последнее сообщение: {chat["last_message"].strftime("%d.%m.%Y %H:%M")}')
        print()


with SessionLock():  # Защита от параллельного запуска
    with client:
        client.loop.run_until_complete(get_active_chats())
