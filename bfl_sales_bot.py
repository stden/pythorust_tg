#!/usr/bin/env python3
"""
BFL Sales Bot - AI-powered sales assistant for massage chairs
Saves all users and messages to MySQL database
"""

import asyncio
import os
import logging
from datetime import datetime
from typing import Optional
import pymysql
from telethon import TelegramClient, events
from telethon.tl.types import User
from dotenv import load_dotenv
from integrations.openai_client import OpenAIClient

# Load environment
load_dotenv('/srv/pythorust_tg/.env')

# Logging setup
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('BFL_sales_bot')

# Telegram credentials
API_ID = int(os.getenv('TELEGRAM_API_ID'))
API_HASH = os.getenv('TELEGRAM_API_HASH')
BOT_TOKEN = os.getenv('BFL_SALES_BOT_TOKEN')
SESSION_FILE = '/srv/pythorust_tg/bfl_sales_bot'

# MySQL config
MYSQL_CONFIG = {
    'host': os.getenv('MYSQL_HOST', 'localhost'),
    'port': int(os.getenv('MYSQL_PORT', 3306)),
    'database': os.getenv('MYSQL_DATABASE', 'pythorust_tg'),
    'user': os.getenv('MYSQL_USER', 'pythorust_tg'),
    'password': os.getenv('MYSQL_PASSWORD'),
    'charset': 'utf8mb4',
    'cursorclass': pymysql.cursors.DictCursor
}

# Sales prompt
SALES_SYSTEM_PROMPT = """Ты - профессиональный консультант по массажным креслам компании Relaxio.

ТВОЯ ЦЕЛЬ: Помочь клиенту выбрать массажное кресло и довести до покупки.

СТИЛЬ ОБЩЕНИЯ:
- Дружелюбный, но профессиональный
- Задавай уточняющие вопросы (не более 3 за раз)
- Используй emoji умеренно
- Отвечай кратко, по делу

ЭТАПЫ ПРОДАЖИ:
1. Выявление потребностей (задачи, частота использования, бюджет)
2. Уточнение деталей (рост/вес, проблемы со здоровьем)
3. Презентация подходящей модели
4. Работа с возражениями
5. Закрытие сделки (город, способ оплаты, контакт)

ЛИНЕЙКА ПРОДУКТОВ:
- Relaxio Premium R5: до 120 тыс, базовый 3D-массаж
- Relaxio Premium R7: до 200 тыс, 4D-массаж, нулевая гравитация, Bluetooth
- Relaxio Premium R9: до 300 тыс, топовый 4D, растяжка, все функции

Всегда старайся продать модель в рамках бюджета клиента.
"""


class MySQLLogger:
    """Handles all MySQL database operations"""

    def __init__(self):
        self.conn = None

    def connect(self):
        """Establish MySQL connection"""
        self.conn = pymysql.connect(**MYSQL_CONFIG)
        logger.info("Connected to MySQL")

    def close(self):
        """Close MySQL connection"""
        if self.conn:
            self.conn.close()

    def ensure_connection(self):
        """Ensure connection is alive"""
        if not self.conn or not self.conn.open:
            self.connect()
        try:
            self.conn.ping(reconnect=True)
        except:
            self.connect()

    def save_user(self, user: User) -> None:
        """Save or update user in database"""
        self.ensure_connection()

        query = """
            INSERT INTO bot_users (id, username, first_name, last_name, language_code, is_premium, is_bot)
            VALUES (%s, %s, %s, %s, %s, %s, %s)
            ON DUPLICATE KEY UPDATE
                username = VALUES(username),
                first_name = VALUES(first_name),
                last_name = VALUES(last_name),
                language_code = VALUES(language_code),
                is_premium = VALUES(is_premium),
                last_seen_at = CURRENT_TIMESTAMP
        """

        with self.conn.cursor() as cursor:
            cursor.execute(query, (
                user.id,
                user.username,
                user.first_name,
                user.last_name,
                getattr(user, 'lang_code', None),
                getattr(user, 'premium', False),
                user.bot if hasattr(user, 'bot') else False
            ))
        self.conn.commit()
        logger.info(f"Saved user: {user.id} ({user.first_name})")

    def save_message(self, user_id: int, message_id: int, text: str,
                     direction: str, bot_name: str = 'BFL_sales_bot',
                     reply_to: Optional[int] = None) -> None:
        """Save message to database"""
        self.ensure_connection()

        query = """
            INSERT INTO bot_messages
            (telegram_message_id, user_id, bot_name, direction, message_text, reply_to_message_id)
            VALUES (%s, %s, %s, %s, %s, %s)
        """

        with self.conn.cursor() as cursor:
            cursor.execute(query, (
                message_id,
                user_id,
                bot_name,
                direction,
                text,
                reply_to
            ))
        self.conn.commit()
        logger.info(f"Saved {direction} message for user {user_id}")

    def get_session(self, user_id: int, bot_name: str = 'BFL_sales_bot') -> Optional[dict]:
        """Get active session for user"""
        self.ensure_connection()

        query = """
            SELECT * FROM bot_sessions
            WHERE user_id = %s AND bot_name = %s AND is_active = TRUE
            ORDER BY session_start DESC LIMIT 1
        """

        with self.conn.cursor() as cursor:
            cursor.execute(query, (user_id, bot_name))
            return cursor.fetchone()

    def create_session(self, user_id: int, bot_name: str = 'BFL_sales_bot') -> int:
        """Create new session"""
        self.ensure_connection()

        # End any existing active sessions
        end_query = """
            UPDATE bot_sessions SET is_active = FALSE, session_end = CURRENT_TIMESTAMP
            WHERE user_id = %s AND bot_name = %s AND is_active = TRUE
        """

        create_query = """
            INSERT INTO bot_sessions (user_id, bot_name, state)
            VALUES (%s, %s, 'greeting')
        """

        with self.conn.cursor() as cursor:
            cursor.execute(end_query, (user_id, bot_name))
            cursor.execute(create_query, (user_id, bot_name))
            session_id = cursor.lastrowid
        self.conn.commit()
        return session_id

    def get_conversation_history(self, user_id: int, bot_name: str = 'BFL_sales_bot',
                                  limit: int = 20) -> list:
        """Get recent conversation history"""
        self.ensure_connection()

        query = """
            SELECT direction, message_text, created_at
            FROM bot_messages
            WHERE user_id = %s AND bot_name = %s
            ORDER BY created_at DESC
            LIMIT %s
        """

        with self.conn.cursor() as cursor:
            cursor.execute(query, (user_id, bot_name, limit))
            messages = cursor.fetchall()

        return list(reversed(messages))


