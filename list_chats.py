from telegram_session import get_client, SessionLock

# Получаем клиент с SQLite сессией
client = get_client()


async def main():
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

    # Сохраняем в YAML файл
    with open('chats.yml', 'w', encoding='utf-8') as f:
        f.write('# Активные чаты Telegram\n')
        if chat_activity:
            f.write('# Обновлено: ' + chat_activity[0]['last_message'].strftime("%d.%m.%Y %H:%M") + '\n\n')
        f.write('chats:\n')
        for chat in chat_activity:
            f.write(f'  - title: "{chat["title"]}"\n')
            f.write(f'    id: {chat["id"]}\n')
            f.write(f'    type: {chat["type"]}\n')
            f.write(f'    unread: {chat["unread"]}\n')
            f.write(f'    last_message: "{chat["last_message"].strftime("%d.%m.%Y %H:%M:%S")}"\n')
            f.write('\n')

    print(f'\nИнформация сохранена в chats.yml ({len(chat_activity)} чатов)')


with SessionLock():  # Защита от параллельного запуска
    with client:
        client.loop.run_until_complete(main())
