# -*- coding: utf-8 -*-
"""
Kurigram client integration - Modern Telegram MTProto API framework.
Alternative to Telethon with similar API.

Kurigram is a fork of Pyrogram: https://docs.kurigram.live/
"""

import os
from typing import Optional, AsyncIterator, List
from dotenv import load_dotenv

load_dotenv()

# Kurigram imports
try:
    from kurigram import Client, filters
    from kurigram.types import Message, Chat, User
    from kurigram.errors import SessionPasswordNeeded
    KURIGRAM_AVAILABLE = True
except ImportError:
    KURIGRAM_AVAILABLE = False
    Client = None
    filters = None
    Message = None
    Chat = None
    User = None
    SessionPasswordNeeded = None


class KurigramClient:
    """Wrapper for Kurigram Telegram client."""

    def __init__(
        self,
        session_name: str = "kurigram_session",
        api_id: Optional[int] = None,
        api_hash: Optional[str] = None,
    ):
        if not KURIGRAM_AVAILABLE:
            raise ImportError("Kurigram not installed. Run: pip install kurigram")

        self.api_id = api_id or int(os.getenv("TELEGRAM_API_ID", "0"))
        self.api_hash = api_hash or os.getenv("TELEGRAM_API_HASH", "")
        self.session_name = session_name

        self.client = Client(
            name=session_name,
            api_id=self.api_id,
            api_hash=self.api_hash,
        )

    async def start(self) -> "KurigramClient":
        """Start the client and connect to Telegram."""
        await self.client.start()
        return self

    async def stop(self) -> None:
        """Stop the client."""
        await self.client.stop()

    async def __aenter__(self) -> "KurigramClient":
        return await self.start()

    async def __aexit__(self, *args) -> None:
        await self.stop()

    async def get_me(self) -> User:
        """Get current user info."""
        return await self.client.get_me()

    async def get_dialogs(self, limit: int = 100) -> List[Chat]:
        """Get list of chats/dialogs."""
        dialogs = []
        async for dialog in self.client.get_dialogs(limit=limit):
            dialogs.append(dialog.chat)
        return dialogs

    async def get_chat(self, chat_id: int | str) -> Chat:
        """Get chat by ID or username."""
        return await self.client.get_chat(chat_id)

    async def iter_messages(
        self,
        chat_id: int | str,
        limit: int = 100,
        offset_id: int = 0,
    ) -> AsyncIterator[Message]:
        """Iterate over messages in a chat."""
        async for msg in self.client.get_chat_history(
            chat_id,
            limit=limit,
            offset_id=offset_id,
        ):
            yield msg

    async def send_message(
        self,
        chat_id: int | str,
        text: str,
        reply_to_message_id: Optional[int] = None,
    ) -> Message:
        """Send a text message."""
        return await self.client.send_message(
            chat_id,
            text,
            reply_to_message_id=reply_to_message_id,
        )

    async def delete_messages(
        self,
        chat_id: int | str,
        message_ids: List[int],
    ) -> int:
        """Delete messages. Returns count of deleted messages."""
        return await self.client.delete_messages(chat_id, message_ids)

    async def export_chat_to_markdown(
        self,
        chat_id: int | str,
        limit: int = 1000,
        output_file: Optional[str] = None,
    ) -> str:
        """Export chat messages to markdown format."""
        chat = await self.get_chat(chat_id)
        chat_name = getattr(chat, 'title', None) or getattr(chat, 'first_name', 'chat')

        lines = [f"# {chat_name}\n"]

        messages = []
        async for msg in self.iter_messages(chat_id, limit=limit):
            messages.append(msg)

        # Reverse to get chronological order
        for msg in reversed(messages):
            if not msg.text:
                continue

            # Get sender name
            sender_name = "Unknown"
            if msg.from_user:
                sender_name = msg.from_user.first_name or msg.from_user.username or str(msg.from_user.id)

            # Format timestamp
            timestamp = msg.date.strftime("%d.%m.%Y %H:%M:%S")

            # Format message
            text = msg.text.replace("\n", "\n> ")
            lines.append(f"**{timestamp}** {sender_name}:\n> {text}\n")

        content = "\n".join(lines)

        if output_file:
            with open(output_file, "w", encoding="utf-8") as f:
                f.write(content)

        return content

    def on_message(self, filters_=None):
        """Decorator for message handlers (for bots/userbots)."""
        return self.client.on_message(filters_ or filters.all)


async def main():
    """Example usage."""
    async with KurigramClient() as client:
        me = await client.get_me()
        print(f"Logged in as: {me.first_name} (@{me.username})")

        # List dialogs
        dialogs = await client.get_dialogs(limit=10)
        print(f"\nTop {len(dialogs)} chats:")
        for chat in dialogs:
            name = getattr(chat, 'title', None) or getattr(chat, 'first_name', 'Unknown')
            print(f"  - {name} (ID: {chat.id})")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
