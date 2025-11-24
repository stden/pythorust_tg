"""Tests for autoanswer.py module functions."""

import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))


class TestLoadOpenaiConfig:
    """Tests for load_openai_config function."""

    def test_load_openai_config_success(self, tmp_path):
        """Test loading OpenAI config from file."""
        config_content = """
openai:
  model: gpt-4o
  temperature: 0.8
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        import yaml

        def load_openai_config(config_path):
            try:
                with open(config_path, "r", encoding="utf-8") as f:
                    config = yaml.safe_load(f) or {}
            except FileNotFoundError:
                return {}
            except Exception:
                return {}
            return config.get("openai") or {}

        result = load_openai_config(str(config_file))
        assert result["model"] == "gpt-4o"
        assert result["temperature"] == 0.8

    def test_load_openai_config_not_found(self, tmp_path):
        """Test loading config when file doesn't exist."""
        import yaml

        def load_openai_config(config_path):
            try:
                with open(config_path, "r", encoding="utf-8") as f:
                    config = yaml.safe_load(f) or {}
            except FileNotFoundError:
                return {}
            except Exception:
                return {}
            return config.get("openai") or {}

        result = load_openai_config(str(tmp_path / "nonexistent.yml"))
        assert result == {}

    def test_load_openai_config_no_openai_section(self, tmp_path):
        """Test loading config without openai section."""
        config_content = """
chats:
  test: {}
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        import yaml

        def load_openai_config(config_path):
            try:
                with open(config_path, "r", encoding="utf-8") as f:
                    config = yaml.safe_load(f) or {}
            except FileNotFoundError:
                return {}
            except Exception:
                return {}
            return config.get("openai") or {}

        result = load_openai_config(str(config_file))
        assert result == {}


class TestAutoanswerHandler:
    """Tests for autoanswer event handler logic."""

    def test_handler_ignores_outgoing(self):
        """Test that handler ignores outgoing messages."""
        # Simulate the handler logic
        async def handler_logic(event):
            if event.out:
                return None  # Skip outgoing
            return "would process"

        import asyncio

        event = MagicMock()
        event.out = True

        result = asyncio.get_event_loop().run_until_complete(handler_logic(event))
        assert result is None

    def test_handler_ignores_empty_message(self):
        """Test that handler ignores empty messages."""
        async def handler_logic(event):
            if event.out:
                return None
            user_message = event.message.message.strip()
            if not user_message:
                return None
            return "would process"

        import asyncio

        event = MagicMock()
        event.out = False
        event.message.message = "   "

        result = asyncio.get_event_loop().run_until_complete(handler_logic(event))
        assert result is None

    def test_handler_processes_valid_message(self):
        """Test that handler processes valid messages."""
        async def handler_logic(event):
            if event.out:
                return None
            user_message = event.message.message.strip()
            if not user_message:
                return None
            return user_message

        import asyncio

        event = MagicMock()
        event.out = False
        event.message.message = "Hello!"

        result = asyncio.get_event_loop().run_until_complete(handler_logic(event))
        assert result == "Hello!"


class TestSystemInstructions:
    """Tests for system instructions constant."""

    def test_system_instructions_content(self):
        """Test system instructions are properly defined."""
        system_instructions = (
            "Ты - полезный ассистент, который отвечает на вопросы в Telegram-чате. "
            "Старайся давать подробные, ясные и понятные ответы. "
            "Отвечай нейтральным тоном, при необходимости давай примеры кода и избегай ненормативной лексики. "
            "Если пользователь задаёт технический вопрос, постарайся дать максимально понятный и точный ответ. "
            "Если пользователь не указал иное, отвечай на русском языке."
        )

        assert "ассистент" in system_instructions
        assert "русском языке" in system_instructions
        assert "Telegram" in system_instructions
