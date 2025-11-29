#!/usr/bin/env python3
"""
Сбор идей по доработке из всех чатов.

Анализирует последние сообщения из чатов и извлекает:
- Запросы на новые функции
- Проблемы пользователей
- Упоминания инструментов
- Идеи по улучшению
"""

import asyncio
import os
from pathlib import Path
from collections import defaultdict
from datetime import datetime, timedelta
from typing import List, Dict, Set
import re

from telethon import TelegramClient
from dotenv import load_dotenv

load_dotenv()

API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
SESSION_FILE = os.getenv("TELEGRAM_SESSION_FILE", "telegram_session")


# Ключевые слова для поиска идей
FEATURE_KEYWORDS = [
    "надо", "нужно", "хочу", "было бы классно", "предлагаю",
    "идея", "можно добавить", "круто было бы", "не хватает",
    "попросил", "запросил", "автоматизировать", "интеграция",
    "add", "feature", "implement", "need", "want", "idea"
]

PROBLEM_KEYWORDS = [
    "не работает", "сломалось", "баг", "ошибка", "проблема",
    "не могу", "не получается", "почему", "как сделать",
    "broken", "bug", "error", "issue", "problem", "doesn't work"
]

TOOL_KEYWORDS = [
    "cursor", "claude", "gpt", "chatgpt", "copilot", "github",
    "linear", "notion", "slack", "telegram", "n8n", "zapier",
    "api", "webhook", "bot", "automation", "ai", "ml",
    "docker", "kubernetes", "python", "rust", "typescript"
]


