#!/usr/bin/env python3
"""
Проверка всех чатов на наличие задач и важных обсуждений.

Анализирует последние сообщения из всех чатов и ищет:
- Запросы на действия (@stden сделай, надо, нужно)
- Вопросы требующие ответа
- Упоминания задач, багов, проблем
- Важные обсуждения
"""

import asyncio
import os
from pathlib import Path
from collections import defaultdict
from datetime import datetime, timedelta
from typing import List, Dict

from telethon import TelegramClient
from telethon.tl.types import Channel
from dotenv import load_dotenv

load_dotenv()

API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
SESSION_FILE = os.getenv("TELEGRAM_SESSION_FILE", "telegram_session")

# Мой username для поиска упоминаний
MY_USERNAME = os.getenv("MY_NAME", "stden")

# Ключевые слова для поиска задач
TASK_KEYWORDS = [
    "сделай", "нужно", "надо", "можешь", "помоги", "исправь",
    "добавь", "удали", "настрой", "проверь", "запусти",
    "@" + MY_USERNAME,
]

# Ключевые слова для вопросов
QUESTION_KEYWORDS = [
    "?", "как", "почему", "что делать", "где", "когда", "кто"
]

# Ключевые слова для проблем
PROBLEM_KEYWORDS = [
    "не работает", "ошибка", "баг", "сломалось", "проблема",
    "упало", "крашится", "зависло"
]


