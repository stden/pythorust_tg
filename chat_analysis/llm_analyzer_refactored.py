"""Refactored LLM-based chat analyzer using Strategy Pattern and SRP."""

import json
from typing import Dict, Any, List, Optional
from datetime import datetime

from .config import AnalyzerConfig
from .models import ChatAnalysisResult, Topic, ActivityMetrics, Discussion
from .llm_providers import LLMProviderFactory
from .utils import parse_datetime, load_prompt_template


class ResponseParser:
    """Parses LLM responses and extracts JSON."""

    @staticmethod
    def parse(response: str, verbose: bool = False) -> Dict[str, Any]:
        """Parse LLM response to extract JSON.

        Args:
            response: LLM response text
            verbose: Whether to print debug info

        Returns:
            Parsed JSON data
        """
        # Clean up response
        response = response.strip()

        # Remove markdown code blocks
        if response.startswith("```json"):
            response = response[7:]
        elif response.startswith("```"):
            response = response[3:]

        if response.endswith("```"):
            response = response[:-3]

        response = response.strip()

        # Try to parse JSON
        try:
            return json.loads(response)
        except json.JSONDecodeError as e:
            if verbose:
                print(f"Failed to parse JSON: {e}")
                print(f"Response: {response[:500]}...")

            # Return minimal valid structure
            return ResponseParser._fallback_structure()

    @staticmethod
    def _fallback_structure() -> Dict[str, Any]:
        """Return fallback structure when parsing fails."""
        return {
            "category": "Unknown",
            "subcategories": [],
            "sentiment": "neutral",
            "activity_level": "unknown",
            "professionalism": "unknown",
            "topics": [],
            "discussions": [],
            "key_participants": [],
            "summary": "Failed to parse analysis",
            "insights": [],
            "recommendations": [],
        }


class ResultBuilder:
    """Builds ChatAnalysisResult from parsed data."""

    @staticmethod
    def build(analysis_data: Dict[str, Any], chat_name: str, metadata: Dict[str, Any]) -> ChatAnalysisResult:
        """Create ChatAnalysisResult from parsed data.

        Args:
            analysis_data: Parsed LLM response
            chat_name: Chat name
            metadata: Chat metadata

        Returns:
            Complete ChatAnalysisResult
        """
        # Parse topics
        topics = ResultBuilder._parse_topics(analysis_data.get("topics", []))

        # Parse discussions
        discussions = ResultBuilder._parse_discussions(analysis_data.get("discussions", []))

        # Build activity metrics
        activity_metrics = ResultBuilder._build_activity_metrics(metadata)

        # Parse date range
        date_range_start, date_range_end = ResultBuilder._parse_date_range(metadata)

        # Create result
        return ChatAnalysisResult(
            chat_name=chat_name,
            analyzed_at=datetime.now(),
            category=analysis_data.get("category", "Unknown"),
            subcategories=analysis_data.get("subcategories", []),
            sentiment=analysis_data.get("sentiment", "neutral"),
            activity_level=analysis_data.get("activity_level", "unknown"),
            professionalism=analysis_data.get("professionalism", "unknown"),
            topics=topics,
            discussions=discussions,
            key_participants=analysis_data.get("key_participants", []),
            activity_metrics=activity_metrics,
            date_range_start=date_range_start,
            date_range_end=date_range_end,
            summary=analysis_data.get("summary", ""),
            insights=analysis_data.get("insights", []),
            recommendations=analysis_data.get("recommendations", []),
        )

    @staticmethod
    def _parse_topics(topics_data: List[Dict]) -> List[Topic]:
        """Parse topics from data."""
        topics = []
        for topic_data in topics_data:
            topic = Topic(
                name=topic_data.get("name", "Unknown"),
                mentions=topic_data.get("mentions", 0),
                sentiment=topic_data.get("sentiment", "neutral"),
                key_message_ids=topic_data.get("key_message_ids", []),
            )
            topics.append(topic)
        return topics

    @staticmethod
    def _parse_discussions(discussions_data: List[Dict]) -> List[Discussion]:
        """Parse discussions from data."""
        discussions = []
        for disc_data in discussions_data:
            date = parse_datetime(disc_data.get("date", ""))

            discussion = Discussion(
                title=disc_data.get("title", "Unknown Discussion"),
                date=date,
                participants=disc_data.get("participants", []),
                messages_count=disc_data.get("messages_count", 0),
                summary=disc_data.get("summary", ""),
            )
            discussions.append(discussion)
        return discussions

    @staticmethod
    def _build_activity_metrics(metadata: Dict[str, Any]) -> ActivityMetrics:
        """Build activity metrics from metadata."""
        total_messages = metadata.get("total_messages", 0)
        unique_senders = metadata.get("unique_senders", 0)
        total_reactions = metadata.get("total_reactions", 0)

        # Calculate messages per day
        messages_per_day = 0.0
        date_range = metadata.get("date_range")
        if date_range and total_messages > 0:
            start = parse_datetime(date_range.get("start", ""))
            end = parse_datetime(date_range.get("end", ""))
            days = (end - start).days + 1
            if days > 0:
                messages_per_day = total_messages / days

        return ActivityMetrics(
            total_messages=total_messages,
            active_users=unique_senders,
            messages_per_day=messages_per_day,
            avg_message_length=0.0,  # Would need to calculate from messages
            media_percentage=0.0,  # Would need to calculate from messages
            reactions_count=total_reactions,
        )

    @staticmethod
    def _parse_date_range(metadata: Dict[str, Any]) -> tuple[Optional[datetime], Optional[datetime]]:
        """Parse date range from metadata."""
        date_range = metadata.get("date_range")
        if not date_range:
            return None, None

        start = parse_datetime(date_range.get("start", ""), None)
        end = parse_datetime(date_range.get("end", ""), None)

        return start, end


