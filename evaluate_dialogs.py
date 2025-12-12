#!/usr/bin/env python3
"""
Evaluate BFL Sales Bot dialogues using OpenAI (acting as ChatGPT 5.1 QA).
"""

import asyncio
import logging
from typing import List, Dict

from dotenv import load_dotenv
from bfl_sales_bot import MySQLLogger
from integrations.openai_client import OpenAIClient

# Load environment
load_dotenv()

# Logging setup
logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s")
logger = logging.getLogger("DialogEvaluator")

EVALUATION_PROMPT = """Ты — ChatGPT 5.1, передовой ИИ для контроля качества работы операторов и ботов.
Твоя задача — проанализировать диалог между Кредитным Экспертом (бот) и Клиентом.

КРИТЕРИИ ОЦЕНКИ (по 10-балльной шкале):
1. Эмпатия (насколько бот был вежлив и поддерживал клиента).
2. Следование скрипту (выявление долга, просрочек, кредиторов).
3. Работа с возражениями (успокоил ли клиента, объяснил ли законность).
4. Закрытие (попытался ли взять номер телефона или записать на консультацию).

ФОРМАТ ОТВЕТА:
Общая оценка: X/10
Плюсы: ...
Минусы: ...
Рекомендации: ...
"""


async def evaluate_session(session_id: int, messages: List[Dict]):
    """Evaluate a single session."""
    if not messages:
        logger.warning(f"Session {session_id} has no messages.")
        return

    transcript = ""
    for msg in messages:
        role = "Бот" if msg["direction"] == "outgoing" else "Клиент"
        transcript += f"{role}: {msg['message_text']}\n"

    logger.info(f"Evaluating session {session_id} with {len(messages)} messages...")

    ai = OpenAIClient()

    prompt = [
        {"role": "system", "content": EVALUATION_PROMPT},
        {"role": "user", "content": f"Вот диалог для анализа:\n\n{transcript}"},
    ]

    try:
        response = await ai.chat_completion(
            messages=prompt,
            model="gpt-4o",  # Using gpt-4o as proxy for "ChatGPT 5.1"
            temperature=0.3,
        )
        evaluation = response.choices[0].message.content

        print(f"\n--- ОТЧЕТ ПО СЕССИИ {session_id} ---\n")
        print(evaluation)
        print("\n-----------------------------------\n")

    except Exception as e:
        logger.error(f"Failed to evaluate session {session_id}: {e}")


async def main():
    db = MySQLLogger()
    db.connect()

    # Fetch recent active sessions (or just all users for simplicity in this demo)
    # For this script, we'll iterate over recent users and their history

    with db.conn.cursor() as cursor:
        cursor.execute("""
            SELECT user_id, MAX(created_at) as last_msg 
            FROM bot_messages 
            GROUP BY user_id 
            ORDER BY last_msg DESC 
            LIMIT 5
        """)
        users = cursor.fetchall()

    for user_row in users:
        user_id = user_row["user_id"]
        history = db.get_conversation_history(user_id, limit=50)
        # History is returned reversed (chronological), which is what we want for transcript

        if history:
            await evaluate_session(user_id, history)

    db.close()


if __name__ == "__main__":
    asyncio.run(main())
