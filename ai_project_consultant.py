#!/usr/bin/env python3
"""
AI Project Consultant with RAG
ИИ-консультант для проектов с поиском по базе знаний
"""

import asyncio
import os
import sys
from pathlib import Path
from typing import Dict, List, Optional

from dotenv import load_dotenv

# Load environment variables
load_dotenv()

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent))

import logging

from integrations.openai_client import chat_completion

# Configuration from .env
API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
DEFAULT_MODEL = os.getenv("AI_CONSULTANT_MODEL")
DEFAULT_TEMPERATURE = float(os.getenv("AI_CONSULTANT_TEMPERATURE"))
KNOWLEDGE_BASE_PATH = os.getenv("KNOWLEDGE_BASE_PATH")

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class AIProjectConsultant:
    """
    AI consultant with knowledge base search using RAG.
    Мощный ИИ-консультант для помощи по проектам.
    """

    def __init__(
        self,
        model: Optional[str] = None,
        knowledge_base_path: Optional[Path] = None,
        system_prompt: Optional[str] = None,
    ):
        self.model = model or DEFAULT_MODEL
        self.knowledge_base = knowledge_base_path or Path(KNOWLEDGE_BASE_PATH)
        self.conversation_history: List[Dict[str, str]] = []

        # Загружаем системный промпт
        if system_prompt:
            self.system_prompt = system_prompt
        else:
            self.system_prompt = """Ты - опытный технический консультант и архитектор решений.

Твои задачи:
1. Помогать с техническими вопросами по проектам
2. Предлагать архитектурные решения
3. Анализировать проблемы и предлагать исправления
4. Писать код и примеры реализации
5. Объяснять сложные концепции простым языком

Стиль общения:
- Конкретно и по делу
- С примерами кода
- Пошаговые инструкции
- Указываем потенциальные проблемы

Когда предлагаешь решение:
1. Анализируй контекст проекта
2. Проверь в базе знаний похожие случаи
3. Предложи оптимальное решение с обоснованием
4. Дай код/конфигурацию если нужно
5. Укажи на возможные подводные камни

Если информации недостаточно - задавай уточняющие вопросы."""

    async def index_knowledge_base(self):
        """Index markdown files from knowledge base."""
        if not self.knowledge_base.exists():
            logger.warning(f"Knowledge base not found: {self.knowledge_base}")
            return []

        documents = []
        for md_file in self.knowledge_base.rglob("*.md"):
            try:
                content = md_file.read_text(encoding="utf-8")
                documents.append(
                    {
                        "file": str(md_file.relative_to(self.knowledge_base)),
                        "content": content[:2000],  # First 2000 chars
                    }
                )
            except Exception as e:
                logger.error(f"Error reading {md_file}: {e}")

        logger.info(f"Indexed {len(documents)} documents from knowledge base")
        return documents

    async def search_knowledge_base(self, query: str, top_k: int = 3) -> List[str]:
        """
        Simple keyword-based search in knowledge base.
        TODO: Replace with vector search using embeddings.
        """
        documents = await self.index_knowledge_base()

        # Simple keyword matching
        query_lower = query.lower()
        scored_docs = []

        for doc in documents:
            content_lower = doc["content"].lower()
            score = sum(1 for word in query_lower.split() if word in content_lower)
            if score > 0:
                scored_docs.append((score, doc))

        # Sort by score and return top_k
        scored_docs.sort(reverse=True, key=lambda x: x[0])
        results = [doc["content"] for score, doc in scored_docs[:top_k]]

        if results:
            logger.info(f"Found {len(results)} relevant documents")

        return results

    async def consult(self, user_message: str, use_rag: bool = True) -> str:
        """
        Get AI consultation on a question.

        Args:
            user_message: Вопрос пользователя
            use_rag: Использовать ли поиск по базе знаний
        """
        # RAG: поиск релевантной информации
        context_docs = []
        if use_rag:
            context_docs = await self.search_knowledge_base(user_message)

        # Формируем контекст для LLM
        messages = [{"role": "system", "content": self.system_prompt}]

        # Добавляем найденные документы как контекст
        if context_docs:
            context_text = "\n\n---\n\n".join(
                [f"Релевантная информация из базы знаний:\n{doc}" for doc in context_docs]
            )
            messages.append({"role": "system", "content": f"Используй эту информацию для ответа:\n\n{context_text}"})

        # Добавляем историю разговора
        messages.extend(self.conversation_history[-10:])  # Last 10 messages

        # Добавляем текущий вопрос
        messages.append({"role": "user", "content": user_message})

        # Получаем ответ от LLM
        try:
            response = await chat_completion(
                messages=messages,
                model=self.model,
                temperature=0.3,  # Более детерминированные ответы
            )

            # Сохраняем в историю
            self.conversation_history.append({"role": "user", "content": user_message})
            self.conversation_history.append({"role": "assistant", "content": response})

            return response

        except Exception as e:
            logger.error(f"Error getting AI response: {e}")
            return f"Ошибка при получении ответа: {e}"

    async def clear_history(self):
        """Clear conversation history."""
        self.conversation_history = []
        logger.info("Conversation history cleared")


