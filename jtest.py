#!/usr/bin/env python3
import asyncio, os
from telethon import TelegramClient
from dotenv import load_dotenv

load_dotenv("/srv/pythorust_tg/.env")
API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
SESSION = "/srv/pythorust_tg/telegram_session"
BOT_ID = 8022688437

async def test():
    client = TelegramClient(SESSION, API_ID, API_HASH)
    await client.start()
    print("=== ТЕСТ ВОССТАНОВЛЕНИЯ СЕССИИ ===")

    # Отправляем привет чтобы проверить восстановление сессии
    await client.send_message(BOT_ID, "привет")
    await asyncio.sleep(4)
    msgs = await client.get_messages(BOT_ID, limit=2)
    for m in msgs:
        if not m.out:
            print(f"Ответ: {m.text[:200]}...")
            if "Помню наш разговор" in m.text:
                print("✅ Женский род: 'Помню наш разговор' вместо 'Нашел сессию'")
            elif "Нашел" in m.text:
                print("❌ Всё ещё мужской род")
            else:
                print("ℹ️ Другой ответ (возможно новая сессия)")
            break

    await client.disconnect()

asyncio.run(test())
