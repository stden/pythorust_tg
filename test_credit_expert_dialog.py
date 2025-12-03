#!/usr/bin/env python3
"""
Interactive test for Credit Expert Bot
Simulates a conversation flow without needing a bot token
Uses the existing Telegram session to send messages to yourself
"""

import asyncio
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from dotenv import load_dotenv

load_dotenv("/srv/pythorust_tg/.env")

from telethon import TelegramClient

from credit_expert_bot import CREDIT_EXPERT_SYSTEM_PROMPT, MySQLLogger


async def simulate_bot_conversation():
    """Simulate the Credit Expert Bot conversation logic"""

    print("=" * 60)
    print("Credit Expert Bot - Интерактивный тест диалога")
    print("=" * 60)

    # Initialize components
    db = MySQLLogger()
    db.connect()

    # Check if we have Google API key for AI
    google_key = os.getenv("GOOGLE_API_KEY")
    openai_key = os.getenv("OPENAI_API_KEY")

    ai_client = None
    if google_key:
        print("✅ Используем Google Gemini API для генерации ответов")
        try:
            import google.generativeai as genai

            genai.configure(api_key=google_key)
            model = genai.GenerativeModel("gemini-2.0-flash")
            ai_client = model
        except ImportError:
            print("⚠️ google-generativeai не установлен, используем заглушки")
    elif openai_key:
        print("✅ Используем OpenAI API")
        from integrations.openai_client import OpenAIClient

        ai_client = OpenAIClient()
    else:
        print("⚠️ AI API ключ не настроен, используем заглушки")

    # Connect to Telegram
    API_ID = int(os.getenv("TELEGRAM_API_ID"))
    API_HASH = os.getenv("TELEGRAM_API_HASH")
    SESSION_FILE = "/srv/pythorust_tg/telegram_session"

    client = TelegramClient(SESSION_FILE, API_ID, API_HASH)
    await client.connect()

    if not await client.is_user_authorized():
        print("❌ Telegram не авторизован")
        return

    me = await client.get_me()
    print(f"✅ Подключено как: {me.first_name} (@{me.username})")

    # Simulated user ID for testing
    test_user_id = 999999999  # Fake user ID for testing

    # Create a test session
    session_id = db.create_session(test_user_id, bot_name="Credit_Expert_Bot_Test")
    print(f"✅ Создана тестовая сессия: {session_id}")

    conversation_history = []

    async def get_ai_response(user_message: str) -> str:
        """Get AI response based on conversation history"""

        conversation_history.append({"role": "user", "content": user_message})

        if ai_client and hasattr(ai_client, "generate_content"):
            # Google Gemini
            full_prompt = f"{CREDIT_EXPERT_SYSTEM_PROMPT}\n\n"
            for msg in conversation_history:
                role = "Бот" if msg["role"] == "assistant" else "Клиент"
                full_prompt += f"{role}: {msg['content']}\n"
            full_prompt += "Бот:"

            response = ai_client.generate_content(full_prompt)
            reply = response.text.strip()
        elif ai_client and hasattr(ai_client, "chat_completion"):
            # OpenAI
            messages = [{"role": "system", "content": CREDIT_EXPERT_SYSTEM_PROMPT}]
            messages.extend(conversation_history)
            response = await ai_client.chat_completion(messages)
            reply = response.choices[0].message.content
        else:
            # Fallback responses
            if len(conversation_history) == 1:
                reply = "Здравствуйте! Я Дарья, кредитный эксперт. Вижу, что обратились по вопросу долгов. Помогу разобраться. Как к вам обращаться?"
            elif "иван" in user_message.lower() or "меня зовут" in user_message.lower():
                name = user_message.split()[-1] if len(user_message.split()) > 2 else "Иван"
                reply = f"{name}, подскажите, вы уже решили заниматься вопросом с долгами или пока изучаете варианты?"
            elif "изучаю" in user_message.lower():
                reply = "Понятно. Давайте разберемся, подходит ли вам. Расскажите кратко — какая ситуация с долгами?"
            elif "долг" in user_message.lower() or "кредит" in user_message.lower():
                reply = "Понимаю, непростая ситуация. Просрочки есть?"
            elif "да" in user_message.lower() and "просрочк" in str(conversation_history):
                reply = "Коллекторы звонят?"
            elif "звонят" in user_message.lower():
                reply = "Да, тяжело. Хорошая новость — в вашем случае есть законные способы решить проблему. Чтобы дать конкретный план, предлагаю созвониться. 10-15 минут, и вы поймете что делать. Бесплатно и не обязывает. Когда удобно?"
            else:
                reply = "Понимаю вас. Давайте созвонимся, чтобы разобрать вашу ситуацию детально. Это бесплатно и ни к чему не обязывает. Когда вам удобно?"

        conversation_history.append({"role": "assistant", "content": reply})
        return reply

    print("\n" + "=" * 60)
    print("ТЕСТ ДИАЛОГА (введите 'выход' для завершения)")
    print("=" * 60)

    # Initial greeting
    greeting = await get_ai_response("/start")
    print(f"\n🤖 БОТ: {greeting}")

    # Save to Saved Messages for visual verification
    await client.send_message("me", f"🧪 [ТЕСТ БОТА]\n\n🤖 БОТ:\n{greeting}")

    while True:
        try:
            user_input = input("\n👤 ВЫ: ").strip()

            if not user_input:
                continue

            if user_input.lower() in ["выход", "exit", "quit", "q"]:
                print("\n👋 Тест завершён")
                break

            # Get AI response
            response = await get_ai_response(user_input)
            print(f"\n🤖 БОТ: {response}")

            # Save to Saved Messages
            await client.send_message("me", f"👤 ВЫ: {user_input}\n\n🤖 БОТ: {response}")

            # Save to database
            db.save_message(
                user_id=test_user_id,
                message_id=len(conversation_history),
                text=user_input,
                direction="incoming",
                bot_name="Credit_Expert_Bot_Test",
            )
            db.save_message(
                user_id=test_user_id,
                message_id=len(conversation_history) + 1000,
                text=response,
                direction="outgoing",
                bot_name="Credit_Expert_Bot_Test",
            )

        except KeyboardInterrupt:
            print("\n\n👋 Тест прерван")
            break
        except EOFError:
            print("\n👋 Тест завершён")
            break

    # Cleanup
    db.close()
    await client.disconnect()

    print("\n" + "=" * 60)
    print("Диалог сохранён в 'Сохранённые сообщения' в Telegram")
    print("=" * 60)


if __name__ == "__main__":
    asyncio.run(simulate_bot_conversation())
