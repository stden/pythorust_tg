#!/usr/bin/env python3
"""
Отправка виральных вопросов в Telegram чаты
"""
import asyncio
import os
from telethon import TelegramClient
from dotenv import load_dotenv

load_dotenv()

API_ID = int(os.getenv('TELEGRAM_API_ID', '0'))
API_HASH = os.getenv('TELEGRAM_API_HASH', '')
SESSION_FILE = os.getenv('TELEGRAM_SESSION_NAME', 'telegram_session') + '.session'

# Вопросы для каждого чата
QUESTIONS = {
    'Golang GO': """Реально ли попасть в Яндекс/Авито на Go без олимпиадных регалий в 2025?

Или там только ICPC финалисты?

Кто проходил собесы недавно — что спрашивали, сколько этапов, какие алгоритмы?""",

    'вайбкодеры': """Claude Haiku 4.5 vs GPT-4.5-mini: кто реально выиграл?

Anthropic говорят что "лучше всех на рынке", OpenAI молчит. Кто тестил обе модели на реальных задачах (не бенчмарки)? Поделитесь примерами где одна слила другую.""",

    'Хара': """Какая самая безумная синхрония случалась в вашей жизни?

У меня: читала книгу про лотерею → получила 'случайные' числа → поставила → выиграла ровно столько, сколько нужно было на книги.

Поделитесь своими историями 🙏✨"""
}


async def send_question(chat_name: str, question: str):
    """Send question to chat"""
    async with TelegramClient(SESSION_FILE, API_ID, API_HASH) as client:
        # Find chat
        dialogs = await client.get_dialogs()
        target_chat = None

        for dialog in dialogs:
            if dialog.title and chat_name.lower() in dialog.title.lower():
                target_chat = dialog
                break

        if not target_chat:
            print(f"❌ Чат '{chat_name}' не найден")
            return False

        # Send message
        try:
            await client.send_message(target_chat.entity, question)
            print(f"✅ Отправлено в '{target_chat.title}'")
            print(f"   Вопрос: {question[:50]}...")
            return True
        except Exception as e:
            print(f"❌ Ошибка отправки в '{chat_name}': {e}")
            return False


async def main():
    """Send questions to all chats"""
    print("📤 Отправка виральных вопросов...\n")

    # Send to Golang GO first (highest expected engagement)
    await send_question('Golang GO', QUESTIONS['Golang GO'])
    await asyncio.sleep(2)  # Delay between messages

    # Send to вайбкодеры
    await send_question('вайбкодеры', QUESTIONS['вайбкодеры'])
    await asyncio.sleep(2)

    # Send to Хара
    await send_question('Хара', QUESTIONS['Хара'])

    print("\n✅ Все вопросы отправлены!")
    print("\n📊 Теперь отслеживайте реакции и отвечайте на комменты в первые 5 минут для максимального engagement.")


if __name__ == '__main__':
    asyncio.run(main())
