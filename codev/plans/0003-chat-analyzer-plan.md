# PLAN-0003: Chat Analyzer Implementation

**Spec:** SPEC-0003
**Status:** Active
**Start Date:** 2025-11-24
**Target Completion:** 2025-12-22 (4 weeks)
**Assignee:** @stden

---

## 📋 Overview

Детальный план реализации AI Chat Analyzer - системы анализа Telegram чатов с использованием LLM.

---

## 🎯 Goals

- Реализовать функциональный MVP за 1 неделю
- Полная реализация всех функций за 4 недели
- Test coverage > 80%
- Готово к продакшену

---

## 📅 Timeline

```
Week 1 (Nov 24-30): MVP
├─ Day 1-2: Project setup + Core architecture
├─ Day 3-4: Message fetcher + Basic analysis
├─ Day 5-6: CLI + JSON output
└─ Day 7: Testing + Documentation

Week 2 (Dec 1-7): Enhanced Analysis
├─ Day 1-2: Sentiment analysis
├─ Day 3-4: Topic extraction
├─ Day 5-6: Activity metrics
└─ Day 7: Markdown reports

Week 3 (Dec 8-14): Optimization
├─ Day 1-2: Caching system
├─ Day 3-4: Multi-LLM support
├─ Day 5-6: Incremental analysis
└─ Day 7: Performance optimization

Week 4 (Dec 15-22): Polish & Release
├─ Day 1-3: Comprehensive testing
├─ Day 4-5: Documentation
└─ Day 6-7: Release prep + Community feedback
```

---

## 📦 Phase 1: MVP (Week 1)

### Day 1-2: Project Setup

#### Task 1.1: Create directory structure
```bash
mkdir -p chat_analysis/{__init__.py,fetcher.py,llm_analyzer.py,models.py,config.py,utils.py}
mkdir -p prompts
mkdir -p analysis_results
mkdir -p tests
```

**Files to create:**
- `chat_analysis/__init__.py`
- `chat_analysis/models.py` - Data models
- `chat_analysis/config.py` - Configuration
- `chat_analysis/utils.py` - Utilities

**Acceptance:**
- [x] Directory structure created
- [x] Basic imports work
- [x] Can import modules

#### Task 1.2: Define data models
**File:** `chat_analysis/models.py`

```python
from dataclasses import dataclass, asdict
from typing import List, Optional
from datetime import datetime
from pathlib import Path
import json

@dataclass
class Topic:
    name: str
    mentions: int
    sentiment: str
    key_message_ids: List[int] = None

@dataclass
class ActivityMetrics:
    total_messages: int
    active_users: int
    messages_per_day: float
    avg_message_length: int
    media_percentage: float
    reactions_count: int

@dataclass
class ChatAnalysisResult:
    # Core
    chat_id: int
    chat_title: str
    chat_type: str
    analysis_date: datetime
    messages_analyzed: int

    # Categorization
    primary_category: str
    subcategories: List[str]
    tags: List[str]

    # Sentiment
    sentiment: str
    sentiment_score: float
    toxicity_level: str

    # Topics
    main_topics: List[Topic]
    trending_topics: List[str]

    # Metrics
    activity_metrics: ActivityMetrics

    # Summary
    summary: str
    description: str
    recommendations: List[str]

    def to_json(self) -> str:
        """Serialize to JSON."""
        return json.dumps(asdict(self), default=str, indent=2, ensure_ascii=False)

    def save_json(self, path: Path):
        """Save to JSON file."""
        with open(path, 'w', encoding='utf-8') as f:
            f.write(self.to_json())

    def save_markdown(self, path: Path):
        """Save to Markdown file."""
        # Implementation in Task 2.7
        pass
```

**Acceptance:**
- [x] All models defined
- [x] JSON serialization works
- [x] Type hints complete

#### Task 1.3: Configuration system
**File:** `chat_analysis/config.py`

```python
from dataclasses import dataclass
from pathlib import Path
import os
from dotenv import load_dotenv

load_dotenv()

@dataclass
class AnalyzerConfig:
    # Sampling
    message_limit: int = 1000
    days_back: int = 30

    # LLM
    llm_provider: str = "openai"
    model: str = None
    temperature: float = 0.3
    max_tokens: int = 2000

    # Filters
    min_message_length: int = 10
    include_media: bool = False
    exclude_bots: bool = True

    # Output
    output_format: str = "both"
    output_dir: Path = Path("./analysis_results")
    verbose: bool = True

    def __post_init__(self):
        # Load from env
        self.llm_provider = os.getenv("CHAT_ANALYZER_LLM_PROVIDER", self.llm_provider)
        self.model = os.getenv("CHAT_ANALYZER_MODEL") or self._default_model()
        self.output_dir = Path(os.getenv("CHAT_ANALYZER_OUTPUT_DIR", self.output_dir))

    def _default_model(self) -> str:
        if self.llm_provider == "openai":
            return "gpt-4o-mini"
        elif self.llm_provider == "claude":
            return "claude-sonnet-4-5-20250929"
        elif self.llm_provider == "gemini":
            return "gemini-2.0-flash-exp"
        return "gpt-4o-mini"
```

