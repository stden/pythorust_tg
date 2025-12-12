"""Data models for chat analysis."""

from dataclasses import dataclass, asdict, field
from typing import List, Optional, Dict, Any
from datetime import datetime
from pathlib import Path
import json


@dataclass
class Topic:
    """Discussion topic."""

    name: str
    mentions: int
    sentiment: str
    key_message_ids: List[int] = field(default_factory=list)


@dataclass
class Discussion:
    """Important discussion."""

    title: str
    date: datetime
    participants: List[str]
    messages_count: int
    summary: str


@dataclass
class ActivityMetrics:
    """Activity metrics for a chat."""

    total_messages: int
    active_users: int
    messages_per_day: float
    avg_message_length: float
    media_percentage: float
    reactions_count: int


@dataclass
class ChatAnalysisResult:
    """Complete chat analysis result."""

    # Core information
    chat_name: str
    analyzed_at: datetime

    # Categorization
    category: str
    subcategories: List[str]

    # Sentiment
    sentiment: str
    activity_level: str
    professionalism: str

    # Topics and discussions
    topics: List[Topic]
    discussions: List[Discussion]

    # Participants
    key_participants: List[Dict[str, Any]]

    # Metrics
    activity_metrics: ActivityMetrics

    # Date range
    date_range_start: Optional[datetime] = None
    date_range_end: Optional[datetime] = None

    # AI-generated content
    summary: str = ""
    insights: List[str] = field(default_factory=list)
    recommendations: List[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        """Convert to dictionary."""
        result = {}

        for key, value in asdict(self).items():
            if isinstance(value, datetime):
                result[key] = value.isoformat()
            elif value is None:
                result[key] = None
            else:
                result[key] = value

        return result

    def to_json(self) -> str:
        """Serialize to JSON."""
        return json.dumps(self.to_dict(), indent=2, ensure_ascii=False)

    def save_json(self, path: Path):
        """Save to JSON file."""
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(self.to_json())

    def save_markdown(self, path: Path):
        """Save to Markdown file with formatted report."""
        from .utils import ensure_dir

        ensure_dir(path)

        lines = []
        lines.extend(self._format_header())
        lines.extend(self._format_categorization())
        lines.extend(self._format_summary())
        lines.extend(self._format_date_range())
        lines.extend(self._format_activity_metrics())
        lines.extend(self._format_topics())
        lines.extend(self._format_discussions())
        lines.extend(self._format_participants())
        lines.extend(self._format_insights())
        lines.extend(self._format_recommendations())

        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))

    def _format_header(self) -> List[str]:
        """Format markdown header section."""
        return [
            "# Chat Analysis Report",
            "",
            f"**Chat:** {self.chat_name}",
            f"**Analyzed:** {self.analyzed_at.strftime('%Y-%m-%d %H:%M:%S')}",
            "",
        ]

    def _format_categorization(self) -> List[str]:
        """Format categorization section."""
        return [
            "## 📂 Categorization",
            "",
            f"- **Category:** {self.category}",
            f"- **Subcategories:** {', '.join(self.subcategories)}",
            f"- **Sentiment:** {self.sentiment}",
            f"- **Activity Level:** {self.activity_level}",
            f"- **Professionalism:** {self.professionalism}",
            "",
        ]

    def _format_summary(self) -> List[str]:
        """Format summary section."""
        return [
            "## 📋 Summary",
            "",
            self.summary,
            "",
        ]

    def _format_date_range(self) -> List[str]:
        """Format date range if available."""
        if self.date_range_start and self.date_range_end:
            return [
                f"**Period:** {self.date_range_start.strftime('%Y-%m-%d')} to {self.date_range_end.strftime('%Y-%m-%d')}",
                "",
            ]
        return []

    def _format_activity_metrics(self) -> List[str]:
        """Format activity metrics section."""
        m = self.activity_metrics
        return [
            "## 📊 Activity Metrics",
            "",
            f"- **Total Messages:** {m.total_messages}",
            f"- **Active Users:** {m.active_users}",
            f"- **Messages/Day:** {m.messages_per_day:.1f}",
            f"- **Avg Message Length:** {m.avg_message_length:.1f} characters",
            f"- **Media Percentage:** {m.media_percentage:.1f}%",
            f"- **Total Reactions:** {m.reactions_count}",
            "",
        ]

    def _format_topics(self) -> List[str]:
        """Format topics section."""
        if not self.topics:
            return []

        lines = ["## 💬 Topics", ""]
        for i, topic in enumerate(self.topics, 1):
            lines.extend(
                [
                    f"### {i}. {topic.name}",
                    "",
                    f"- **Mentions:** {topic.mentions}",
                    f"- **Sentiment:** {topic.sentiment}",
                ]
            )
            if topic.key_message_ids:
                msg_ids = ", ".join(map(str, topic.key_message_ids[:5]))
                lines.append(f"- **Key Messages:** {msg_ids}")
            lines.append("")
        return lines

    def _format_discussions(self) -> List[str]:
        """Format discussions section."""
        if not self.discussions:
            return []

        lines = ["## 🗣️ Key Discussions", ""]
        for i, disc in enumerate(self.discussions, 1):
            lines.extend(
                [
                    f"### {i}. {disc.title}",
                    "",
                    f"- **Date:** {disc.date.strftime('%Y-%m-%d')}",
                    f"- **Participants:** {', '.join(disc.participants[:10])}",
                ]
            )
            if len(disc.participants) > 10:
                lines.append(f"  and {len(disc.participants) - 10} more...")
            lines.extend(
                [
                    f"- **Messages:** {disc.messages_count}",
                    "",
                    disc.summary,
                    "",
                ]
            )
        return lines

    def _format_participants(self) -> List[str]:
        """Format key participants section."""
        if not self.key_participants:
            return []

        lines = ["## 👥 Key Participants", ""]
        for p in self.key_participants[:10]:
            name = p.get("name", "Unknown")
            count = p.get("message_count", 0)
            engagement = p.get("engagement_score", 0)
            lines.append(f"- **{name}** - {count} messages (engagement: {engagement:.1f}/10)")
        lines.append("")
        return lines

    def _format_insights(self) -> List[str]:
        """Format insights section."""
        if not self.insights:
            return []

        lines = ["## 💡 Insights", ""]
        for insight in self.insights:
            lines.append(f"- {insight}")
        lines.append("")
        return lines

    def _format_recommendations(self) -> List[str]:
        """Format recommendations section."""
        if not self.recommendations:
            return []

        lines = ["## 🎯 Recommendations", ""]
        for rec in self.recommendations:
            lines.append(f"- {rec}")
        lines.append("")
        return lines
