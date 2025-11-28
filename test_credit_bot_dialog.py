#!/usr/bin/env python3
"""
Интерактивный тест диалога Credit Expert Bot через Telethon.
Симулирует диалог с ботом, используя реальный Telegram API.
"""

import asyncio
import os
from dotenv import load_dotenv
from telethon import TelegramClient
from telethon.tl.types import User

load_dotenv()

API_ID = int(os.getenv('TELEGRAM_API_ID'))
API_HASH = os.getenv('TELEGRAM_API_HASH')
SESSION_NAME = os.getenv('TELEGRAM_SESSION_NAME', 'telegram_session')

# Бот для тестирования (замените на реального бота или используйте echo-бот для теста)
BOT_USERNAME = os.getenv('CREDIT_EXPERT_BOT_USERNAME', '@BotFather')  # Для теста

async def test_bot_dialog():
    """Тестирование диалога с ботом."""
    print("=" * 60)
    print("🧪 Тест диалога Credit Expert Bot")
    print("=" * 60)
    
    client = TelegramClient(SESSION_NAME, API_ID, API_HASH)
    
    try:
        await client.start()
        print("✅ Telethon подключен")
        
        # Получаем информацию о текущем пользователе
        me = await client.get_me()
        print(f"✅ Авторизован как: {me.first_name} (@{me.username})")
        
        # Проверяем наличие бота
        bot_username = os.getenv('CREDIT_EXPERT_BOT_USERNAME')
        if not bot_username:
            print("\n⚠️  CREDIT_EXPERT_BOT_USERNAME не задан в .env")
            print("   Для полного теста нужно:")
            print("   1. Создать бота через @BotFather")
            print("   2. Добавить CREDIT_EXPERT_BOT_TOKEN в .env")
            print("   3. Добавить CREDIT_EXPERT_BOT_USERNAME в .env")
            print("\n📝 Демонстрация логики бота без Telegram:")
            await demo_bot_logic()
        else:
            print(f"\n🤖 Тестирование бота: {bot_username}")
            await test_real_bot(client, bot_username)
            
    except Exception as e:
        print(f"❌ Ошибка: {e}")
    finally:
        await client.disconnect()
        print("\n✅ Сессия закрыта")


async def demo_bot_logic():
    """Демонстрация логики бота без реального Telegram."""
    from unittest.mock import MagicMock, AsyncMock
    
    # Импортируем системный промпт
    import sys
    sys.path.insert(0, '/srv/pythorust_tg')
    
    try:
        from credit_expert_bot import CREDIT_EXPERT_SYSTEM_PROMPT
        print("\n📋 Системный промпт загружен:")
        print("-" * 40)
        # Показываем первые 500 символов
        print(CREDIT_EXPERT_SYSTEM_PROMPT[:500] + "...")
        print("-" * 40)
    except ImportError as e:
        print(f"⚠️  Не удалось импортировать: {e}")
        return
    
    # Симуляция диалога
    print("\n🎭 Симуляция диалога:")
    print("-" * 40)
    
    dialog = [
        ("Клиент", "Здравствуйте, хочу узнать про списание долгов"),
        ("Бот", "Здравствуйте! Я Дарья, кредитный эксперт. Вижу, что обратились по вопросу долгов. Помогу разобраться. Как к вам обращаться?"),
        ("Клиент", "Иван"),
        ("Бот", "Иван, подскажите, вы уже решили заниматься вопросом с долгами или пока изучаете варианты?"),
        ("Клиент", "Пока изучаю"),
        ("Бот", "Понятно. Какая ситуация с долгами? Опишите кратко"),
        ("Клиент", "Долги в банках, около 500 тысяч"),
        ("Бот", "Понимаю, непростая ситуация. Просрочки есть?"),
        ("Клиент", "Да, 2 месяца"),
        ("Бот", "Тяжело. Коллекторы звонят?"),
        ("Клиент", "Звонят постоянно"),
        ("Бот", "Иван, понимаю вас — и страшно, и непонятно что делать. Многие обращаются с такой ситуацией, выход всегда есть.\n\nЧтобы дать конкретный план действий, предлагаю созвониться — так быстрее. 10-15 минут, и вы получите четкое понимание. Это бесплатно и ни к чему не обязывает. Когда удобно созвониться?"),
    ]
    
    for role, message in dialog:
        emoji = "👤" if role == "Клиент" else "🤖"
        print(f"{emoji} {role}: {message}\n")
        await asyncio.sleep(0.5)
    
    print("-" * 40)
    print("✅ Демонстрация завершена")


async def test_real_bot(client, bot_username):
    """Тестирование реального бота через Telegram."""
    try:
        # Получаем entity бота
        bot_entity = await client.get_entity(bot_username)
        print(f"✅ Бот найден: {bot_entity.first_name}")
        
        # Отправляем /start
        print("\n📤 Отправляем /start...")
        await client.send_message(bot_entity, '/start')
        
        # Ждём ответа
        await asyncio.sleep(3)
        
        # Получаем последние сообщения
        messages = await client.get_messages(bot_entity, limit=5)
        print("\n📥 Последние сообщения:")
        for msg in reversed(messages):
            sender = "🤖 Бот" if msg.out == False else "👤 Я"
            print(f"{sender}: {msg.text[:100] if msg.text else '[медиа]'}...")
        
    except Exception as e:
        print(f"❌ Ошибка при работе с ботом: {e}")


if __name__ == '__main__':
    asyncio.run(test_bot_dialog())
