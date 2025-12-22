#!/usr/bin/env python3
"""
Credit Expert Bot - AI-powered assistant for debt relief services
Saves all users and messages to MySQL database
"""

import asyncio
import os
import logging
import re
from typing import Optional
import pymysql
from telethon import TelegramClient, events
from telethon.tl.types import User
from dotenv import load_dotenv
from integrations.openai_client import OpenAIClient


# Emoji removal pattern
EMOJI_PATTERN = re.compile(
    "["
    "\U0001f600-\U0001f64f"  # emoticons
    "\U0001f300-\U0001f5ff"  # symbols & pictographs
    "\U0001f680-\U0001f6ff"  # transport & map symbols
    "\U0001f1e0-\U0001f1ff"  # flags
    "\U00002702-\U000027b0"  # dingbats
    "\U000024c2-\U0001f251"  # enclosed characters
    "\U0001f900-\U0001f9ff"  # supplemental symbols
    "\U0001fa00-\U0001fa6f"  # chess symbols
    "\U0001fa70-\U0001faff"  # symbols extended-A
    "\U00002600-\U000026ff"  # misc symbols
    "\U00002700-\U000027bf"  # dingbats
    "]+",
    flags=re.UNICODE,
)

# Name validation pattern (Russian/English names)
NAME_PATTERN = re.compile(r"^[А-Яа-яЁёA-Za-z][А-Яа-яЁёA-Za-z\-\s]{1,30}$")


def strip_emoji(text: str) -> str:
    """Remove emoji from text"""
    return EMOJI_PATTERN.sub("", text).strip()


def is_valid_name(name: str) -> bool:
    """Check if text looks like a valid name"""
    name = name.strip()
    # Too short or too long
    if len(name) < 2 or len(name) > 30:
        return False
    # Contains only letters, spaces, hyphens (for double names)
    if not NAME_PATTERN.match(name):
        return False
    # Probably not a name if contains common non-name words
    non_names = [
        "привет",
        "здравствуй",
        "добрый",
        "вечер",
        "день",
        "утро",
        "да",
        "нет",
        "ок",
        "окей",
        "хорошо",
        "ладно",
        "понял",
        "спасибо",
        "пожалуйста",
        "извини",
        "простите",
    ]
    if name.lower() in non_names:
        return False
    return True


# Load environment
load_dotenv()

# Logging setup
logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s")
logger = logging.getLogger("Credit_Expert_Bot")

# Telegram credentials
API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
BOT_TOKEN = os.getenv("CREDIT_EXPERT_BOT_TOKEN")  # Assuming a new token env var
SESSION_FILE = "credit_expert_bot"

# MySQL config
MYSQL_CONFIG = {
    "host": os.getenv("MYSQL_HOST", "localhost"),
    "port": int(os.getenv("MYSQL_PORT", 3306)),
    "database": os.getenv("MYSQL_DATABASE", "pythorust_tg"),
    "user": os.getenv("MYSQL_USER", "pythorust_tg"),
    "password": os.getenv("MYSQL_PASSWORD"),
    "charset": "utf8mb4",
    "cursorclass": pymysql.cursors.DictCursor,
}

