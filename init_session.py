#!/usr/bin/env python3
"""
Скрипт для первичной инициализации Telegram сессии.

ИСПОЛЬЗУЙТЕ ЭТОТ СКРИПТ ТОЛЬКО ОДИН РАЗ для создания session файла!

⚠️  ВНИМАНИЕ:
- Запуск создаст НОВУЮ сессию
- Это ВЫТЕСНИТ активные сессии на других устройствах
- Вас выкинет из Telegram на телефоне/компьютере
- После создания сессии используйте обычные скрипты

Использование:
    uv run python init_session.py

После запуска:
1. Введите код из Telegram
2. Дождитесь сообщения "✅ Сессия успешно создана"
3. Больше НИКОГДА не запускайте этот скрипт!
"""
import sys
from dotenv import load_dotenv
from telethon import TelegramClient

load_dotenv()

# Конфигурация (загружается из telegram_session.py, значения берутся из .env)
from telegram_session import API_HASH, API_ID, PHONE, SESSION_NAME


async def init():
    """Инициализация новой сессии."""
    print(f"""
╔═══════════════════════════════════════════════════════════════╗
║  ИНИЦИАЛИЗАЦИЯ НОВОЙ TELEGRAM СЕССИИ                          ║
╚═══════════════════════════════════════════════════════════════╝

⚠️  КРИТИЧЕСКОЕ ПРЕДУПРЕЖДЕНИЕ:
   Этот скрипт создаст НОВУЮ сессию для номера {PHONE}

   ЭТО ПРИВЕДЁТ К:
   - Выходу из Telegram на всех других устройствах
   - Потере активных сессий

   Вы УВЕРЕНЫ, что хотите продолжить?

   Введите 'YES' (заглавными) для подтверждения: """)

    confirmation = input().strip()
    if confirmation != 'YES':
        print("\n❌ Отменено. Session файл не создан.")
        sys.exit(0)

    print(f"\n🔄 Создаю новую сессию для {PHONE}...")
    print("📱 Ожидайте код подтверждения в Telegram...\n")


async def main():
    """Основная функция инициализации."""
    client = TelegramClient(SESSION_NAME, API_ID, API_HASH)

    await client.start(phone=PHONE)

    me = await client.get_me()
    print(f"""
╔═══════════════════════════════════════════════════════════════╗
║  ✅ СЕССИЯ УСПЕШНО СОЗДАНА                                    ║
╚═══════════════════════════════════════════════════════════════╝

Профиль:
  Имя: {me.first_name} {me.last_name or ''}
  Username: @{me.username or 'не указан'}
  ID: {me.id}
  Телефон: {me.phone}

Файл сессии: {SESSION_NAME}.session

Теперь вы можете:
1. Запускать обычные скрипты (read.py, tg.py, и т.д.)
2. Скрипты будут использовать эту сессию автоматически
3. НИКОГДА больше не запускайте init_session.py!

⚠️  ВАЖНО: Сделайте резервную копию файла {SESSION_NAME}.session
""")

    await client.disconnect()


if __name__ == '__main__':
    import asyncio
    asyncio.run(init())
    asyncio.run(main())
