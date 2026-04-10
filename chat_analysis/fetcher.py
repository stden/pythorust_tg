"""Message fetcher for Telegram chats."""

import os
from datetime import datetime, timedelta
from typing import Optional, List, Dict, Any
from dataclasses import dataclass

from telethon import TelegramClient
from telethon.tl.types import Message, User, Channel, Chat
from dotenv import load_dotenv

from .config import AnalyzerConfig

load_dotenv()


@dataclass
class FormattedMessage:
    """Single formatted message for LLM analysis."""

    date: datetime
    sender_name: str
    text: str
    message_id: int
    reactions_count: int = 0
    has_media: bool = False


class MessageFetcher:
    """Fetches and formats messages from Telegram chats."""

    def __init__(self, config: AnalyzerConfig):
        """Initialize fetcher with configuration.

        Args:
            config: Analyzer configuration
        """
        self.config = config
        self.client: Optional[TelegramClient] = None

        # Get Telegram credentials from environment
        self.api_id = int(os.getenv("TELEGRAM_API_ID"))
        self.api_hash = os.getenv("TELEGRAM_API_HASH")
        self.session_file = os.getenv("TELEGRAM_SESSION_FILE", "telegram_session")

    async def __aenter__(self):
        """Async context manager entry."""
        self.client = TelegramClient(self.session_file, self.api_id, self.api_hash)
        await self.client.connect()

        if not await self.client.is_user_authorized():
            raise RuntimeError("Telegram client not authorized. Run `cargo run -- init-session` first.")

        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        if self.client:
            await self.client.disconnect()

    async def get_messages(self, chat_identifier: str) -> List[FormattedMessage]:
        """Fetch messages from a Telegram chat.

        Args:
            chat_identifier: Chat username, ID, or URL

        Returns:
            List of formatted messages

        Raises:
            RuntimeError: If client not connected
            ValueError: If chat not found
        """
        if not self.client:
            raise RuntimeError("Client not connected. Use async context manager.")

        # Get chat entity
        try:
            entity = await self.client.get_entity(chat_identifier)
        except Exception as e:
            raise ValueError(f"Could not find chat '{chat_identifier}': {e}")

        # Calculate offset date
        offset_date = self._get_offset_date()

        # Fetch messages
        messages = []
        async for message in self.client.iter_messages(
            entity, limit=self.config.message_limit, offset_date=offset_date, reverse=False
        ):
            if await self._should_include(message):
                formatted = await self._format_message(message)
                if formatted:
                    messages.append(formatted)

        if self.config.verbose:
            print(f"Fetched {len(messages)} messages from {chat_identifier}")

        return messages

    def _get_offset_date(self) -> Optional[datetime]:
        """Calculate offset date based on days_back configuration.

        Returns:
            Offset datetime or None if no limit
        """
        if self.config.days_back <= 0:
            return None

        return datetime.now() - timedelta(days=self.config.days_back)

    async def _should_include(self, message: Message) -> bool:
        """Check if message should be included based on filters.

        Args:
            message: Telegram message

        Returns:
            True if message passes filters
        """
        # Skip empty messages
        if not message.message:
            return False

        # Check message length
        if len(message.message) < self.config.min_message_length:
            return False

        # Check if sender is bot
        if self.config.exclude_bots and message.sender:
            if isinstance(message.sender, User) and message.sender.bot:
                return False

        # Check media inclusion
        if not self.config.include_media and message.media:
            # Still include if there's text content
            if not message.message or len(message.message) < self.config.min_message_length:
                return False

        return True

    async def _format_message(self, message: Message) -> Optional[FormattedMessage]:
        """Format a single message for LLM analysis.

        Args:
            message: Telegram message

        Returns:
            FormattedMessage or None if cannot format
        """
        # Get sender name
        sender_name = await self._get_sender_name(message)

        # Count reactions
        reactions_count = 0
        if message.reactions:
            for reaction in message.reactions.results:
                reactions_count += reaction.count

        # Check for media
        has_media = message.media is not None

        return FormattedMessage(
            date=message.date,
            sender_name=sender_name,
            text=message.message,
            message_id=message.id,
            reactions_count=reactions_count,
            has_media=has_media,
        )

    async def _get_sender_name(self, message: Message) -> str:
        """Extract sender name from message.

        Args:
            message: Telegram message

        Returns:
            Sender name or 'Unknown'
        """
        if not message.sender:
            return "Unknown"

        sender = message.sender

        # User
        if isinstance(sender, User):
            if sender.first_name or sender.last_name:
                name = f"{sender.first_name or ''} {sender.last_name or ''}".strip()
                return name if name else "Unknown"
            elif sender.username:
                return f"@{sender.username}"
            else:
                return f"User{sender.id}"

        # Channel or Chat
        elif isinstance(sender, (Channel, Chat)):
            return sender.title or "Unknown"

        return "Unknown"

    def format_messages_for_llm(self, messages: List[FormattedMessage]) -> str:
        """Format messages for LLM input.

        Args:
            messages: List of formatted messages

        Returns:
            Formatted string for LLM
        """
        lines = []

        for msg in messages:
            # Format date
            date_str = msg.date.strftime("%d.%m.%Y %H:%M")

            # Add reactions indicator if any
            reactions_str = f" [{msg.reactions_count} reactions]" if msg.reactions_count > 0 else ""

            # Add media indicator
            media_str = " [media]" if msg.has_media else ""

            # Format line
            line = f"[{date_str}] {msg.sender_name}: {msg.text}{reactions_str}{media_str}"
            lines.append(line)

        return "\n".join(lines)

    def get_metadata(self, messages: List[FormattedMessage]) -> Dict[str, Any]:
        """Extract metadata from messages for analysis context.

        Args:
            messages: List of formatted messages

        Returns:
            Metadata dictionary
        """
        if not messages:
            return {
                "total_messages": 0,
                "date_range": None,
                "unique_senders": 0,
                "has_media": False,
                "total_reactions": 0,
            }

        # Calculate metrics
        total_reactions = sum(msg.reactions_count for msg in messages)
        unique_senders = len(set(msg.sender_name for msg in messages))
        has_media = any(msg.has_media for msg in messages)

        # Date range
        dates = [msg.date for msg in messages]
        date_range = {"start": min(dates).isoformat(), "end": max(dates).isoformat()}

        return {
            "total_messages": len(messages),
            "date_range": date_range,
            "unique_senders": unique_senders,
            "has_media": has_media,
            "total_reactions": total_reactions,
        }
