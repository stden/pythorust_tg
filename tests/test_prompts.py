"""Tests for integrations/prompts.py module."""

import sys
from pathlib import Path

import pytest

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))


class TestPromptEnum:
    """Tests for Prompt enum."""

    def test_prompt_enum_values(self):
        """Test that all prompt enum values are correct."""
        from integrations.prompts import Prompt

        assert Prompt.SALES_AGENT.value == "sales_agent.md"
        assert Prompt.CALCULATOR.value == "calculator.md"
        assert Prompt.FRIENDLY_AI.value == "friendly_ai.md"
        assert Prompt.MODERATOR.value == "moderator.md"
        assert Prompt.DIGEST.value == "digest.md"
        assert Prompt.CRM_PARSER.value == "crm_parser.md"

    def test_prompt_enum_members(self):
        """Test that Prompt enum has expected members."""
        from integrations.prompts import Prompt

        members = list(Prompt)
        assert len(members) == 6
        assert Prompt.SALES_AGENT in members


class TestGetPromptsDir:
    """Tests for get_prompts_dir function."""

    def test_get_prompts_dir_with_existing_dir(self, temp_prompts_dir, monkeypatch):
        """Test getting prompts directory when it exists."""
        monkeypatch.chdir(temp_prompts_dir.parent)

        from integrations.prompts import get_prompts_dir

        result = get_prompts_dir()
        assert result.exists()
        assert result.name == "prompts"

    def test_get_prompts_dir_fallback(self, tmp_path, monkeypatch):
        """Test fallback when prompts directory doesn't exist in cwd."""
        monkeypatch.chdir(tmp_path)

        from integrations.prompts import get_prompts_dir

        result = get_prompts_dir()
        # The function may find prompts in parent path or return default
        assert result.name == "prompts"

    def test_get_prompts_dir_returns_default_when_no_candidates_exist(self, monkeypatch):
        """Test returning Path('prompts') when no candidate directories exist."""
        monkeypatch.setattr(Path, "exists", lambda self: False)

        from integrations.prompts import get_prompts_dir

        assert get_prompts_dir() == Path("prompts")


class TestLoadPrompt:
    """Tests for load_prompt function."""

    def test_load_prompt_with_enum(self, temp_prompts_dir, monkeypatch):
        """Test loading prompt using Prompt enum."""
        monkeypatch.chdir(temp_prompts_dir.parent)

        from integrations.prompts import Prompt, load_prompt

        content = load_prompt(Prompt.SALES_AGENT)
        assert "sales agent" in content.lower()
        assert "SPIN" in content

    def test_load_prompt_with_string(self, temp_prompts_dir, monkeypatch):
        """Test loading prompt using filename string."""
        monkeypatch.chdir(temp_prompts_dir.parent)

        from integrations.prompts import load_prompt

        content = load_prompt("calculator.md")
        assert "calculator" in content.lower()

    def test_load_prompt_with_context(self, temp_prompts_dir, monkeypatch):
        """Test loading prompt with additional context."""
        monkeypatch.chdir(temp_prompts_dir.parent)

        from integrations.prompts import Prompt, load_prompt

        context = "Selling Python courses"
        content = load_prompt(Prompt.SALES_AGENT, context=context)
        assert "Контекст: Selling Python courses" in content

    def test_load_prompt_file_not_found(self, tmp_path, monkeypatch):
        """Test FileNotFoundError when prompt file doesn't exist."""
        monkeypatch.chdir(tmp_path)
        prompts_dir = tmp_path / "prompts"
        prompts_dir.mkdir()

        from integrations.prompts import load_prompt

        with pytest.raises(FileNotFoundError):
            load_prompt("nonexistent.md")


class TestListPrompts:
    """Tests for list_prompts function."""

    def test_list_prompts(self):
        """Test listing all available prompts."""
        from integrations.prompts import Prompt, list_prompts

        prompts = list_prompts()
        assert len(prompts) == 6
        assert all(isinstance(p, Prompt) for p in prompts)


class TestQuickAccessConstants:
    """Tests for quick access constants."""

    def test_quick_access_constants(self):
        """Test that quick access constants are defined."""
        from integrations.prompts import (
            CALCULATOR,
            CRM_PARSER,
            DIGEST,
            FRIENDLY_AI,
            MODERATOR,
            SALES_AGENT,
            Prompt,
        )

        assert SALES_AGENT == Prompt.SALES_AGENT
        assert CALCULATOR == Prompt.CALCULATOR
        assert FRIENDLY_AI == Prompt.FRIENDLY_AI
        assert MODERATOR == Prompt.MODERATOR
        assert DIGEST == Prompt.DIGEST
        assert CRM_PARSER == Prompt.CRM_PARSER
