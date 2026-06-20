import pytest
import json
from unittest.mock import MagicMock, patch
from chat_analysis.llm_analyzer import LLMAnalyzer, AnalyzerConfig


@pytest.fixture
def config():
    return AnalyzerConfig(llm_provider="openai", model="gpt-3.5-turbo")


@pytest.fixture
def llm_analyzer(config):
    with patch.dict("sys.modules", {"openai": MagicMock()}):
        return LLMAnalyzer(config)


def test_build_prompt(llm_analyzer):
    template = "Analyze this: {{messages}}"
    messages_text = "User: Hello"
    metadata = {"total": 1}
    chat_name = "Test Chat"

    prompt = llm_analyzer._build_prompt(template, messages_text, metadata, chat_name)
    assert template in prompt
    assert "User: Hello" in prompt
    assert "Test Chat" in prompt
    assert '"total": 1' in prompt


def test_parse_response_json(llm_analyzer):
    response = '```json\n{"category": "Tech", "sentiment": "positive"}\n```'
    data = llm_analyzer._parse_response(response)
    assert data["category"] == "Tech"
    assert data["sentiment"] == "positive"


def test_parse_response_invalid_json(llm_analyzer):
    response = "Not a JSON"
    data = llm_analyzer._parse_response(response)
    assert data["category"] == "Unknown"
    assert "Failed to parse" in data["summary"]


def test_create_result(llm_analyzer):
    analysis_data = {
        "category": "Tech",
        "subcategories": ["AI"],
        "sentiment": "positive",
        "activity_level": "high",
        "professionalism": "pro",
        "topics": [{"name": "AI", "mentions": 10, "sentiment": "positive"}],
        "discussions": [
            {"title": "D1", "date": "2023-01-01", "participants": ["A"], "messages_count": 5, "summary": "S"}
        ],
        "summary": "Summary text",
        "insights": ["I1"],
        "recommendations": ["R1"],
    }
    metadata = {
        "total_messages": 100,
        "unique_senders": 5,
        "total_reactions": 50,
        "date_range": {"start": "2023-01-01T00:00:00", "end": "2023-01-02T00:00:00"},
    }

    result = llm_analyzer._create_result(analysis_data, "Test Chat", metadata)

    assert result.chat_name == "Test Chat"
    assert result.category == "Tech"
    assert result.activity_metrics.total_messages == 100
    assert result.activity_metrics.messages_per_day == 50.0  # 100 messages / 2 days
    assert len(result.topics) == 1
    assert result.topics[0].name == "AI"
    assert len(result.discussions) == 1
    assert result.discussions[0].title == "D1"


@pytest.mark.asyncio
async def test_analyze_integration(llm_analyzer):
    messages_text = "Text"
    metadata = {"total_messages": 1}

    mock_result_data = {"category": "Test", "topics": [], "discussions": [], "summary": "S", "insights": []}

    with (
        patch.object(llm_analyzer, "_load_default_prompt", return_value="Template"),
        patch.object(llm_analyzer, "_call_llm", return_value=json.dumps(mock_result_data)),
    ):
        result = llm_analyzer.analyze(messages_text, metadata, "Chat")
        assert result.category == "Test"
        assert result.summary == "S"