class PromptBuilder:
    """Builds analysis prompts."""

    @staticmethod
    def build(template: str, messages_text: str, metadata: Dict[str, Any], chat_name: str) -> str:
        """Build complete prompt with messages and metadata.

        Args:
            template: Prompt template
            messages_text: Formatted messages
            metadata: Chat metadata
            chat_name: Chat name

        Returns:
            Complete prompt
        """
        metadata_str = json.dumps(metadata, indent=2, ensure_ascii=False)

        return f"""{template}

## Метаданные чата
{metadata_str}

## Название чата
{chat_name}

## Сообщения
{messages_text}

Предоставь анализ в формате JSON как указано выше."""


class LLMAnalyzer:
    """Analyzes chat messages using LLM APIs."""

    def __init__(self, config: AnalyzerConfig):
        """Initialize analyzer with configuration.

        Args:
            config: Analyzer configuration
        """
        self.config = config
        self.provider = LLMProviderFactory.create(config.llm_provider)
        self.parser = ResponseParser()
        self.result_builder = ResultBuilder()
        self.prompt_builder = PromptBuilder()

    def analyze(
        self, messages_text: str, metadata: Dict[str, Any], chat_name: str, prompt_template: Optional[str] = None
    ) -> ChatAnalysisResult:
        """Analyze chat messages using LLM.

        Args:
            messages_text: Formatted messages for LLM
            metadata: Chat metadata
            chat_name: Name of the chat
            prompt_template: Optional custom prompt template

        Returns:
            ChatAnalysisResult with complete analysis
        """
        # Load prompt template
        if prompt_template is None:
            prompt_template = load_prompt_template("chat_categorizer")

        # Build analysis prompt
        prompt = self.prompt_builder.build(prompt_template, messages_text, metadata, chat_name)

        # Get LLM response
        response = self.provider.call(prompt, self.config)

        # Parse response
        analysis_data = self.parser.parse(response, self.config.verbose)

        # Create result
        result = self.result_builder.build(analysis_data, chat_name, metadata)

        return result
