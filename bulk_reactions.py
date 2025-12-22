#!/usr/bin/env python3
"""Send the same reaction to many messages in a chat."""

from __future__ import annotations

import argparse
import asyncio
import os
import re
from pathlib import Path
from typing import Iterable

from dotenv import load_dotenv
from telethon.tl.functions.messages import SendReactionRequest
from telethon.tl.types import ReactionEmoji

from chat_export_utils import load_chats_from_config
from telegram_session import SessionLock, get_client

load_dotenv()


def parse_message_token(token: str) -> int | None:
    """Extract message id from a number or t.me link."""
    cleaned = token.strip()
    if not cleaned:
        return None
    if cleaned.isdigit():
        return int(cleaned)

    match = re.search(r"/(\d+)(?:\?.*)?$", cleaned)
    if match:
        return int(match.group(1))
    return None


def collect_message_ids(raw_ids: Iterable[str], file_path: Path | None) -> list[int]:
    """Combine ids from CLI and file, allowing comma/space-separated tokens."""
    tokens: list[str] = []
    for raw in raw_ids:
        tokens.extend(part for part in re.split(r"[,\s]+", raw.strip()) if part)

    if file_path:
        for line in file_path.read_text(encoding="utf-8").splitlines():
            tokens.extend(part for part in re.split(r"[,\s]+", line.strip()) if part)

    ids: list[int] = []
    for token in tokens:
        msg_id = parse_message_token(token)
        if msg_id is not None:
            ids.append(msg_id)
    return ids


def resolve_chat_target(chat_arg: str | None):
    """Resolve chat alias/id from CLI/env into something Telethon can use."""
    chat = chat_arg or os.getenv("BULK_REACTIONS_CHAT") or os.getenv("LIKE_CHAT_ID")
    if not chat or not str(chat).strip():
        raise SystemExit("Set --chat or BULK_REACTIONS_CHAT/LIKE_CHAT_ID env var.")

    chat = str(chat).strip()

    aliases = load_chats_from_config("config.yml", silent_missing=True, skip_invalid=True)
    if chat in aliases:
        return aliases[chat]
    if chat.startswith("@"):
        return chat
    if chat.lstrip("-").isdigit():
        return int(chat)
    return chat


async def fetch_recent_ids(client, entity, limit: int, user_id: int | None) -> list[int]:
    """Collect last N message ids, optionally filtering by sender."""
    messages = await client.get_messages(entity, limit=limit)
    return [m.id for m in messages if not user_id or m.sender_id == user_id]


async def build_previews(client, entity, ids: list[int]) -> dict[int, str]:
    """Fetch messages once to show short previews alongside ids."""
    previews: dict[int, str] = {}
    fetched = await client.get_messages(entity, ids=ids)
    for msg in fetched:
        if not msg:
            continue
        text = (msg.text or msg.raw_text or "").replace("\n", " ").strip()
        label = text[:80] + ("…" if len(text) > 80 else "")
        previews[msg.id] = label or "[no text]"
    return previews


async def main():
    parser = argparse.ArgumentParser(description="Send reactions to many messages.")
    parser.add_argument(
        "--chat",
        help="Chat alias from config.yml, @username or numeric id (fallback: BULK_REACTIONS_CHAT or LIKE_CHAT_ID env).",
    )
    parser.add_argument(
        "--emoji",
        default=os.getenv("BULK_REACTIONS_EMOJI", "🔥"),
        help="Reaction emoji to send (default: 🔥 or BULK_REACTIONS_EMOJI env).",
    )
    parser.add_argument(
        "--ids",
        nargs="*",
        default=[],
        help="Message ids or t.me links (space/comma separated).",
    )
    parser.add_argument(
        "--file",
        type=Path,
        help="Path to file with message ids or t.me links (one per line).",
    )
    parser.add_argument(
        "--recent",
        type=int,
        default=0,
        help="Also react to last N messages from chat (optional).",
    )
    parser.add_argument(
        "--user-id",
        type=int,
        help="When used with --recent, only react to messages from this sender id.",
    )
    parser.add_argument(
        "--delay",
        type=float,
        default=float(os.getenv("BULK_REACTIONS_DELAY", "0.6")),
        help="Delay in seconds between reactions to avoid rate limits (default: 0.6s).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview actions without sending reactions.",
    )

    args = parser.parse_args()

    chat_target = resolve_chat_target(args.chat)
    message_ids = collect_message_ids(args.ids, args.file)
    if not message_ids and args.recent <= 0:
        raise SystemExit("Provide message ids/links or set --recent to react to latest messages.")

    with SessionLock():
        client = get_client()
        async with client:
            entity = await client.get_entity(chat_target)

            if args.recent > 0:
                recent_ids = await fetch_recent_ids(client, entity, args.recent, args.user_id)
                message_ids.extend(recent_ids)

            unique_ids: list[int] = []
            seen = set()
            for msg_id in message_ids:
                if msg_id not in seen:
                    seen.add(msg_id)
                    unique_ids.append(msg_id)

            if not unique_ids:
                print("Nothing to do: no valid message ids resolved.")
                return

            previews = await build_previews(client, entity, unique_ids)

            print(f"Chat: {chat_target}")
            print(f"Emoji: {args.emoji}")
            print(f"Messages to react: {len(unique_ids)}")
            if args.dry_run:
                print("Dry run: no reactions will be sent.")
            print()

            sent = 0
            errors = 0
            for msg_id in unique_ids:
                preview = previews.get(msg_id, "[message not found]")
                print(f"{args.emoji} -> {msg_id}: {preview}")

                if args.dry_run:
                    continue

                try:
                    await client(
                        SendReactionRequest(
                            peer=entity,
                            msg_id=msg_id,
                            reaction=[ReactionEmoji(emoticon=args.emoji)],
                            big=False,
                            add_to_recent=False,
                        )
                    )
                    sent += 1
                    await asyncio.sleep(args.delay)
                except Exception as exc:
                    errors += 1
                    print(f"  Error on {msg_id}: {exc}")

            print()
            print("Done.")
            print(f"Sent: {sent}")
            print(f"Errors: {errors}")


if __name__ == "__main__":
    asyncio.run(main())
