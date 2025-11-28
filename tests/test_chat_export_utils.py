"""Unit tests for chat_export_utils helpers."""

import sys
from datetime import datetime
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock

import pytest
from telethon.tl.types import ReactionEmoji

# Add project root to import path
sys.path.insert(0, str(Path(__file__).parent.parent))

from chat_export_utils import (
    MAX_FILENAME_LEN,
    build_message_text,
    collect_reaction_breakdown,
    collect_reaction_emojis,
    collect_reactions_summary,
    format_timestamp,
    resolve_sender_name,
    sanitize_filename,
)


class TestSanitizeFilename:
    """Tests for sanitize_filename."""

    def test_removes_special_characters_and_whitespace(self):
        result = sanitize_filename("My Chat: Special/Name? *")
        assert result == "My_Chat_SpecialName"

    def test_uses_fallback_and_limits_length(self):
        long_name = "a" * (MAX_FILENAME_LEN + 10)
        result = sanitize_filename(long_name, fallback="fallback_name")
        assert result == "a" * MAX_FILENAME_LEN

        empty_result = sanitize_filename(None, fallback="custom")
        assert empty_result == "custom"


class TestFormatTimestamp:
    """Tests for format_timestamp."""

    def test_formats_datetime_or_returns_empty(self):
        dt = datetime(2024, 12, 31, 23, 59)
        assert format_timestamp(dt) == "31.12.2024 23:59"
        assert format_timestamp(None) == ""


class TestBuildMessageText:
    """Tests for build_message_text."""

    def test_appends_media_marker(self):
        message_with_media = SimpleNamespace(text="Hello", media=True)
        assert build_message_text(message_with_media) == "Hello [Media]"

        message_media_only = SimpleNamespace(text="", media=object())
        assert build_message_text(message_media_only) == "[Media]"

        message_without_media = SimpleNamespace(text="Just text", media=None)
        assert build_message_text(message_without_media) == "Just text"


class TestResolveSenderName:
    """Tests for resolve_sender_name."""

    @pytest.mark.asyncio
    async def test_uses_cache_when_available(self):
        cache = {123: "Cached User"}
        message = MagicMock()
        message.sender_id = 123
        message.sender = None
        message.get_sender = AsyncMock()

        result = await resolve_sender_name(message, cache)

        assert result == "Cached User"
        message.get_sender.assert_not_awaited()

    @pytest.mark.asyncio
    async def test_builds_name_from_sender_fields(self):
        cache: dict[int, str] = {}
        sender = SimpleNamespace(first_name="Jane", last_name="Doe", username="jdoe", title="")
        message = MagicMock()
        message.sender_id = 42
        message.sender = sender
        message.get_sender = AsyncMock()

        result = await resolve_sender_name(message, cache)

        assert result == "Jane Doe"
        assert cache[42] == "Jane Doe"
        message.get_sender.assert_not_awaited()

    @pytest.mark.asyncio
    async def test_returns_unknown_when_sender_missing(self):
        cache: dict[int, str] = {}
        message = MagicMock()
        message.sender_id = 777
        message.sender = None
        message.get_sender = AsyncMock(side_effect=Exception("fail"))

        result = await resolve_sender_name(message, cache, unknown="unknown")

        assert result == "unknown"
        assert cache[777] == "unknown"


class TestReactionHelpers:
    """Tests for reaction parsing helpers."""

    def test_collects_reaction_data(self):
        reaction_results = [
            SimpleNamespace(count=2, reaction=ReactionEmoji(emoticon="🔥")),
            SimpleNamespace(count=1, reaction="👍"),
        ]
        message = SimpleNamespace(reactions=SimpleNamespace(results=reaction_results))

        total, emojis = collect_reactions_summary(message)
        breakdown = collect_reaction_breakdown(message)
        emoji_string = collect_reaction_emojis(message)

        assert total == 3
        assert emojis == "🔥👍"
        assert emoji_string == "🔥👍"
        assert breakdown == [
            {"emoji": "🔥", "count": 2},
            {"emoji": "👍", "count": 1},
        ]

    def test_handles_absent_reactions(self):
        message = SimpleNamespace(reactions=None)

        total, emojis = collect_reactions_summary(message)
        breakdown = collect_reaction_breakdown(message)

        assert total == 0
        assert emojis == ""
        assert breakdown == []
