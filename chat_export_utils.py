"""Shared helpers for exporting Telegram chats."""

from __future__ import annotations

import re
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Optional, Sequence

import yaml
from telethon.tl.types import PeerChannel, PeerChat, ReactionEmoji

DEFAULT_TIMESTAMP_FMT = "%d.%m.%Y %H:%M"
DEFAULT_UNKNOWN_SENDER = "Неизвестный отправитель"
DEFAULT_TIMESTAMP_WITH_SECONDS = "%d.%m.%Y %H:%M:%S"
MAX_FILENAME_LEN = 50


def sanitize_filename(name: Optional[str], fallback: str = "unknown_chat") -> str:
    """Convert a title or username to a safe file name."""
    base = re.sub(r"[^\w\s\-]", "", name or "")
    base = re.sub(r"\s+", "_", base.strip())
    return (base or fallback)[:MAX_FILENAME_LEN]


def format_timestamp(dt: Optional[datetime], fmt: str = DEFAULT_TIMESTAMP_FMT) -> str:
    """Return a formatted timestamp or empty string."""
    return dt.strftime(fmt) if dt else ""


async def resolve_sender_name(message, cache: Dict[int, str], unknown: str = DEFAULT_UNKNOWN_SENDER) -> str:
    """
    Return sender name with caching to avoid repeated lookups.

    unknown parameter keeps caller control over fallback wording.
    """
    sender_id = getattr(message, "sender_id", None)
    if sender_id is not None and sender_id in cache:
        return cache[sender_id]

    sender = getattr(message, "sender", None)
    if sender is None:
        try:
            sender = await message.get_sender()
        except Exception:
            sender = None

    name = unknown
    if sender:
        first_name = getattr(sender, "first_name", "") or ""
        last_name = getattr(sender, "last_name", "") or ""
        username = getattr(sender, "username", "") or ""
        title = getattr(sender, "title", "") or ""

        name = f"{first_name} {last_name}".strip() or title or (f"@{username}" if username else unknown)

    if sender_id is not None:
        cache[sender_id] = name
    return name


def collect_reactions_summary(message) -> tuple[int, str]:
    """Return (total_count, emoji_string) for message reactions."""
    total, emojis, _ = _parse_reactions(message)
    return total, emojis


def collect_reaction_breakdown(message) -> list[dict[str, int]]:
    """Return a list of {'emoji': str, 'count': int} for message reactions."""
    _, _, breakdown = _parse_reactions(message)
    return breakdown


def collect_reaction_emojis(message) -> str:
    """Return concatenated reaction emojis for a message."""
    _, emojis, _ = _parse_reactions(message)
    return emojis


def build_message_text(message) -> str:
    """Build text content with media marker if needed."""
    text = message.text or ""
    if getattr(message, "media", None):
        text = f"{text} [Media]" if text else "[Media]"
    return text


async def build_markdown_entry(
    message, cache: Dict[int, str], timestamp_fmt: str = DEFAULT_TIMESTAMP_FMT
) -> Optional[str]:
    """Render a single message as markdown or None when there's nothing to write."""
    text = build_message_text(message)
    if not text.strip():
        return None

    sender_name = await resolve_sender_name(message, cache)
    timestamp = format_timestamp(getattr(message, "date", None), timestamp_fmt)
    emojis = collect_reaction_emojis(message)
    emoji_suffix = f" {emojis}" if emojis else ""
    return f"**{sender_name}** ({timestamp}):\n{text}{emoji_suffix}\n\n"


def _to_entity(cfg: dict[str, Any]) -> Any:
    ctype = cfg.get("type")
    if ctype == "channel":
        return PeerChannel(int(cfg["id"]))
    if ctype == "group":
        return PeerChat(int(cfg["id"]))
    if ctype == "user":
        return int(cfg["id"])
    if ctype == "username":
        username = cfg["username"]
        return username if str(username).startswith("@") else f"@{username}"
    raise ValueError(f"Unknown chat type: {ctype}")


def load_chats_from_config(
    config_path: str = "config.yml", *, silent_missing: bool = False, skip_invalid: bool = False
) -> dict[str, Any]:
    """
    Load chat aliases from config.yml into telethon entities or raw IDs/usernames.
    Falls back to a config next to this module if the given path does not exist.
    Optionally skips invalid entries instead of raising.
    """
    path = Path(config_path)
    if not path.is_file():
        fallback = Path(__file__).with_name(config_path)
        if fallback.is_file():
            path = fallback
        elif silent_missing:
            return {}
        else:
            print(f"{config_path} не найден, возвращаю пустой список чатов")
            return {}

    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    chats_cfg = data.get("chats", {}) or {}

    result: dict[str, Any] = {}
    for name, cfg in chats_cfg.items():
        try:
            result[name] = _to_entity(cfg)
        except (ValueError, KeyError):
            if skip_invalid:
                continue
            raise
    return result


async def fetch_and_export_messages(
    client: Any,
    entity: Any,
    *,
    cache: Dict[int, str],
    filename: str,
    title: str,
    meta: Sequence[str] | None = None,
    limit: int = 200,
    output_dir: str | Path = "chats",
    timestamp_fmt: str = DEFAULT_TIMESTAMP_FMT,
) -> tuple[str, int]:
    """
    Download messages for a Telegram entity and export them to markdown.

    Returns the path to the saved file and the number of fetched messages.
    """
    messages = await client.get_messages(entity, limit=limit)
    output_path = Path(output_dir) / f"{filename}.md"

    path_str = await export_messages_to_markdown(
        messages,
        cache=cache,
        output_path=output_path,
        title=title,
        meta=meta,
        timestamp_fmt=timestamp_fmt,
    )

    return path_str, len(messages)


async def export_messages_to_markdown(
    messages: Sequence[Any],
    *,
    cache: Dict[int, str],
    output_path: str | Path,
    title: str,
    meta: Sequence[str] | None = None,
    timestamp_fmt: str = DEFAULT_TIMESTAMP_FMT,
) -> str:
    """
    Write messages into a markdown file with consistent formatting.

    Args:
        messages: sequence of telethon messages
        cache: sender cache used by resolve_sender_name
        output_path: target markdown path
        title: heading for the export file
        meta: optional lines under the title (e.g., chat id, username)
        timestamp_fmt: formatting string for timestamps

    Returns:
        The output path as a string.
    """
    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)

    with path.open("w", encoding="utf-8") as md:
        md.write(f"# {title}\n")
        for line in meta or []:
            md.write(f"{line}\n")
        md.write(f"Messages: {len(messages)}\n\n---\n\n")

        for message in reversed(messages):
            entry = await build_markdown_entry(message, cache, timestamp_fmt)
            if entry:
                md.write(entry)

    return str(path)


# Backward-compatible aliases
extract_reaction_summary = collect_reactions_summary


def _parse_reactions(message) -> tuple[int, str, list[dict[str, int]]]:
    reactions_attr = getattr(message, "reactions", None)
    if not reactions_attr or not getattr(reactions_attr, "results", None):
        return 0, "", []

    total = 0
    emojis: list[str] = []
    breakdown: list[dict[str, int]] = []

    for result in reactions_attr.results:
        count = getattr(result, "count", 0) or 0
        total += count
        emoji = result.reaction
        emoji_text = emoji.emoticon if isinstance(emoji, ReactionEmoji) else str(emoji)
        emojis.append(emoji_text)
        breakdown.append({"emoji": emoji_text, "count": count})

    return total, "".join(emojis), breakdown
