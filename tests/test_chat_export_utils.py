"""Tests for chat_export_utils helpers."""

from __future__ import annotations

from datetime import datetime
from pathlib import Path
from types import SimpleNamespace

import pytest

pytest.importorskip("telethon")

import chat_export_utils
from chat_export_utils import (
    MAX_FILENAME_LEN,
    build_markdown_entry,
    build_message_text,
    collect_reaction_breakdown,
    collect_reaction_emojis,
    collect_reactions_summary,
    export_messages_to_markdown,
    fetch_and_export_messages,
    format_timestamp,
    load_chats_from_config,
    resolve_sender_name,
    sanitize_filename,
    _parse_reactions,
)
from telethon.tl.types import PeerChannel, ReactionEmoji


class DummySender:
    def __init__(self, first_name: str = "", last_name: str = "", username: str = "", title: str = "") -> None:
        self.first_name = first_name
        self.last_name = last_name
        self.username = username
        self.title = title


class DummyReaction:
    def __init__(self, value: str) -> None:
        self.value = value

    def __str__(self) -> str:
        return self.value


class DummyMessage:
    def __init__(
        self,
        *,
        text: str | None = "",
        media: object | None = None,
        sender_id: int | None = None,
        sender: DummySender | None = None,
        date: datetime | None = None,
        reactions: object | None = None,
    ) -> None:
        self.text = text
        self.media = media
        self.sender_id = sender_id
        self.sender = sender
        self.date = date
        self.reactions = reactions
        self.raise_on_get_sender = False
        self.get_sender_calls = 0

    async def get_sender(self) -> DummySender | None:
        self.get_sender_calls += 1
        if self.raise_on_get_sender:
            raise RuntimeError("boom")
        return self.sender


def make_reactions() -> object:
    return SimpleNamespace(
        results=[
            SimpleNamespace(count=2, reaction=ReactionEmoji(emoticon=":)")),
            SimpleNamespace(count=1, reaction=DummyReaction(":(")),
        ]
    )


def test_sanitize_filename_strips_and_truncates() -> None:
    name = "My Chat! @Test"
    assert sanitize_filename(name) == "My_Chat_Test"

    long_name = "a" * (MAX_FILENAME_LEN + 5)
    assert len(sanitize_filename(long_name)) == MAX_FILENAME_LEN


def test_sanitize_filename_fallback() -> None:
    assert sanitize_filename(None) == "unknown_chat"
    assert sanitize_filename("", fallback="fallback") == "fallback"


def test_format_timestamp_variants() -> None:
    dt = datetime(2025, 1, 2, 3, 4, 5)
    assert format_timestamp(dt) == "02.01.2025 03:04"
    assert format_timestamp(None) == ""


@pytest.mark.parametrize(
    ("text", "media", "expected"),
    [
        ("Hello", None, "Hello"),
        ("Hello", object(), "Hello [Media]"),
        ("", object(), "[Media]"),
    ],
)
def test_build_message_text_variants(text: str, media: object | None, expected: str) -> None:
    message = SimpleNamespace(text=text, media=media)
    assert build_message_text(message) == expected


async def test_resolve_sender_name_uses_cache() -> None:
    message = DummyMessage(sender_id=1, sender=None)
    message.raise_on_get_sender = True
    cache = {1: "Cached"}
    assert await resolve_sender_name(message, cache, unknown="Unknown") == "Cached"
    assert message.get_sender_calls == 0


@pytest.mark.parametrize(
    ("sender", "expected"),
    [
        (DummySender(first_name="First", last_name="Last"), "First Last"),
        (DummySender(title="Channel Title"), "Channel Title"),
        (DummySender(username="user123"), "@user123"),
    ],
)
async def test_resolve_sender_name_from_sender(sender: DummySender, expected: str) -> None:
    message = DummyMessage(sender_id=2, sender=sender)
    cache: dict[int, str] = {}
    assert await resolve_sender_name(message, cache, unknown="Unknown") == expected
    assert cache[2] == expected


async def test_resolve_sender_name_handles_get_sender_error() -> None:
    message = DummyMessage(sender_id=3, sender=None)
    message.raise_on_get_sender = True
    cache: dict[int, str] = {}
    assert await resolve_sender_name(message, cache, unknown="Unknown") == "Unknown"
    assert cache[3] == "Unknown"


def test_parse_reactions_empty() -> None:
    message = DummyMessage(reactions=None)
    assert _parse_reactions(message) == (0, "", [])

    message = DummyMessage(reactions=SimpleNamespace(results=[]))
    assert _parse_reactions(message) == (0, "", [])