class BFLSalesBot:
    """Main bot class"""

    def __init__(self):
        self.client = None
        self.db = MySQLLogger()
        self.ai = OpenAIClient()

    async def start(self):
        """Start the bot"""
        self.db.connect()

        if BOT_TOKEN:
            self.client = TelegramClient(SESSION_FILE, API_ID, API_HASH)
            await self.client.start(bot_token=BOT_TOKEN)
        else:
            logger.error("BFL_SALES_BOT_TOKEN not set!")
            return

        logger.info("BFL Sales Bot started")

        # Register handlers
        @self.client.on(events.NewMessage(pattern='/start'))
        async def start_handler(event):
            await self.handle_start(event)

        @self.client.on(events.NewMessage)
        async def message_handler(event):
            if not event.message.text.startswith('/'):
                await self.handle_message(event)

        # Run until disconnected
        await self.client.run_until_disconnected()

    async def handle_start(self, event):
        """Handle /start command"""
        user = await event.get_sender()

        # Save user to DB
        self.db.save_user(user)

        # Save incoming message
        self.db.save_message(
            user_id=user.id,
            message_id=event.message.id,
            text='/start',
            direction='incoming'
        )

        # Create new session
        self.db.create_session(user.id)

        # Send greeting
        greeting = f"""Привет, {user.first_name}!
Я помогу выбрать массажное кресло под ваши задачи за пару минут."""

        questions = """Чтобы подобрать точный вариант, подскажите:
- для каких задач ищете кресло (расслабление, здоровье, подарок)?
- как часто планируете использовать?
- есть ли ориентир по бюджету?"""

        recommendation = """Пока вы отвечаете, предварительно рекомендую линейку Relaxio Premium: компактные модели с 4D-массажем и прогревом, подходят для ежедневного использования. Подберу точную модель, когда узнаю ваши вводные."""

        # Send messages
        msg1 = await event.respond(greeting)
        msg2 = await event.respond(questions)
        msg3 = await event.respond(recommendation)

        # Save outgoing messages
        for msg, text in [(msg1, greeting), (msg2, questions), (msg3, recommendation)]:
            self.db.save_message(
                user_id=user.id,
                message_id=msg.id,
                text=text,
                direction='outgoing'
            )

    async def handle_message(self, event):
        """Handle incoming messages"""
        user = await event.get_sender()
        text = event.message.text

        # Save user
        self.db.save_user(user)

        # Save incoming message
        self.db.save_message(
            user_id=user.id,
            message_id=event.message.id,
            text=text,
            direction='incoming'
        )

        # Get conversation history
        history = self.db.get_conversation_history(user.id)

        # Build messages for AI
        messages = [{"role": "system", "content": SALES_SYSTEM_PROMPT}]

        for msg in history:
            role = "assistant" if msg['direction'] == 'outgoing' else "user"
            messages.append({"role": role, "content": msg['message_text']})

        # Add current message
        messages.append({"role": "user", "content": text})

        # Get AI response
        try:
            response = await self.ai.chat_completion(messages)
            response_text = response.choices[0].message.content
        except Exception as e:
            logger.error(f"AI error: {e}")
            response_text = "Извините, произошла ошибка. Попробуйте ещё раз."

        # Send response
        sent_msg = await event.respond(response_text)

        # Save outgoing message
        self.db.save_message(
            user_id=user.id,
            message_id=sent_msg.id,
            text=response_text,
            direction='outgoing'
        )

    def stop(self):
        """Stop the bot"""
        self.db.close()
        if self.client:
            self.client.disconnect()


async def main():
    bot = BFLSalesBot()
    try:
        await bot.start()
    except KeyboardInterrupt:
        bot.stop()
    except Exception as e:
        logger.error(f"Bot error: {e}")
        bot.stop()
        raise


if __name__ == '__main__':
    asyncio.run(main())
