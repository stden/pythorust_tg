"""Tests for chat analysis data models."""

import pytest
from datetime import datetime
from pathlib import Path
import json
import tempfile

from chat_analysis.models import Topic, Discussion, ActivityMetrics, ChatAnalysisResult


def test_topic_creation():
    """Test Topic dataclass creation."""
    topic = Topic(name="AI/ML", mentions=25, sentiment="positive", key_message_ids=[123, 456, 789])

    assert topic.name == "AI/ML"
    assert topic.mentions == 25
    assert topic.sentiment == "positive"
    assert len(topic.key_message_ids) == 3


def test_discussion_creation():
    """Test Discussion dataclass creation."""
    discussion = Discussion(
        title="Factory AI Discussion",
        date=datetime(2025, 11, 24),
        participants=["User1", "User2"],
        messages_count=15,
        summary="Discussion about Factory AI and Droids",
    )

    assert discussion.title == "Factory AI Discussion"
    assert discussion.participants == ["User1", "User2"]
    assert discussion.messages_count == 15


def test_activity_metrics_creation():
    """Test ActivityMetrics dataclass creation."""
    metrics = ActivityMetrics(
        total_messages=1000,
        active_users=50,
        messages_per_day=33.3,
        avg_message_length=85.5,
        media_percentage=15.2,
        reactions_count=250,
    )

    assert metrics.total_messages == 1000
    assert metrics.active_users == 50
    assert metrics.messages_per_day == pytest.approx(33.3)


def test_chat_analysis_result_to_dict():
    """Test ChatAnalysisResult to_dict conversion."""
    result = ChatAnalysisResult(
        chat_name="@test_chat",
        analyzed_at=datetime(2025, 11, 24, 12, 0),
        category="IT & Programming",
        subcategories=["AI/ML", "Web Dev"],
        sentiment="positive",
        activity_level="high",
        professionalism="professional",
        topics=[Topic("AI", 10, "positive", [1, 2, 3])],
        discussions=[],
        key_participants=[],
        activity_metrics=ActivityMetrics(
            total_messages=100,
            active_users=10,
            messages_per_day=10.0,
            avg_message_length=50.0,
            media_percentage=5.0,
            reactions_count=20,
        ),
        date_range_start=datetime(2025, 11, 1),
        date_range_end=datetime(2025, 11, 24),
        summary="Test chat",
        insights=["Insight 1"],
        recommendations=["Rec 1"],
    )

    data = result.to_dict()

    assert data["chat_name"] == "@test_chat"
    assert data["category"] == "IT & Programming"
    assert len(data["subcategories"]) == 2
    assert len(data["topics"]) == 1


def test_chat_analysis_result_to_json():
    """Test ChatAnalysisResult JSON serialization."""
    result = ChatAnalysisResult(
        chat_name="@test_chat",
        analyzed_at=datetime(2025, 11, 24, 12, 0),
        category="IT",
        subcategories=[],
        sentiment="neutral",
        activity_level="medium",
        professionalism="casual",
        topics=[],
        discussions=[],
        key_participants=[],
        activity_metrics=ActivityMetrics(100, 10, 10.0, 50.0, 5.0, 20),
        date_range_start=None,
        date_range_end=None,
        summary="Test",
        insights=[],
        recommendations=[],
    )

    json_str = result.to_json()

    # Should be valid JSON
    data = json.loads(json_str)
    assert data["chat_name"] == "@test_chat"
    assert data["category"] == "IT"


def test_chat_analysis_result_save_json():
    """Test saving ChatAnalysisResult to JSON file."""
    result = ChatAnalysisResult(
        chat_name="@test_chat",
        analyzed_at=datetime(2025, 11, 24, 12, 0),
        category="Test",
        subcategories=[],
        sentiment="neutral",
        activity_level="low",
        professionalism="casual",
        topics=[],
        discussions=[],
        key_participants=[],
        activity_metrics=ActivityMetrics(10, 5, 1.0, 20.0, 0.0, 5),
        date_range_start=None,
        date_range_end=None,
        summary="Test summary",
        insights=[],
        recommendations=[],
    )

    with tempfile.TemporaryDirectory() as tmpdir:
        path = Path(tmpdir) / "test.json"
        result.save_json(path)

        assert path.exists()

        # Read and verify
        with open(path) as f:
            data = json.load(f)

        assert data["chat_name"] == "@test_chat"
        assert data["category"] == "Test"


def test_chat_analysis_result_save_markdown():
    """Test saving ChatAnalysisResult to Markdown file."""
    result = ChatAnalysisResult(
        chat_name="@test_chat",
        analyzed_at=datetime(2025, 11, 24, 12, 0),
        category="IT & Programming",
        subcategories=["AI/ML"],
        sentiment="positive",
        activity_level="high",
        professionalism="professional",
        topics=[Topic("AI", 10, "positive", [1, 2, 3])],
        discussions=[],
        key_participants=[{"name": "User1", "message_count": 50, "engagement_score": 8.0}],
        activity_metrics=ActivityMetrics(100, 10, 10.0, 50.0, 5.0, 20),
        date_range_start=datetime(2025, 11, 1),
        date_range_end=datetime(2025, 11, 24),
        summary="High-quality IT community focused on AI/ML",
        insights=["Very active community", "High engagement"],
        recommendations=["Continue current trajectory"],
    )

    with tempfile.TemporaryDirectory() as tmpdir:
        path = Path(tmpdir) / "test.md"
        result.save_markdown(path)

        assert path.exists()

        # Read and verify content
        content = path.read_text()

        assert "@test_chat" in content
        assert "IT & Programming" in content
        assert "# Chat Analysis Report" in content
        assert "Topics" in content
        assert "Activity Metrics" in content
