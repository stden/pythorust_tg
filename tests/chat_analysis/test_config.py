"""Tests for chat analysis configuration."""

from pathlib import Path

from chat_analysis.config import AnalyzerConfig


def test_default_config():
    """Test default configuration values."""
    config = AnalyzerConfig()

    assert config.message_limit == 1000
    assert config.days_back == 30
    assert config.llm_provider == "openai"
    assert config.temperature == 0.3
    assert config.max_tokens == 2000
    assert config.min_message_length == 10
    assert config.include_media is False
    assert config.exclude_bots is True
    assert config.output_format == "both"
    assert config.verbose is True


def test_custom_config():
    """Test custom configuration values."""
    config = AnalyzerConfig(message_limit=500, days_back=7, llm_provider="claude", temperature=0.5, verbose=False)

    assert config.message_limit == 500
    assert config.days_back == 7
    assert config.llm_provider == "claude"
    assert config.temperature == 0.5
    assert config.verbose is False


def test_default_model_openai():
    """Test default model selection for OpenAI."""
    config = AnalyzerConfig(llm_provider="openai")
    assert config.model == "gpt-4o-mini"


def test_default_model_claude():
    """Test default model selection for Claude."""
    config = AnalyzerConfig(llm_provider="claude")
    assert config.model == "claude-sonnet-4-5-20250929"


def test_default_model_gemini():
    """Test default model selection for Gemini."""
    config = AnalyzerConfig(llm_provider="gemini")
    assert config.model == "gemini-2.0-flash-exp"


def test_custom_model():
    """Test custom model specification."""
    config = AnalyzerConfig(model="gpt-4")
    assert config.model == "gpt-4"


def test_output_dir_creation():
    """Test output directory path handling."""
    config = AnalyzerConfig()
    assert isinstance(config.output_dir, Path)
    assert str(config.output_dir) == "analysis_results"


def test_custom_output_dir():
    """Test custom output directory."""
    config = AnalyzerConfig(output_dir=Path("/tmp/test_output"))
    assert config.output_dir == Path("/tmp/test_output")