class ChatTaskChecker:
    """Проверка задач во всех чатах."""

    def __init__(self):
        self.client: TelegramClient = None
        self.tasks_by_chat: Dict[str, List[Dict]] = defaultdict(list)
        self.questions_by_chat: Dict[str, List[Dict]] = defaultdict(list)
        self.problems_by_chat: Dict[str, List[Dict]] = defaultdict(list)

    async def __aenter__(self):
        """Connect to Telegram."""
        self.client = TelegramClient(SESSION_FILE, API_ID, API_HASH)
        await self.client.connect()

        if not await self.client.is_user_authorized():
            raise RuntimeError("Not authorized. Run init_session.py first.")

        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Disconnect from Telegram."""
        if self.client:
            await self.client.disconnect()

    async def check_all_chats(self, days_back: int = 3, limit_per_chat: int = 100):
        """Check all chats for tasks.

        Args:
            days_back: How many days back to check
            limit_per_chat: Maximum messages per chat
        """
        print(f"\n🔍 Проверяю чаты за последние {days_back} дней...")
        print(f"Лимит сообщений на чат: {limit_per_chat}\n")

        offset_date = datetime.now() - timedelta(days=days_back)

        # Получаем все диалоги
        dialogs = await self.client.get_dialogs()

        # Фильтруем только каналы и группы
        relevant_dialogs = [
            d for d in dialogs
            if isinstance(d.entity, Channel) and not d.entity.broadcast
        ]

        print(f"Найдено релевантных чатов: {len(relevant_dialogs)}\n")

        for dialog in relevant_dialogs:
            try:
                chat_name = dialog.title or "Unknown"
                await self.check_chat(dialog.entity, chat_name, offset_date, limit_per_chat)
            except Exception as e:
                print(f"❌ Ошибка при проверке '{dialog.title}': {e}")

    async def check_chat(
        self,
        entity,
        chat_name: str,
        offset_date: datetime,
        limit: int
    ):
        """Check single chat for tasks.

        Args:
            entity: Telegram entity
            chat_name: Chat name
            offset_date: Date to start from
            limit: Message limit
        """
        message_count = 0
        tasks_found = 0
        questions_found = 0
        problems_found = 0

        async for message in self.client.iter_messages(
            entity,
            limit=limit,
            offset_date=offset_date
        ):
            if not message.message:
                continue

            message_count += 1
            text = message.message.lower()

            # Check for tasks
            if self._is_task(text, message.message):
                self.tasks_by_chat[chat_name].append({
                    'text': message.message,
                    'date': message.date,
                    'sender': await self._get_sender_name(message),
                    'id': message.id
                })
                tasks_found += 1

            # Check for questions
            if self._is_question(text):
                self.questions_by_chat[chat_name].append({
                    'text': message.message,
                    'date': message.date,
                    'sender': await self._get_sender_name(message),
                    'id': message.id
                })
                questions_found += 1

            # Check for problems
            if self._is_problem(text):
                self.problems_by_chat[chat_name].append({
                    'text': message.message,
                    'date': message.date,
                    'sender': await self._get_sender_name(message),
                    'id': message.id
                })
                problems_found += 1

        if tasks_found > 0 or questions_found > 0 or problems_found > 0:
            print(f"📌 {chat_name}")
            if tasks_found > 0:
                print(f"   ✅ Задач: {tasks_found}")
            if questions_found > 0:
                print(f"   ❓ Вопросов: {questions_found}")
            if problems_found > 0:
                print(f"   ⚠️  Проблем: {problems_found}")
            print(f"   📊 Проверено сообщений: {message_count}\n")

    def _is_task(self, text_lower: str, original_text: str) -> bool:
        """Check if message contains a task."""
        # Check for mentions
        if f"@{MY_USERNAME.lower()}" in text_lower:
            return True

        # Check for task keywords
        for keyword in TASK_KEYWORDS:
            if keyword.lower() in text_lower and len(original_text) > 20:
                return True

        return False

    def _is_question(self, text: str) -> bool:
        """Check if message is a question."""
        for keyword in QUESTION_KEYWORDS:
            if keyword in text:
                return True
        return False

    def _is_problem(self, text: str) -> bool:
        """Check if message describes a problem."""
        for keyword in PROBLEM_KEYWORDS:
            if keyword in text:
                return True
        return False

    async def _get_sender_name(self, message) -> str:
        """Get sender name from message."""
        if not message.sender:
            return "Unknown"

        sender = message.sender
        if hasattr(sender, 'first_name'):
            return f"{sender.first_name or ''} {sender.last_name or ''}".strip() or "Unknown"
        elif hasattr(sender, 'title'):
            return sender.title
        return "Unknown"

    def generate_report(self, output_file: Path):
        """Generate markdown report.

        Args:
            output_file: Path to save report
        """
        lines = []

        # Header
        lines.append("# 📋 Задачи и обсуждения из всех чатов")
        lines.append("")
        lines.append(f"**Дата проверки:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        lines.append("")

        # Summary
        total_tasks = sum(len(tasks) for tasks in self.tasks_by_chat.values())
        total_questions = sum(len(q) for q in self.questions_by_chat.values())
        total_problems = sum(len(p) for p in self.problems_by_chat.values())

        lines.append("## 📊 Общая статистика")
        lines.append("")
        lines.append(f"- **Чатов проверено:** {len(set(list(self.tasks_by_chat.keys()) + list(self.questions_by_chat.keys()) + list(self.problems_by_chat.keys())))}")
        lines.append(f"- **Задач найдено:** {total_tasks}")
        lines.append(f"- **Вопросов найдено:** {total_questions}")
        lines.append(f"- **Проблем найдено:** {total_problems}")
        lines.append("")

        # Tasks by chat
        if self.tasks_by_chat:
            lines.append("## ✅ Задачи по чатам")
            lines.append("")

            for chat_name, tasks in sorted(self.tasks_by_chat.items(), key=lambda x: len(x[1]), reverse=True):
                lines.append(f"### {chat_name} ({len(tasks)} задач)")
                lines.append("")

                for task in sorted(tasks, key=lambda x: x['date'], reverse=True)[:5]:
                    date_str = task['date'].strftime('%Y-%m-%d %H:%M')
                    lines.append(f"**[{date_str}] {task['sender']}:**")
                    lines.append(f"> {task['text'][:200]}{'...' if len(task['text']) > 200 else ''}")
                    lines.append("")

        # Questions by chat
        if self.questions_by_chat:
            lines.append("## ❓ Вопросы по чатам")
            lines.append("")

            for chat_name, questions in sorted(self.questions_by_chat.items(), key=lambda x: len(x[1]), reverse=True):
                if len(questions) < 3:  # Skip chats with few questions
                    continue

                lines.append(f"### {chat_name} ({len(questions)} вопросов)")
                lines.append("")

                for q in sorted(questions, key=lambda x: x['date'], reverse=True)[:3]:
                    date_str = q['date'].strftime('%Y-%m-%d %H:%M')
                    lines.append(f"**[{date_str}] {q['sender']}:**")
                    lines.append(f"> {q['text'][:200]}{'...' if len(q['text']) > 200 else ''}")
                    lines.append("")

        # Problems by chat
        if self.problems_by_chat:
            lines.append("## ⚠️ Проблемы по чатам")
            lines.append("")

            for chat_name, problems in sorted(self.problems_by_chat.items(), key=lambda x: len(x[1]), reverse=True):
                lines.append(f"### {chat_name} ({len(problems)} проблем)")
                lines.append("")

                for p in sorted(problems, key=lambda x: x['date'], reverse=True)[:5]:
                    date_str = p['date'].strftime('%Y-%m-%d %H:%M')
                    lines.append(f"**[{date_str}] {p['sender']}:**")
                    lines.append(f"> {p['text'][:200]}{'...' if len(p['text']) > 200 else ''}")
                    lines.append("")

        # Save
        output_file.parent.mkdir(parents=True, exist_ok=True)
        output_file.write_text('\n'.join(lines), encoding='utf-8')

        print(f"\n✅ Отчёт сохранён: {output_file}")


async def main():
    """Main function."""
    import argparse

    parser = argparse.ArgumentParser(description="Проверить все чаты на задачи")
    parser.add_argument(
        "--days",
        type=int,
        default=3,
        help="Сколько дней назад проверять (по умолчанию: 3)"
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=100,
        help="Максимум сообщений на чат (по умолчанию: 100)"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("analysis_results/all_chats_tasks.md"),
        help="Путь для сохранения отчёта"
    )

    args = parser.parse_args()

    async with ChatTaskChecker() as checker:
        await checker.check_all_chats(args.days, args.limit)
        checker.generate_report(args.output)

    print("\n✨ Готово!")


if __name__ == "__main__":
    asyncio.run(main())
