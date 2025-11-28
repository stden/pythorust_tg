#!/usr/bin/env python3
"""
Анализ сообщений чатов с помощью LightRAG.

Загружает сообщения из MySQL и строит граф знаний для поиска:
- Потребностей людей (что ищут)
- Предложений услуг (что предлагают)
- Связей между людьми и темами
"""

import asyncio
import os

import httpx
import mysql.connector
from lightrag import LightRAG, QueryParam
from lightrag.utils import EmbeddingFunc

# Конфигурация
WORKING_DIR = "./lightrag_telegram"
os.makedirs(WORKING_DIR, exist_ok=True)


async def llm_model_func(
    prompt, system_prompt=None, history_messages=[], keyword_extraction=False, **kwargs
) -> str:
    """Вызов GPT-4o-mini через прямой HTTP (обход проблемы с proxies)."""
    messages = []
    if system_prompt:
        messages.append({"role": "system", "content": system_prompt})
    for msg in history_messages:
        messages.append(msg)
    messages.append({"role": "user", "content": prompt})

    async with httpx.AsyncClient(timeout=60) as client:
        response = await client.post(
            "https://api.openai.com/v1/chat/completions",
            headers={"Authorization": f"Bearer {os.getenv('OPENAI_API_KEY')}"},
            json={
                "model": "gpt-4o-mini",
                "messages": messages,
                "temperature": 0.7,
            }
        )
        data = response.json()
        return data["choices"][0]["message"]["content"]


async def embedding_func(texts: list[str]) -> list[list[float]]:
    """Embedding через прямой HTTP (обход проблемы с proxies на Python 3.14)."""
    api_key = os.getenv('OPENAI_API_KEY')
    if not api_key:
        raise ValueError("OPENAI_API_KEY not set!")

    async with httpx.AsyncClient(timeout=60) as client:
        response = await client.post(
            "https://api.openai.com/v1/embeddings",
            headers={"Authorization": f"Bearer {api_key}"},
            json={
                "model": "text-embedding-3-small",
                "input": texts,
            }
        )
        data = response.json()
        if "error" in data:
            raise ValueError(f"OpenAI API error: {data['error']}")
        if "data" not in data:
            raise ValueError(f"Unexpected API response: {data}")
        return [item["embedding"] for item in data["data"]]


def get_mysql_connection():
    """Подключение к MySQL."""
    return mysql.connector.connect(
        host=os.getenv("MYSQL_HOST", "localhost"),
        port=int(os.getenv("MYSQL_PORT", "3306")),
        database=os.getenv("MYSQL_DATABASE", "pythorust_tg"),
        user=os.getenv("MYSQL_USER"),
        password=os.getenv("MYSQL_PASSWORD"),
        charset="utf8mb4",
    )


def get_messages_for_rag(conn, limit: int = 5000) -> list[str]:
    """Получить сообщения с реакциями для загрузки в RAG."""
    import json
    cursor = conn.cursor(dictionary=True)

    # Берём сообщения с текстом и реакциями
    cursor.execute("""
        SELECT
            tc.title as chat_title,
            tm.sender_name,
            tm.message_text,
            tm.date,
            tm.reactions_count,
            tm.reactions_json,
            tm.views,
            tm.forwards,
            tm.reply_to_msg_id
        FROM telegram_messages tm
        JOIN telegram_chats tc ON tm.chat_id = tc.id
        WHERE tm.message_text IS NOT NULL
        AND LENGTH(tm.message_text) > 20
        AND tm.message_text NOT LIKE 'http%%'
        ORDER BY tm.reactions_count DESC, tm.date DESC
        LIMIT %s
    """, (limit,))

    messages = cursor.fetchall()
    cursor.close()

    # Форматируем сообщения для RAG с реакциями
    docs = []
    for msg in messages:
        reactions_str = ""
        if msg['reactions_json']:
            try:
                reactions = json.loads(msg['reactions_json']) if isinstance(msg['reactions_json'], str) else msg['reactions_json']
                if reactions:
                    reactions_str = f"\nРеакции: {reactions}"
            except (json.JSONDecodeError, TypeError):
                pass

        engagement = ""
        if msg['reactions_count'] and msg['reactions_count'] > 0:
            engagement += f"Реакций: {msg['reactions_count']} "
        if msg['views'] and msg['views'] > 0:
            engagement += f"Просмотров: {msg['views']} "
        if msg['forwards'] and msg['forwards'] > 0:
            engagement += f"Репостов: {msg['forwards']}"

        is_reply = "Да" if msg['reply_to_msg_id'] else "Нет"

        doc = f"""Чат: {msg['chat_title']}
Отправитель: {msg['sender_name'] or 'Аноним'}
Дата: {msg['date']}
Это ответ на другое сообщение: {is_reply}
{f"Вовлечённость: {engagement}" if engagement else ""}
Сообщение: {msg['message_text']}{reactions_str}
---"""
        docs.append(doc)

    return docs


async def main():
    import argparse
    parser = argparse.ArgumentParser(description="Анализ чатов с LightRAG")
    parser.add_argument("--index", action="store_true", help="Индексировать сообщения")
    parser.add_argument("--query", type=str, help="Запрос для поиска")
    parser.add_argument("--mode", type=str, default="hybrid", choices=["naive", "local", "global", "hybrid"], help="Режим поиска")
    parser.add_argument("--limit", type=int, default=3000, help="Лимит сообщений для индексации")
    args = parser.parse_args()

    # Инициализация LightRAG
    rag = LightRAG(
        working_dir=WORKING_DIR,
        llm_model_func=llm_model_func,
        embedding_func=EmbeddingFunc(
            embedding_dim=1536,
            max_token_size=8192,
            func=embedding_func
        ),
    )

    # Инициализация хранилищ (требуется в новых версиях)
    await rag.initialize_storages()
    from lightrag.kg.shared_storage import initialize_pipeline_status
    await initialize_pipeline_status()

    if args.index:
        print("📥 Загружаю сообщения из MySQL...")
        conn = get_mysql_connection()
        docs = get_messages_for_rag(conn, args.limit)
        conn.close()
        print(f"  ✅ Загружено {len(docs)} сообщений")

        print("🔨 Индексирую в LightRAG...")
        # Объединяем в один большой документ (LightRAG разобьёт сам)
        full_text = "\n\n".join(docs)
        await rag.ainsert(full_text)
        print("  ✅ Индексация завершена!")

    if args.query:
        print(f"\n🔍 Поиск: {args.query}")
        print(f"   Режим: {args.mode}")

        result = await rag.aquery(
            args.query,
            param=QueryParam(mode=args.mode)
        )

        print("\n📋 Результат:")
        print("-" * 50)
        print(result)


if __name__ == "__main__":
    from dotenv import load_dotenv
    load_dotenv()

    asyncio.run(main())