def test_collect_reaction_helpers() -> None:
    message = DummyMessage(reactions=make_reactions())
    total, emojis = collect_reactions_summary(message)
    assert total == 3
    assert emojis == ":):("
    assert collect_reaction_emojis(message) == ":):("
    assert collect_reaction_breakdown(message) == [
        {"emoji": ":)", "count": 2},
        {"emoji": ":(", "count": 1},
    ]


async def test_build_markdown_entry_skips_empty_text() -> None:
    message = DummyMessage(text="   ", sender=DummySender(first_name="Test"))
    entry = await build_markdown_entry(message, {}, "%Y-%m-%d")
    assert entry is None


async def test_build_markdown_entry_includes_emojis() -> None:
    message = DummyMessage(
        text="Hello",
        sender_id=4,
        sender=DummySender(first_name="Test", last_name="User"),
        date=datetime(2025, 1, 2, 3, 4, 5),
        reactions=make_reactions(),
    )
    entry = await build_markdown_entry(message, {}, "%Y-%m-%d %H:%M")
    assert entry == "**Test User** (2025-01-02 03:04):\nHello :):(\n\n"


async def test_export_messages_to_markdown_writes_file(tmp_path: Path) -> None:
    messages = [
        DummyMessage(
            text="First",
            sender=DummySender(first_name="A"),
            date=datetime(2025, 1, 1, 10, 0, 0),
        ),
        DummyMessage(
            text="Second",
            sender=DummySender(first_name="B"),
            date=datetime(2025, 1, 1, 11, 0, 0),
        ),
    ]
    output_path = tmp_path / "export.md"
    result = await export_messages_to_markdown(
        messages,
        cache={},
        output_path=output_path,
        title="Test Export",
        meta=["Meta Line"],
        timestamp_fmt="%Y-%m-%d %H:%M",
    )
    assert result == str(output_path)
    content = output_path.read_text(encoding="utf-8")
    assert content.startswith("# Test Export")
    assert "Meta Line" in content
    assert "Messages: 2" in content
    assert content.find("Second") < content.find("First")


async def test_fetch_and_export_messages(tmp_path: Path) -> None:
    messages = [
        DummyMessage(
            text="Hello",
            sender=DummySender(first_name="User"),
            date=datetime(2025, 1, 1, 12, 0, 0),
        )
    ]

    class DummyClient:
        def __init__(self, messages: list[DummyMessage]) -> None:
            self.messages = messages
            self.calls: list[tuple[object, int]] = []

        async def get_messages(self, entity: object, limit: int = 0) -> list[DummyMessage]:
            self.calls.append((entity, limit))
            return self.messages

    client = DummyClient(messages)
    path, count = await fetch_and_export_messages(
        client,
        entity="entity",
        cache={},
        filename="test_chat",
        title="Test Chat",
        limit=5,
        output_dir=tmp_path,
        timestamp_fmt="%Y-%m-%d %H:%M",
    )
    assert count == 1
    assert (tmp_path / "test_chat.md").is_file()
    assert client.calls == [("entity", 5)]
    assert path == str(tmp_path / "test_chat.md")


def test_load_chats_from_config_missing_file_silent() -> None:
    assert load_chats_from_config("missing_config.yml", silent_missing=True) == {}


def test_load_chats_from_config_missing_file_prints(capsys: pytest.CaptureFixture[str]) -> None:
    result = load_chats_from_config("missing_config.yml", silent_missing=False)
    assert result == {}
    captured = capsys.readouterr()
    assert "missing_config.yml" in captured.out


def test_load_chats_from_config_skips_invalid(tmp_path: Path) -> None:
    config_content = """
chats:
  valid_channel:
    type: channel
    id: 123456
  missing_id:
    type: channel
  unknown_type:
    type: mystery
    id: 999
"""
    config_file = tmp_path / "config.yml"
    config_file.write_text(config_content, encoding="utf-8")
    result = load_chats_from_config(str(config_file), skip_invalid=True)
    assert list(result.keys()) == ["valid_channel"]
    assert isinstance(result["valid_channel"], PeerChannel)


def test_load_chats_from_config_fallback_path(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    fallback_name = "fallback_config.yml"
    fallback_path = Path(chat_export_utils.__file__).with_name(fallback_name)
    config_content = """
chats:
  fallback_channel:
    type: channel
    id: 123456
"""
    try:
        fallback_path.write_text(config_content, encoding="utf-8")
        monkeypatch.chdir(tmp_path)
        result = load_chats_from_config(fallback_name)
        assert isinstance(result["fallback_channel"], PeerChannel)
    finally:
        fallback_path.unlink(missing_ok=True)