class IdeaCollector:
    """Сборщик идей из чатов."""

    def __init__(self):
        self.client: TelegramClient = None
        self.ideas: Dict[str, List[Dict]] = defaultdict(list)
        self.problems: Dict[str, List[Dict]] = defaultdict(list)
        self.tools: Dict[str, Set[str]] = defaultdict(set)

    async def __aenter__(self):
        """Connect to Telegram."""
        self.client = TelegramClient(SESSION_FILE, API_ID, API_HASH)
        await self.client.connect()

        if not await self.client.is_user_authorized():
            raise RuntimeError("Not authorized. Run `cargo run -- init-session` first.")

        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Disconnect from Telegram."""
        if self.client:
            await self.client.disconnect()

    async def collect_from_chat(self, chat_id: str, days_back: int = 7, limit: int = 500):
        """Collect ideas from a single chat.

        Args:
            chat_id: Chat identifier (@username or ID)
            days_back: How many days back to analyze
            limit: Maximum number of messages
        """
        print(f"\n📊 Анализирую чат: {chat_id}")

        try:
            entity = await self.client.get_entity(chat_id)
            chat_name = getattr(entity, 'title', chat_id)
        except Exception as e:
            print(f"❌ Не могу получить чат {chat_id}: {e}")
            return

        offset_date = datetime.now() - timedelta(days=days_back)

        message_count = 0
        async for message in self.client.iter_messages(
            entity,
            limit=limit,
            offset_date=offset_date
        ):
            if not message.message:
                continue

            message_count += 1
            text = message.message.lower()

            # Поиск идей
            for keyword in FEATURE_KEYWORDS:
                if keyword in text and len(message.message) > 20:
                    self.ideas[chat_name].append({
                        'text': message.message,
                        'date': message.date,
                        'sender': await self._get_sender_name(message),
                        'id': message.id
                    })
                    break

            # Поиск проблем
            for keyword in PROBLEM_KEYWORDS:
                if keyword in text and len(message.message) > 20:
                    self.problems[chat_name].append({
                        'text': message.message,
                        'date': message.date,
                        'sender': await self._get_sender_name(message),
                        'id': message.id
                    })
                    break

            # Поиск упоминаний инструментов
            for tool in TOOL_KEYWORDS:
                if re.search(r'\b' + re.escape(tool) + r'\b', text):
                    self.tools[chat_name].add(tool)

        print(f"  ✅ Проанализировано сообщений: {message_count}")
        print(f"  💡 Найдено идей: {len(self.ideas[chat_name])}")
        print(f"  ⚠️  Найдено проблем: {len(self.problems[chat_name])}")
        print(f"  🔧 Упомянуто инструментов: {len(self.tools[chat_name])}")

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
        """Generate markdown report with collected ideas.

        Args:
            output_file: Path to save report
        """
        lines = []

        # Header
        lines.append("# 💡 Идеи по доработке из чатов")
        lines.append("")
        lines.append(f"**Дата анализа:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        lines.append("")

        # Summary
        total_ideas = sum(len(ideas) for ideas in self.ideas.values())
        total_problems = sum(len(probs) for probs in self.problems.values())
        all_tools = set()
        for tools in self.tools.values():
            all_tools.update(tools)

        lines.append("## 📊 Общая статистика")
        lines.append("")
        lines.append(f"- **Чатов проанализировано:** {len(self.ideas)}")
        lines.append(f"- **Идей найдено:** {total_ideas}")
        lines.append(f"- **Проблем найдено:** {total_problems}")
        lines.append(f"- **Уникальных инструментов:** {len(all_tools)}")
        lines.append("")

        # Ideas by chat
        lines.append("## 💡 Идеи по чатам")
        lines.append("")

        for chat_name, ideas in sorted(self.ideas.items()):
            if not ideas:
                continue

            lines.append(f"### {chat_name}")
            lines.append("")
            lines.append(f"Найдено идей: {len(ideas)}")
            lines.append("")

            # Show top 10 ideas
            for idea in sorted(ideas, key=lambda x: x['date'], reverse=True)[:10]:
                date_str = idea['date'].strftime('%Y-%m-%d %H:%M')
                lines.append(f"**[{date_str}] {idea['sender']}:**")
                lines.append(f"> {idea['text'][:200]}{'...' if len(idea['text']) > 200 else ''}")
                lines.append("")

        # Problems by chat
        lines.append("## ⚠️ Проблемы по чатам")
        lines.append("")

        for chat_name, problems in sorted(self.problems.items()):
            if not problems:
                continue

            lines.append(f"### {chat_name}")
            lines.append("")
            lines.append(f"Найдено проблем: {len(problems)}")
            lines.append("")

            # Show top 10 problems
            for problem in sorted(problems, key=lambda x: x['date'], reverse=True)[:10]:
                date_str = problem['date'].strftime('%Y-%m-%d %H:%M')
                lines.append(f"**[{date_str}] {problem['sender']}:**")
                lines.append(f"> {problem['text'][:200]}{'...' if len(problem['text']) > 200 else ''}")
                lines.append("")

        # Tool mentions
        lines.append("## 🔧 Упоминаемые инструменты")
        lines.append("")

        for chat_name, tools in sorted(self.tools.items()):
            if not tools:
                continue

            lines.append(f"### {chat_name}")
            lines.append("")
            lines.append(f"Инструменты: {', '.join(sorted(tools))}")
            lines.append("")

        # Popular tools across all chats
        lines.append("## 🌟 Топ инструментов по популярности")
        lines.append("")

        tool_counts = defaultdict(int)
        for tools in self.tools.values():
            for tool in tools:
                tool_counts[tool] += 1

        for tool, count in sorted(tool_counts.items(), key=lambda x: x[1], reverse=True)[:20]:
            lines.append(f"- **{tool}**: упомянут в {count} чатах")

        lines.append("")

        # Action items
        lines.append("## 🎯 Рекомендации по доработке")
        lines.append("")
        lines.append("На основе анализа чатов, рекомендую:")
        lines.append("")
        lines.append("1. **Приоритетные фичи:** Проанализировать самые частые запросы")
        lines.append("2. **Критичные проблемы:** Исправить упомянутые баги")
        lines.append("3. **Интеграции:** Рассмотреть интеграции с популярными инструментами")
        lines.append("4. **Документация:** Создать гайды по решению частых проблем")
        lines.append("")

        # Save
        output_file.parent.mkdir(parents=True, exist_ok=True)
        output_file.write_text('\n'.join(lines), encoding='utf-8')

        print(f"\n✅ Отчёт сохранён: {output_file}")


async def main():
    """Main function."""
    import argparse

    parser = argparse.ArgumentParser(description="Собрать идеи по доработке из чатов")
    parser.add_argument(
        "--chats",
        nargs="+",
        default=["@vibecod3rs"],
        help="Чаты для анализа (например: @vibecod3rs @channel2)"
    )
    parser.add_argument(
        "--days",
        type=int,
        default=7,
        help="Сколько дней назад анализировать (по умолчанию: 7)"
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=500,
        help="Максимум сообщений на чат (по умолчанию: 500)"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("analysis_results/chat_ideas.md"),
        help="Путь для сохранения отчёта"
    )

    args = parser.parse_args()

    print("🚀 Начинаю сбор идей из чатов...")
    print(f"📅 Период анализа: последние {args.days} дней")
    print(f"📝 Лимит сообщений: {args.limit} на чат")
    print()

    async with IdeaCollector() as collector:
        for chat in args.chats:
            await collector.collect_from_chat(chat, args.days, args.limit)

        collector.generate_report(args.output)

    print("\n✨ Готово!")


if __name__ == "__main__":
    asyncio.run(main())