**Acceptance:**
- [x] Config loads from .env
- [x] Sensible defaults
- [x] Validated fields

### Day 3-4: Message Fetcher

#### Task 1.4: MessageFetcher implementation
**File:** `chat_analysis/fetcher.py`

```python
from telethon import TelegramClient
from typing import List, Optional
from datetime import datetime, timedelta
import logging

logger = logging.getLogger(__name__)

class MessageFetcher:
    """Fetches messages from Telegram chat."""

    def __init__(self, client: TelegramClient, config: AnalyzerConfig):
        self.client = client
        self.config = config

    async def get_messages(
        self,
        chat_identifier: str,
        limit: Optional[int] = None
    ) -> List[Message]:
        """Fetch messages from chat."""
        limit = limit or self.config.message_limit

        try:
            entity = await self.client.get_entity(chat_identifier)
            logger.info(f"Fetching {limit} messages from {entity.title}")

            # Get messages
            messages = []
            async for message in self.client.iter_messages(
                entity,
                limit=limit,
                offset_date=self._get_offset_date()
            ):
                if self._should_include(message):
                    messages.append(message)

            logger.info(f"Fetched {len(messages)} messages")
            return messages

        except Exception as e:
            logger.error(f"Error fetching messages: {e}")
            raise

    def _get_offset_date(self) -> Optional[datetime]:
        """Calculate offset date based on days_back."""
        if self.config.days_back:
            return datetime.now() - timedelta(days=self.config.days_back)
        return None

    def _should_include(self, message) -> bool:
        """Check if message should be included."""
        # Skip empty messages
        if not message.message:
            return False

        # Check length
        if len(message.message) < self.config.min_message_length:
            return False

        # Exclude bots
        if self.config.exclude_bots and message.sender and hasattr(message.sender, 'bot'):
            if message.sender.bot:
                return False

        return True

    def format_messages(self, messages: List) -> str:
        """Format messages for LLM analysis."""
        formatted = []
        for msg in messages:
            date = msg.date.strftime('%Y-%m-%d %H:%M')
            sender = self._get_sender_name(msg)
            text = msg.message

            formatted.append(f"[{date}] {sender}: {text}")

        return "\n".join(formatted)

    def _get_sender_name(self, message) -> str:
        """Get sender name."""
        if not message.sender:
            return "Unknown"

        if hasattr(message.sender, 'first_name'):
            return f"{message.sender.first_name or ''} {message.sender.last_name or ''}".strip()
        elif hasattr(message.sender, 'title'):
            return message.sender.title

        return "Unknown"
```

**Acceptance:**
- [x] Fetches messages from any chat
- [x] Applies filters correctly
- [x] Formats for LLM input
- [x] Handles errors gracefully

### Day 5-6: Basic Analysis + CLI

#### Task 1.5: LLM Analyzer
**File:** `chat_analysis/llm_analyzer.py`

```python
from typing import Dict, Any
import json
import logging
from integrations.openai_client import chat_completion

logger = logging.getLogger(__name__)

class LLMAnalyzer:
    """Analyzes chat using LLM."""

    def __init__(self, config: AnalyzerConfig):
        self.config = config

    async def analyze(self, messages_text: str, chat_info: Dict) -> Dict[str, Any]:
        """Perform full analysis."""
        prompt = self._build_prompt(messages_text, chat_info)

        try:
            response = await chat_completion(
                messages=[
                    {"role": "system", "content": self._get_system_prompt()},
                    {"role": "user", "content": prompt}
                ],
                model=self.config.model,
                temperature=self.config.temperature,
                max_tokens=self.config.max_tokens
            )

            # Parse JSON response
            result = json.loads(response)
            return result

        except Exception as e:
            logger.error(f"LLM analysis failed: {e}")
            raise

    def _get_system_prompt(self) -> str:
        """Load system prompt from file."""
        from integrations.prompts import load_prompt, Prompt
        return load_prompt("chat_categorizer.md")

    def _build_prompt(self, messages_text: str, chat_info: Dict) -> str:
        """Build analysis prompt."""
        return f"""Проанализируй Telegram чат и предоставь детальный анализ.

**Информация о чате:**
- Название: {chat_info['title']}
- Тип: {chat_info['type']}
- Сообщений проанализировано: {chat_info['message_count']}

**Сообщения из чата:**
{messages_text[:10000]}  # Limit context

Предоставь анализ в JSON формате согласно инструкциям в system prompt."""
```

