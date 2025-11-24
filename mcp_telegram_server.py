"""MCP server exposing Telegram chat management tools.

Requirements:
- A local Telegram session file created via ``init_session.py`` (see README).
- ``config.yml`` with chat aliases (channel/group/user/username) to resolve targets.

Tools:
- list_configured_chats: list chat aliases from config.yml
- fetch_recent_messages: read recent messages with reactions
- send_message: send a text message (optionally replying)
"""

from __future__ import annotations

import asyncio
import fcntl
import os
from contextlib import asynccontextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml
from dotenv import load_dotenv
from mcp.server.fastmcp import Context, FastMCP
from telethon.tl.types import PeerChannel, PeerChat, ReactionEmoji

from telegram_session import LOCK_FILE, get_client, known_senders

load_dotenv()

CONFIG_PATH = Path(__file__).with_name("config.yml")
DEFAULT_LIMIT = int(os.getenv("MCP_TELEGRAM_LIMIT", "50"))
MAX_LIMIT = int(os.getenv("MCP_TELEGRAM_MAX_LIMIT", "200"))
LOCK_RETRY_SECONDS = float(os.getenv("MCP_TELEGRAM_LOCK_RETRY", "0.25"))


class AsyncSessionLock:
    """Async-friendly lock that reuses the Telethon session lock file."""

    def __init__(self, path: str, retry_seconds: float = 0.25):
        self.path = path
        self.retry_seconds = retry_seconds
        self.handle = None

    async def __aenter__(self):
        self.handle = open(self.path, "w")
        while True:
            try:
                fcntl.flock(self.handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                return self
            except BlockingIOError:
                await asyncio.sleep(self.retry_seconds)

    async def __aexit__(self, exc_type, exc, tb):
        if self.handle:
            fcntl.flock(self.handle.fileno(), fcntl.LOCK_UN)
            self.handle.close()
            try:
                os.remove(self.path)
            except OSError:
                pass


@dataclass
class ChatTarget:
    name: str
    target: Any
    kind: str
    title: str | None
    raw: dict[str, Any]

    def summary(self) -> dict[str, Any]:
        summary = {"name": self.name, "type": self.kind}
        if self.title:
            summary["title"] = self.title
        if "id" in self.raw:
            summary["id"] = self.raw["id"]
        if "username" in self.raw:
            summary["username"] = self.raw["username"]
        return summary


def _load_chat_targets() -> dict[str, ChatTarget]:
    if not CONFIG_PATH.exists():
        return {}

    data = yaml.safe_load(CONFIG_PATH.read_text()) or {}
    chats = data.get("chats") or {}
    targets: dict[str, ChatTarget] = {}

    for name, cfg in chats.items():
        if not isinstance(cfg, dict):
            continue
        chat_type = cfg.get("type")
        try:
            if chat_type == "channel":
                target = PeerChannel(cfg["id"])
            elif chat_type == "group":
                target = PeerChat(cfg["id"])
            elif chat_type == "username":
                username = cfg["username"]
                target = f"@{username}"
            elif chat_type == "user":
                target = int(cfg["id"])
            else:
                continue
        except KeyError:
            # Skip entries missing required keys
            continue

        targets[name] = ChatTarget(
            name=name,
            target=target,
            kind=chat_type or "unknown",
            title=cfg.get("title"),
            raw=cfg,
        )

    return targets


def _resolve_chat(chat: str) -> ChatTarget:
    chat = chat.strip()
    if not chat:
        raise ValueError("Chat name or identifier is required.")

    configured = _load_chat_targets()
    if chat in configured:
        return configured[chat]

    if chat.startswith("@"):
        username = chat[1:]
        return ChatTarget(
            name=chat,
            target=chat,
            kind="username",
            title=None,
            raw={"username": username},
        )

    if chat.isdigit():
        user_id = int(chat)
        return ChatTarget(
            name=chat,
            target=user_id,
            kind="user",
            title=None,
            raw={"id": user_id},
        )

    available = ", ".join(sorted(configured.keys())) or "no chats configured"
    raise ValueError(f"Chat '{chat}' not found in config.yml. Available: {available}")


def _extract_reactions(message) -> list[dict[str, Any]]:
    reactions = getattr(message, "reactions", None)
    if not reactions or not reactions.results:
        return []

    extracted = []
    for reaction in reactions.results:
        emoji = reaction.reaction.emoticon if isinstance(reaction.reaction, ReactionEmoji) else str(reaction.reaction)
        extracted.append({"emoji": emoji, "count": reaction.count})
    return extracted


async def _sender_name(message, sender_cache: dict[int, str], client) -> str:
    if message.sender_id in sender_cache:
        return sender_cache[message.sender_id]

    try:
        sender = await message.get_sender()
    except Exception:
        sender = None

    if sender:
        first = getattr(sender, "first_name", "") or ""
        last = getattr(sender, "last_name", "") or ""
        username = getattr(sender, "username", None)
        display = " ".join(part for part in (first, last) if part).strip()
        if not display and username:
            display = f"@{username}"
    else:
        display = "Unknown sender"

    if message.sender_id:
        sender_cache[message.sender_id] = display
    return display


@asynccontextmanager
async def telegram_client():
    async with AsyncSessionLock(LOCK_FILE, retry_seconds=LOCK_RETRY_SECONDS):
        try:
            client = get_client()
        except SystemExit as exc:
            raise RuntimeError("Telegram session is missing. Run init_session.py locally to create it.") from exc

        async with client:
            if not await client.is_user_authorized():
                raise RuntimeError("Telegram session is not authorized. Refresh the session before retrying.")
            yield client


async def _fetch_messages(chat: ChatTarget, limit: int) -> list[dict[str, Any]]:
    async with telegram_client() as client:
        entity = await client.get_entity(chat.target)
        messages = await client.get_messages(entity, limit=limit)
        sender_cache: dict[int, str] = known_senders.copy()

        result = []
        for message in messages:
            sender = await _sender_name(message, sender_cache, client)
            result.append(
                {
                    "id": message.id,
                    "sender_id": message.sender_id,
                    "sender": sender,
                    "date": message.date.isoformat(),
                    "text": message.message or message.raw_text or "",
                    "reply_to": message.reply_to_msg_id,
                    "views": message.views,
                    "forwards": message.forwards,
                    "reactions": _extract_reactions(message),
                    "has_media": bool(message.media),
                }
            )
        return result


server = FastMCP(
    name="telegram-chat-mcp",
    instructions="Tools for reading and responding to Telegram chats using the existing Telethon session.",
    dependencies=["config.yml", ".env"],
)


@server.resource("resource://telegram/chats")
def configured_chats_resource() -> str:
    """Expose configured chats as a resource for quick inspection."""
    chats = [target.summary() for target in _load_chat_targets().values()]
    payload = {"configured": chats, "source": str(CONFIG_PATH)}
    return yaml.safe_dump(payload, sort_keys=False)


@server.tool(
    description="List chats configured in config.yml (channel/group/user/username). "
    "Returns id/username metadata but no network requests.",
)
def list_configured_chats() -> list[dict[str, Any]]:
    return [target.summary() for target in _load_chat_targets().values()]


@server.tool(
    description="Fetch recent messages from a configured chat. "
    "Limit defaults to MCP_TELEGRAM_LIMIT (50) and caps at MCP_TELEGRAM_MAX_LIMIT.",
)
async def fetch_recent_messages(chat: str, limit: int = DEFAULT_LIMIT, ctx: Context | None = None) -> dict[str, Any]:
    limit = max(1, min(limit, MAX_LIMIT))
    chat_target = _resolve_chat(chat)

    if ctx:
        await ctx.report_progress(0, limit, message=f"Reading last {limit} messages from {chat_target.name}")

    messages = await _fetch_messages(chat_target, limit)

    if ctx:
        await ctx.report_progress(limit, limit, message="Messages loaded")

    return {"chat": chat_target.summary(), "limit": limit, "messages": messages}


@server.tool(
    description="Send a text message to a configured chat. "
    "Supports reply_to message id and silent flag.",
)
async def send_message(chat: str, text: str, reply_to: int | None = None, silent: bool = False) -> dict[str, Any]:
    if not text or not text.strip():
        raise ValueError("Text message cannot be empty.")

    chat_target = _resolve_chat(chat)
    async with telegram_client() as client:
        entity = await client.get_entity(chat_target.target)
        sent = await client.send_message(entity, text.strip(), reply_to=reply_to, silent=silent)

    return {
        "chat": chat_target.summary(),
        "message_id": sent.id,
        "date": sent.date.isoformat(),
        "reply_to": reply_to,
        "silent": silent,
    }


if __name__ == "__main__":
    server.run()