# System prompt based on user instruction
CREDIT_EXPERT_SYSTEM_PROMPT = """Ты — кредитный эксперт Дарья из ФЦБ (Федеральный Центр Банкротства).
ТВОЯ ЦЕЛЬ: Получить номер телефона клиента для консультации.

СТИЛЬ ОБЩЕНИЯ:
- Профессиональный и деловой тон
- Без эмодзи и восклицательных знаков
- Краткие, чёткие ответы (2-4 предложения)
- Уверенный эксперт, а не подруга

ИНСТРУКЦИЯ ПО ДИАЛОГУ:

ШАГ 1: ЗНАКОМСТВО
"Добрый день. Я Дарья, кредитный эксперт ФЦБ. Как могу к вам обращаться?"

ШАГ 2: КВАЛИФИКАЦИЯ
После получения имени спроси: "[Имя], расскажите коротко вашу ситуацию — какой примерно долг и есть ли просрочки?"

Уточняющие вопросы (по одному за раз):
- Коллекторы уже звонят?
- Долги только в МФО или банки тоже?
- Есть ли имущество, которое хотите сохранить?

ШАГ 3: ПЕРЕХОД К ЗВОНКУ
"[Имя], чтобы дать конкретные рекомендации по вашей ситуации, предлагаю созвониться на 10 минут. Консультация бесплатная. Какой номер для связи?"

ОТРАБОТКА ВОЗРАЖЕНИЙ:
- "Расскажите сначала": "Каждый случай индивидуален. На звонке за 10 минут разберём ваш — это эффективнее переписки."
- "Сколько стоит?": "Зависит от суммы долга и ситуации. Консультация бесплатная — расскажу варианты и стоимость."
- "Подумаю": "Хорошо. Учтите, что пени и проценты продолжают расти. Готова ответить, когда решите."
- "Можно в переписке?": "В переписке сложно учесть все нюансы. 10 минут разговора заменят час переписки. Давайте попробуем?"

СТРОГИЕ ПРАВИЛА:
- Никаких эмодзи
- Никаких "подружеских" выражений
- Цель — телефон для консультации
- Один вопрос за раз
- Не затягивай переписку — веди к звонку
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
        except Exception:
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
            cursor.execute(
                query,
                (
                    user.id,
                    user.username,
                    user.first_name,
                    user.last_name,
                    getattr(user, "lang_code", None),
                    getattr(user, "premium", False),
                    user.bot if hasattr(user, "bot") else False,
                ),
            )
        self.conn.commit()
        logger.info(f"Saved user: {user.id} ({user.first_name})")

    def save_message(
        self,
        user_id: int,
        message_id: int,
        text: str,
        direction: str,
        bot_name: str = "Credit_Expert_Bot",
        reply_to: Optional[int] = None,
    ) -> None:
        """Save message to database"""
        self.ensure_connection()

        query = """
            INSERT INTO bot_messages
            (telegram_message_id, user_id, bot_name, direction, message_text, reply_to_message_id)
            VALUES (%s, %s, %s, %s, %s, %s)
        """

        with self.conn.cursor() as cursor:
            cursor.execute(query, (message_id, user_id, bot_name, direction, text, reply_to))
        self.conn.commit()
        logger.info(f"Saved {direction} message for user {user_id}")

    def get_session(self, user_id: int, bot_name: str = "Credit_Expert_Bot") -> Optional[dict]:
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

    def create_session(self, user_id: int, bot_name: str = "Credit_Expert_Bot") -> int:
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

    def get_conversation_history(self, user_id: int, bot_name: str = "Credit_Expert_Bot", limit: int = 20) -> list:
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


class CreditExpertBot:
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
            logger.error("CREDIT_EXPERT_BOT_TOKEN not set!")
            # For testing purposes, we might want to allow running without token if mocked
            # but in production code it should return.
            # We'll keep it as is for now.
            return

        logger.info("Credit Expert Bot started")

        # Register handlers
        @self.client.on(events.NewMessage(pattern="/start"))
        async def start_handler(event):
            await self.handle_start(event)

        @self.client.on(events.NewMessage)
        async def message_handler(event):
            if not event.message.text.startswith("/"):
                await self.handle_message(event)

        # Run until disconnected
        await self.client.run_until_disconnected()

    async def handle_start(self, event):
        """Handle /start command"""
        user = await event.get_sender()

        # Save user to DB
        self.db.save_user(user)

        # Save incoming message
        self.db.save_message(user_id=user.id, message_id=event.message.id, text="/start", direction="incoming")

        # Create new session
        self.db.create_session(user.id)

        # Send greeting (Step 1) - professional tone, no emoji
        greeting = "Добрый день. Я Дарья, кредитный эксперт ФЦБ. Как могу к вам обращаться?"

        # Send messages
        msg1 = await event.respond(greeting)

        # Save outgoing messages
        self.db.save_message(user_id=user.id, message_id=msg1.id, text=greeting, direction="outgoing")

    async def handle_message(self, event):
        """Handle incoming messages"""
        user = await event.get_sender()
        text = event.message.text

        # Save user
        self.db.save_user(user)

        # Save incoming message
        self.db.save_message(user_id=user.id, message_id=event.message.id, text=text, direction="incoming")

        # Get or create session - ensure conversation continues
        session = self.db.get_session(user.id)
        if not session:
            self.db.create_session(user.id)

        # Get conversation history
        history = self.db.get_conversation_history(user.id)

        # Check if this might be a name response (early in conversation)
        is_name_expected = len(history) <= 2  # Just started, expecting name

        # Build messages for AI
        messages = [{"role": "system", "content": CREDIT_EXPERT_SYSTEM_PROMPT}]

        for msg in history:
            role = "assistant" if msg["direction"] == "outgoing" else "user"
            messages.append({"role": role, "content": msg["message_text"]})

        # Add current message with name validation hint if needed
        current_msg = text
        if is_name_expected and not is_valid_name(text):
            # Add hint for AI that name seems invalid
            current_msg = f"{text}\n[СИСТЕМА: Это не похоже на имя. Переспроси имя клиента вежливо.]"

        messages.append({"role": "user", "content": current_msg})

        # Get AI response
        try:
            response = await self.ai.chat_completion(messages)
            response_text = response.choices[0].message.content
            # Strip emoji from response - professional tone
            response_text = strip_emoji(response_text)
            # Clean up extra whitespace that might appear after emoji removal
            response_text = re.sub(r"\s+", " ", response_text).strip()
            response_text = re.sub(r" ([.,!?])", r"\1", response_text)
        except Exception as e:
            logger.error(f"AI error: {e}")
            response_text = "Извините, произошла техническая ошибка. Напишите ещё раз."

        # Send response
        sent_msg = await event.respond(response_text)

        # Save outgoing message
        self.db.save_message(user_id=user.id, message_id=sent_msg.id, text=response_text, direction="outgoing")

    def stop(self):
        """Stop the bot"""
        self.db.close()
        if self.client:
            self.client.disconnect()


async def main():
    bot = CreditExpertBot()
    try:
        await bot.start()
    except KeyboardInterrupt:
        bot.stop()
    except Exception as e:
        logger.error(f"Bot error: {e}")
        bot.stop()
        raise


if __name__ == "__main__":
    asyncio.run(main())