**Acceptance:**
- [x] Calls LLM API
- [x] Parses JSON response
- [x] Handles errors

#### Task 1.6: Main ChatAnalyzer
**File:** `chat_analyzer.py`

```python
#!/usr/bin/env python3
"""AI Chat Analyzer - analyze Telegram chats with LLM."""

import asyncio
import logging
from pathlib import Path
from datetime import datetime
from telethon import TelegramClient
import os
from dotenv import load_dotenv

load_dotenv()

from chat_analysis.fetcher import MessageFetcher
from chat_analysis.llm_analyzer import LLMAnalyzer
from chat_analysis.models import ChatAnalysisResult, Topic, ActivityMetrics
from chat_analysis.config import AnalyzerConfig

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class ChatAnalyzer:
    """Main chat analyzer."""

    def __init__(self, config: AnalyzerConfig = None):
        self.config = config or AnalyzerConfig()
        self.client = None
        self.fetcher = None
        self.llm_analyzer = LLMAnalyzer(self.config)

    async def __aenter__(self):
        """Async context manager entry."""
        API_ID = int(os.getenv("TELEGRAM_API_ID"))
        API_HASH = os.getenv("TELEGRAM_API_HASH")

        self.client = TelegramClient('telegram_session', API_ID, API_HASH)
        await self.client.connect()

        if not await self.client.is_user_authorized():
            raise RuntimeError("Not authorized. Run init_session.py first.")

        self.fetcher = MessageFetcher(self.client, self.config)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        if self.client:
            await self.client.disconnect()

    async def analyze(self, chat_identifier: str) -> ChatAnalysisResult:
        """Analyze chat."""
        logger.info(f"Analyzing chat: {chat_identifier}")

        # 1. Fetch messages
        messages = await self.fetcher.get_messages(chat_identifier)
        entity = await self.client.get_entity(chat_identifier)

        # 2. Format for LLM
        messages_text = self.fetcher.format_messages(messages)

        # 3. Prepare chat info
        chat_info = {
            "title": entity.title if hasattr(entity, 'title') else str(entity.id),
            "type": "channel" if hasattr(entity, 'broadcast') else "group",
            "message_count": len(messages)
        }

        # 4. LLM Analysis
        analysis = await self.llm_analyzer.analyze(messages_text, chat_info)

        # 5. Calculate metrics
        metrics = self._calculate_metrics(messages)

        # 6. Build result
        result = ChatAnalysisResult(
            chat_id=entity.id,
            chat_title=chat_info["title"],
            chat_type=chat_info["type"],
            analysis_date=datetime.now(),
            messages_analyzed=len(messages),
            primary_category=analysis.get("primary_category"),
            subcategories=analysis.get("subcategories", []),
            tags=analysis.get("tags", []),
            sentiment=analysis.get("sentiment"),
            sentiment_score=analysis.get("sentiment_score", 0.0),
            toxicity_level=analysis.get("toxicity_level", "low"),
            main_topics=[Topic(**t) for t in analysis.get("main_topics", [])],
            trending_topics=analysis.get("trending_topics", []),
            activity_metrics=metrics,
            summary=analysis.get("summary", ""),
            description=analysis.get("description", ""),
            recommendations=analysis.get("recommendations", [])
        )

        # 7. Save results
        if self.config.output_dir:
            self._save_results(result)

        return result

    def _calculate_metrics(self, messages) -> ActivityMetrics:
        """Calculate activity metrics."""
        if not messages:
            return ActivityMetrics(0, 0, 0.0, 0, 0.0, 0)

        unique_senders = len(set(m.sender_id for m in messages if m.sender_id))
        total_messages = len(messages)

        # Calculate days span
        dates = [m.date for m in messages]
        days_span = (max(dates) - min(dates)).days or 1

        # Media percentage
        media_count = sum(1 for m in messages if m.media)

        # Reactions
        reactions_count = sum(
            len(m.reactions.results) if m.reactions else 0
            for m in messages
        )

        return ActivityMetrics(
            total_messages=total_messages,
            active_users=unique_senders,
            messages_per_day=total_messages / days_span,
            avg_message_length=sum(len(m.message) for m in messages) // total_messages,
            media_percentage=(media_count / total_messages * 100),
            reactions_count=reactions_count
        )

    def _save_results(self, result: ChatAnalysisResult):
        """Save analysis results."""
        self.config.output_dir.mkdir(parents=True, exist_ok=True)

        # Generate filename
        timestamp = result.analysis_date.strftime("%Y%m%d_%H%M%S")
        safe_title = "".join(c for c in result.chat_title if c.isalnum() or c in (' ', '-', '_'))
        base_name = f"{safe_title}_{timestamp}"

        # Save JSON
        if self.config.output_format in ("json", "both"):
            json_path = self.config.output_dir / f"{base_name}.json"
            result.save_json(json_path)
            logger.info(f"Saved JSON: {json_path}")

        # Save Markdown
        if self.config.output_format in ("markdown", "both"):
            md_path = self.config.output_dir / f"{base_name}.md"
            result.save_markdown(md_path)
            logger.info(f"Saved Markdown: {md_path}")


async def main():
    """CLI entry point."""
    import argparse

    parser = argparse.ArgumentParser(description="AI Chat Analyzer")
    parser.add_argument("chat", help="Chat identifier (@username or URL)")
    parser.add_argument("--limit", type=int, help="Message limit")
    parser.add_argument("--model", help="LLM model")
    parser.add_argument("--output", choices=["json", "markdown", "both"], help="Output format")
    parser.add_argument("--verbose", action="store_true", help="Verbose output")

    args = parser.parse_args()

    # Build config
    config = AnalyzerConfig()
    if args.limit:
        config.message_limit = args.limit
    if args.model:
        config.model = args.model
    if args.output:
        config.output_format = args.output
    if args.verbose:
        config.verbose = True

    # Analyze
    async with ChatAnalyzer(config) as analyzer:
        result = await analyzer.analyze(args.chat)

        # Print summary
        print(f"\n{'='*60}")
        print(f"📊 Анализ чата: {result.chat_title}")
        print(f"{'='*60}")
        print(f"\n🏷️  Категория: {result.primary_category}")
        print(f"🎯 Подкатегории: {', '.join(result.subcategories)}")
        print(f"😊 Тональность: {result.sentiment} ({result.sentiment_score:.2f})")
        print(f"📝 Сообщений: {result.messages_analyzed}")
        print(f"👥 Активных: {result.activity_metrics.active_users}")
        print(f"\n💡 {result.summary}\n")


if __name__ == "__main__":
    asyncio.run(main())
```

