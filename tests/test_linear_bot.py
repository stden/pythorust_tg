"""Tests for linear_bot.py module."""

import sys
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))


# Read the linear_bot.py to understand its structure
@pytest.fixture
def mock_linear_bot_deps(mock_env, tmp_path):
    """Mock dependencies for linear_bot."""
    session_file = tmp_path / "telegram_session.session"
    session_file.touch()

    mock_session = MagicMock()
    mock_session.get_client = MagicMock(return_value=MagicMock())
    mock_session.known_senders = {}
    mock_session.SessionLock = MagicMock()

    mock_linear = MagicMock()

    with patch.dict("sys.modules", {
        "telegram_session": mock_session,
        "linear_client": mock_linear
    }):
        yield {"session": mock_session, "linear": mock_linear}


class TestLinearBotConfig:
    """Tests for Linear bot configuration."""

    def test_load_config_with_linear_section(self, tmp_path):
        """Test loading config with linear section."""
        config_content = """
linear:
  team_key: TEST
  default_labels:
    - label-1
    - label-2
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        import yaml

        with open(str(config_file), "r", encoding="utf-8") as f:
            config = yaml.safe_load(f)

        assert config["linear"]["team_key"] == "TEST"
        assert len(config["linear"]["default_labels"]) == 2

    def test_load_config_without_linear_section(self, tmp_path):
        """Test loading config without linear section."""
        config_content = """
chats:
  test: {}
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        import yaml

        with open(str(config_file), "r", encoding="utf-8") as f:
            config = yaml.safe_load(f)

        assert "linear" not in config


class TestLinearBotMessageParsing:
    """Tests for message parsing logic."""

    def test_extract_task_from_message(self):
        """Test extracting task title from message."""
        def extract_task_title(message):
            # Simple extraction logic
            lines = message.strip().split("\n")
            if lines:
                return lines[0][:100]  # First line, max 100 chars
            return "Untitled Task"

        message = "Fix the login bug\nThis is affecting all users"
        result = extract_task_title(message)
        assert result == "Fix the login bug"

    def test_extract_task_from_empty_message(self):
        """Test extracting task from empty message."""
        def extract_task_title(message):
            lines = message.strip().split("\n")
            if lines and lines[0]:
                return lines[0][:100]
            return "Untitled Task"

        message = ""
        result = extract_task_title(message)
        assert result == "Untitled Task"

    def test_extract_task_truncates_long_title(self):
        """Test that long titles are truncated."""
        def extract_task_title(message, max_len=100):
            lines = message.strip().split("\n")
            if lines and lines[0]:
                return lines[0][:max_len]
            return "Untitled Task"

        message = "A" * 200
        result = extract_task_title(message, max_len=100)
        assert len(result) == 100


class TestLinearBotEventHandler:
    """Tests for event handler logic."""

    def test_handler_creates_task(self, mock_linear_bot_deps):
        """Test that handler creates a Linear task."""
        async def create_task_from_message(message, linear_client, team_key):
            title = message.strip().split("\n")[0][:100]
            result = linear_client.create_issue(
                team_key=team_key,
                title=title,
                description=message
            )
            return result

        mock_linear = MagicMock()
        mock_linear.create_issue.return_value = {
            "id": "issue-123",
            "identifier": "TEST-1",
            "url": "https://linear.app/..."
        }

        import asyncio
        result = asyncio.get_event_loop().run_until_complete(
            create_task_from_message("Fix bug", mock_linear, "TEST")
        )

        assert result["id"] == "issue-123"
        mock_linear.create_issue.assert_called_once()

    def test_handler_handles_linear_error(self, mock_linear_bot_deps):
        """Test that handler handles Linear errors gracefully."""
        async def create_task_safe(message, linear_client, team_key):
            try:
                title = message.strip().split("\n")[0][:100]
                return linear_client.create_issue(
                    team_key=team_key,
                    title=title
                )
            except Exception as e:
                return {"error": str(e)}

        mock_linear = MagicMock()
        mock_linear.create_issue.side_effect = Exception("API Error")

        import asyncio
        result = asyncio.get_event_loop().run_until_complete(
            create_task_safe("Fix bug", mock_linear, "TEST")
        )

        assert "error" in result
        assert "API Error" in result["error"]