async def interactive_mode():
    """Interactive console mode."""
    print("🤖 ИИ-консультант по проектам запущен")
    print("Команды:")
    print("  /clear - очистить историю")
    print("  /exit - выход")
    print("  /norag - вопрос без поиска в базе знаний")
    print()

    consultant = AIProjectConsultant()

    while True:
        try:
            user_input = input("❓ Ваш вопрос: ").strip()

            if not user_input:
                continue

            if user_input == "/exit":
                print("👋 До свидания!")
                break

            if user_input == "/clear":
                await consultant.clear_history()
                print("✅ История очищена")
                continue

            use_rag = True
            if user_input.startswith("/norag "):
                use_rag = False
                user_input = user_input[7:]

            print("\n🤔 Думаю...\n")
            response = await consultant.consult(user_input, use_rag=use_rag)
            print(f"🤖 Ответ:\n{response}\n")
            print("-" * 80)
            print()

        except KeyboardInterrupt:
            print("\n👋 До свидания!")
            break
        except Exception as e:
            print(f"❌ Ошибка: {e}")


async def telegram_bot_mode():
    """Telegram bot mode."""
    from telethon import TelegramClient, events

    BOT_TOKEN = os.getenv("AI_CONSULTANT_BOT_TOKEN")

    if not BOT_TOKEN:
        print("❌ Установите переменную AI_CONSULTANT_BOT_TOKEN")
        return

    client = TelegramClient("ai_consultant_bot", API_ID, API_HASH)
    AIProjectConsultant()

    # User sessions
    user_sessions: Dict[int, AIProjectConsultant] = {}

    @client.on(events.NewMessage(pattern="/start"))
    async def start_handler(event):
        await event.respond(
            "🤖 **ИИ-консультант по проектам**\n\n"
            "Я помогу с:\n"
            "• Техническими вопросами\n"
            "• Архитектурой решений\n"
            "• Отладкой проблем\n"
            "• Написанием кода\n\n"
            "Просто напишите свой вопрос!\n\n"
            "Команды:\n"
            "/clear - очистить историю\n"
            "/help - помощь"
        )

    @client.on(events.NewMessage(pattern="/clear"))
    async def clear_handler(event):
        user_id = event.sender_id
        if user_id in user_sessions:
            await user_sessions[user_id].clear_history()
        await event.respond("✅ История диалога очищена")

    @client.on(events.NewMessage(pattern="/help"))
    async def help_handler(event):
        await event.respond(
            "📚 **Как пользоваться:**\n\n"
            "1. Просто напишите свой вопрос\n"
            "2. Я найду релевантную информацию в базе знаний\n"
            "3. Дам подробный ответ с примерами\n\n"
            "**Примеры вопросов:**\n"
            "• Как настроить N8N с Caddy?\n"
            "• Почему приложение недоступно извне?\n"
            "• Как интегрировать Telegram бота с Bitrix24?\n"
            "• Напиши скрипт для мониторинга сервиса"
        )

    @client.on(events.NewMessage)
    async def message_handler(event):
        if event.message.text.startswith("/"):
            return  # Skip commands

        user_id = event.sender_id

        # Create session if not exists
        if user_id not in user_sessions:
            user_sessions[user_id] = AIProjectConsultant()

        consultant = user_sessions[user_id]

        # Show typing indicator
        async with client.action(event.chat_id, "typing"):
            response = await consultant.consult(event.message.text)

        await event.respond(response)

    print("🤖 Telegram bot started...")
    await client.start(bot_token=BOT_TOKEN)
    await client.run_until_disconnected()


async def main():
    """Entry point."""
    import argparse

    parser = argparse.ArgumentParser(description="AI Project Consultant")
    parser.add_argument(
        "--mode",
        choices=["interactive", "telegram"],
        default="interactive",
        help="Run mode: interactive console or Telegram bot",
    )

    args = parser.parse_args()

    if args.mode == "interactive":
        await interactive_mode()
    elif args.mode == "telegram":
        await telegram_bot_mode()


if __name__ == "__main__":
    asyncio.run(main())