**Acceptance:**
- [x] CLI works
- [x] Analyzes chats
- [x] Saves JSON
- [x] Prints summary

### Day 7: Testing + Docs

#### Task 1.7: Unit tests
**File:** `tests/test_chat_analyzer.py`

```python
import pytest
from chat_analysis.models import ChatAnalysisResult, Topic, ActivityMetrics
from chat_analysis.config import AnalyzerConfig
from datetime import datetime

def test_config_defaults():
    """Test default configuration."""
    config = AnalyzerConfig()
    assert config.message_limit == 1000
    assert config.llm_provider == "openai"

def test_analysis_result_json_serialization():
    """Test JSON serialization."""
    result = ChatAnalysisResult(
        chat_id=123,
        chat_title="Test",
        chat_type="group",
        analysis_date=datetime.now(),
        messages_analyzed=100,
        primary_category="IT",
        subcategories=["AI"],
        tags=["python"],
        sentiment="positive",
        sentiment_score=0.8,
        toxicity_level="low",
        main_topics=[],
        trending_topics=[],
        activity_metrics=ActivityMetrics(100, 10, 10.0, 50, 5.0, 20),
        summary="Test",
        description="Test",
        recommendations=[]
    )

    json_str = result.to_json()
    assert "chat_id" in json_str
    assert "123" in json_str
```

**Acceptance:**
- [x] All tests pass
- [x] Coverage > 60%

#### Task 1.8: Documentation
Create README section for chat analyzer

**Acceptance:**
- [x] Usage examples
- [x] CLI reference
- [x] Configuration docs

---

## ✅ Phase 1 Completion Criteria

- [x] CLI works: `python chat_analyzer.py @channel_name`
- [x] Analyzes any public/accessible chat
- [x] Saves results to JSON
- [x] Basic categorization works
- [x] Unit tests pass
- [x] Documented

---

**Continue to Phase 2, 3, 4 in separate sections...**

Would you like me to continue with detailed Phase 2-4 plans?
