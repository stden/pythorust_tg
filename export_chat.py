import sys
from telegram_session import get_client, SessionLock

# Получаем клиент с SQLite сессией
client = get_client()


async def export_chat(username, output_file, limit=100):
    try:
        entity = await client.get_entity(username)
        print(f"Экспортирую чат: {entity.first_name or ''} {entity.last_name or ''} (@{username})")

        messages = await client.get_messages(entity, limit=limit)

        with open(output_file, 'w', encoding='utf-8') as f:
            f.write(f"# Чат с @{username}\n\n")

            for m in messages[::-1]:  # Сначала старые сообщения
                timestamp = m.date.strftime('%d.%m.%Y %H:%M:%S')
                sender = "Я" if m.out else entity.first_name or username

                if m.text:
                    f.write(f"{timestamp} {sender}: {m.text}\n")
                elif m.media:
                    f.write(f"{timestamp} {sender}: [Media]\n")

        print(f"Экспортировано {len(messages)} сообщений в {output_file}")

    except Exception as e:
        print(f"Ошибка: {e}")
        import traceback
        traceback.print_exc()


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python export_chat.py <username> [output_file]")
        sys.exit(1)
    username = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else f'{username}.md'

    with SessionLock():  # Защита от параллельного запуска
        with client:
            client.loop.run_until_complete(export_chat(username, output_file))
