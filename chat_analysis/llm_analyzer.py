"""LLM-based chat analyzer using OpenAI, Claude, or Gemini."""

import os
import json
from typing import Dict, Any, List, Optional
from datetime import datetime
from pathlib import Path

from dotenv import load_dotenv

from .config import AnalyzerConfig
from .models import ChatAnalysisResult, Topic, ActivityMetrics, Discussion

load_dotenv()


class LLMAnalyzer:
    """Analyzes chat messages using LLM APIs."""

    def __init__(self, config: AnalyzerConfig):
        """Initialize analyzer with configuration.

        Args:
            config: Analyzer configuration
        """
        self.config = config
        self._setup_client()

    def _setup_client(self):
        """Setup LLM client based on provider."""
        if self.config.llm_provider == "openai":
            import openai
            self.client = openai.OpenAI(api_key=os.getenv("OPENAI_API_KEY"))
        elif self.config.llm_provider == "claude":
            import anthropic
            self.client = anthropic.Anthropic(api_key=os.getenv("ANTHROPIC_API_KEY"))
        elif self.config.llm_provider == "gemini":
            import google.generativeai as genai
            genai.configure(api_key=os.getenv("GOOGLE_API_KEY"))
            self.client = genai.GenerativeModel(self.config.model)
        else:
            raise ValueError(f"Unsupported LLM provider: {self.config.llm_provider}")

    def analyze(
        self,
        messages_text: str,
        metadata: Dict[str, Any],
        chat_name: str,
        prompt_template: Optional[str] = None
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
        # Load prompt
        if prompt_template is None:
            prompt_template = self._load_default_prompt()

        # Build analysis prompt
        prompt = self._build_prompt(prompt_template, messages_text, metadata, chat_name)

        # Get LLM response
        response = self._call_llm(prompt)

        # Parse response
        analysis_data = self._parse_response(response)

        # Create result
        result = self._create_result(analysis_data, chat_name, metadata)

        return result

    def _load_default_prompt(self) -> str:
        """Load default categorization prompt.

        Returns:
            Prompt template string
        """
        # Try PROMPTS_DIR from .env first, then fall back to relative path
        prompts_dir = os.getenv("PROMPTS_DIR", "prompts")
        prompt_path = Path(prompts_dir) / "chat_categorizer.md"

        if prompt_path.exists():
            return prompt_path.read_text(encoding="utf-8")

        # Fallback inline prompt
        return """You are a chat analyzer. Analyze the provided Telegram chat messages and provide a comprehensive analysis.

Your analysis must be in JSON format with the following structure:

{
  "category": "primary category (e.g., IT, Business, Community, Education)",
  "subcategories": ["subcategory1", "subcategory2"],
  "sentiment": "overall sentiment (positive/negative/neutral/mixed)",
  "activity_level": "activity level (high/medium/low)",
  "professionalism": "professionalism level (professional/casual/mixed)",
  "topics": [
    {
      "name": "topic name",
      "mentions": 10,
      "sentiment": "positive/negative/neutral",
      "key_message_ids": [123, 456]
    }
  ],
  "discussions": [
    {
      "title": "discussion title",
      "date": "2025-11-24",
      "participants": ["User1", "User2"],
      "messages_count": 15,
      "summary": "brief summary of discussion"
    }
  ],
  "key_participants": [
    {
      "name": "User Name",
      "message_count": 50,
      "engagement_score": 8.5
    }
  ],
  "summary": "Overall summary of the chat (2-3 sentences)",
  "insights": ["insight 1", "insight 2", "insight 3"],
  "recommendations": ["recommendation 1", "recommendation 2"]
}

Analyze the messages and provide your response in this exact JSON format."""

    def _build_prompt(
        self,
        template: str,
        messages_text: str,
        metadata: Dict[str, Any],
        chat_name: str
    ) -> str:
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

        prompt = f"""{template}

## Chat Metadata
{metadata_str}

## Chat Name
{chat_name}

## Messages
{messages_text}

Provide your analysis in JSON format as specified above."""

        return prompt

    def _call_llm(self, prompt: str) -> str:
        """Call LLM API with prompt.

        Args:
            prompt: Analysis prompt

        Returns:
            LLM response text
        """
        if self.config.llm_provider == "openai":
            response = self.client.chat.completions.create(
                model=self.config.model,
                messages=[
                    {"role": "system", "content": "You are an expert chat analyzer. Always respond in valid JSON format."},
                    {"role": "user", "content": prompt}
                ],
                temperature=self.config.temperature,
                max_tokens=self.config.max_tokens
            )
            return response.choices[0].message.content

        elif self.config.llm_provider == "claude":
            response = self.client.messages.create(
                model=self.config.model,
                max_tokens=self.config.max_tokens,
                temperature=self.config.temperature,
                messages=[
                    {"role": "user", "content": prompt}
                ]
            )
            return response.content[0].text

        elif self.config.llm_provider == "gemini":
            response = self.client.generate_content(
                prompt,
                generation_config={
                    "temperature": self.config.temperature,
                    "max_output_tokens": self.config.max_tokens
                }
            )
            return response.text

        else:
            raise ValueError(f"Unsupported provider: {self.config.llm_provider}")

    def _parse_response(self, response: str) -> Dict[str, Any]:
        """Parse LLM response to extract JSON.

        Args:
            response: LLM response text

        Returns:
            Parsed JSON data
        """
        # Try to find JSON in response
        response = response.strip()

        # Remove markdown code blocks if present
        if response.startswith("```json"):
            response = response[7:]
        elif response.startswith("```"):
            response = response[3:]

        if response.endswith("```"):
            response = response[:-3]

        response = response.strip()

        try:
            data = json.loads(response)
            return data
        except json.JSONDecodeError as e:
            if self.config.verbose:
                print(f"Failed to parse JSON: {e}")
                print(f"Response: {response[:500]}...")

            # Return minimal valid structure
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
                "recommendations": []
            }

    def _create_result(
        self,
        analysis_data: Dict[str, Any],
        chat_name: str,
        metadata: Dict[str, Any]
    ) -> ChatAnalysisResult:
        """Create ChatAnalysisResult from parsed data.

        Args:
            analysis_data: Parsed LLM response
            chat_name: Chat name
            metadata: Chat metadata

        Returns:
            Complete ChatAnalysisResult
        """
        # Parse topics
        topics = []
        for topic_data in analysis_data.get("topics", []):
            topic = Topic(
                name=topic_data.get("name", "Unknown"),
                mentions=topic_data.get("mentions", 0),
                sentiment=topic_data.get("sentiment", "neutral"),
                key_message_ids=topic_data.get("key_message_ids", [])
            )
            topics.append(topic)

        # Parse discussions
        discussions = []
        for disc_data in analysis_data.get("discussions", []):
            date_str = disc_data.get("date", datetime.now().strftime("%Y-%m-%d"))
            try:
                date = datetime.fromisoformat(date_str)
            except ValueError:
                date = datetime.now()

            discussion = Discussion(
                title=disc_data.get("title", "Unknown Discussion"),
                date=date,
                participants=disc_data.get("participants", []),
                messages_count=disc_data.get("messages_count", 0),
                summary=disc_data.get("summary", "")
            )
            discussions.append(discussion)

        # Calculate activity metrics from metadata
        total_messages = metadata.get("total_messages", 0)
        unique_senders = metadata.get("unique_senders", 0)
        total_reactions = metadata.get("total_reactions", 0)

        # Calculate messages per day
        date_range = metadata.get("date_range")
        messages_per_day = 0.0
        if date_range and total_messages > 0:
            try:
                start = datetime.fromisoformat(date_range["start"])
                end = datetime.fromisoformat(date_range["end"])
                days = (end - start).days + 1
                if days > 0:
                    messages_per_day = total_messages / days
            except (ValueError, KeyError):
                pass

        # Parse date range for result
        date_range_start = None
        date_range_end = None
        if date_range:
            try:
                date_range_start = datetime.fromisoformat(date_range["start"])
                date_range_end = datetime.fromisoformat(date_range["end"])
            except (ValueError, KeyError):
                pass

        # Create activity metrics
        activity_metrics = ActivityMetrics(
            total_messages=total_messages,
            active_users=unique_senders,
            messages_per_day=messages_per_day,
            avg_message_length=0.0,  # Would need to calculate from messages
            media_percentage=0.0,  # Would need to calculate from messages
            reactions_count=total_reactions
        )

        # Create result
        result = ChatAnalysisResult(
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
            recommendations=analysis_data.get("recommendations", [])
        )

        return result
