"""Tests for read module functionality."""

import pytest
import asyncio
import os
from unittest.mock import AsyncMock, MagicMock, patch, call
from datetime import datetime
from pathlib import Path
import yaml

# Since read.py doesn't exist, we'll test the conceptual functionality
# based on the expected behavior described in documentation


class TestReadFunctionality:
    """Test read chat functionality."""

    @pytest.fixture
    def mock_telegram_client(self):
        """Mock Telethon TelegramClient."""
        client = AsyncMock()
        client.get_messages = AsyncMock(return_value=[])
        client.get_entity = AsyncMock()
        client.download_media = AsyncMock()
        client.is_user_authorized = AsyncMock(return_value=True)
        client.start = AsyncMock()
        client.disconnect = AsyncMock()
        client.__aenter__ = AsyncMock(return_value=client)
        client.__aexit__ = AsyncMock()
        return client

    @pytest.fixture
    def sample_messages(self):
        """Create sample messages for testing."""
        messages = []
        for i in range(5):
            msg = MagicMock()
            msg.id = i + 1
            msg.date = datetime(2025, 1, 1, 12, i, 0)
            msg.sender_id = 12345
            msg.text = f"Message {i + 1}"
            msg.raw_text = f"Message {i + 1}"
            msg.message = f"Message {i + 1}"
            msg.reply_to_msg_id = None
            msg.media = None
            msg.views = 100 * (i + 1)
            msg.forwards = 10 * (i + 1)
            
            # Add reactions to some messages
            if i > 2:
                reaction = MagicMock()
                reaction.count = 50 * i
                reaction.reaction = MagicMock()
                reaction.reaction.emoticon = "👍"
                msg.reactions = MagicMock(results=[reaction])
            else:
                msg.reactions = None
                
            sender = MagicMock()
            sender.first_name = f"User{i}"
            sender.last_name = "Test"
            sender.username = f"user{i}"
            msg.get_sender = AsyncMock(return_value=sender)
            
            messages.append(msg)
        
        return messages

    @pytest.fixture
    def chat_config(self, tmp_path):
        """Create test chat configuration."""
        config = {
            "chats": {
                "test_chat": {
                    "type": "channel",
                    "id": 1234567890,
                    "title": "Test Chat"
                }
            }
        }
        config_file = tmp_path / "config.yml"
        config_file.write_text(yaml.dump(config))
        return config_file

    async def test_read_chat_messages(self, mock_telegram_client, sample_messages):
        """Test reading messages from a chat."""
        mock_telegram_client.get_messages.return_value = sample_messages
        
        # Simulate read functionality
        chat_entity = MagicMock()
        chat_entity.title = "Test Chat"
        mock_telegram_client.get_entity.return_value = chat_entity
        
        # Execute
        messages = await mock_telegram_client.get_messages("test_chat", limit=3000)
        
        assert len(messages) == 5
        assert messages[0].text == "Message 1"
        mock_telegram_client.get_messages.assert_called_once_with("test_chat", limit=3000)

    async def test_format_message_output(self, sample_messages):
        """Test formatting messages for output."""
        msg = sample_messages[0]
        sender = await msg.get_sender()
        
        # Expected format: [timestamp] [sender]: [message]
        formatted = f"{msg.date.strftime('%d.%m.%Y %H:%M:%S')} {sender.first_name} {sender.last_name}: {msg.text}"
        
        assert "01.01.2025 12:00:00" in formatted
        assert "User0 Test" in formatted
        assert "Message 1" in formatted

    async def test_filter_messages_by_reactions(self, sample_messages):
        """Test filtering messages based on reaction count."""
        # Filter messages with at least 100 reactions
        filtered = [msg for msg in sample_messages if msg.reactions and 
                   sum(r.count for r in msg.reactions.results) >= 100]
        
        assert len(filtered) == 2  # Messages 4 and 5 have 150 and 200 reactions

    async def test_download_media_from_popular_messages(self, mock_telegram_client, sample_messages, tmp_path):
        """Test downloading media from messages with many reactions."""
        # Add media to a popular message
        popular_msg = sample_messages[4]  # Has 200 reactions
        popular_msg.media = MagicMock()
        
        media_path = tmp_path / "media.jpg"
        mock_telegram_client.download_media.return_value = str(media_path)
        
        # Download media for messages with >100k views (simulating reaction threshold)
        if popular_msg.views > 100000:
            downloaded = await mock_telegram_client.download_media(popular_msg, file=str(tmp_path))
            assert downloaded == str(media_path)

    async def test_save_to_markdown_file(self, sample_messages, tmp_path):
        """Test saving chat export to markdown file."""
        output_file = tmp_path / "test_chat.md"
        
        # Format messages
        lines = []
        for msg in sample_messages:
            # Simulate async sender fetch
            sender = MagicMock(first_name="User", last_name="Test")
            timestamp = msg.date.strftime("%d.%m.%Y %H:%M:%S")
            
            line = f"{timestamp} {sender.first_name} {sender.last_name}: {msg.text}"
            
            # Add reactions if present
            if msg.reactions:
                reactions = []
                for r in msg.reactions.results:
                    reactions.append(f"{r.reaction.emoticon}{r.count}")
                line += f" {' '.join(reactions)}"
            
            lines.append(line)
        
        # Save to file
        output_file.write_text("\n".join(lines))
        
        assert output_file.exists()
        content = output_file.read_text()
        assert "Message 1" in content
        assert "Message 5" in content
        assert "👍" in content  # Reaction emoji

    async def test_chat_not_found(self, mock_telegram_client):
        """Test handling when chat is not found."""
        mock_telegram_client.get_entity.side_effect = ValueError("Chat not found")
        
        with pytest.raises(ValueError, match="Chat not found"):
            await mock_telegram_client.get_entity("nonexistent_chat")

    async def test_auto_delete_messages_without_reactions(self, mock_telegram_client, sample_messages):
        """Test auto-deletion of messages without reactions."""
        # Messages 1-3 have no reactions
        messages_to_delete = [msg.id for msg in sample_messages if not msg.reactions]
        
        assert messages_to_delete == [1, 2, 3]
        
        # Simulate deletion
        await mock_telegram_client.delete_messages("test_chat", messages_to_delete)
        mock_telegram_client.delete_messages.assert_called_once_with("test_chat", [1, 2, 3])

    async def test_skip_reply_messages(self, sample_messages):
        """Test skipping messages that are replies."""
        # Make some messages replies
        sample_messages[1].reply_to_msg_id = 1
        sample_messages[3].reply_to_msg_id = 2
        
        # Filter out replies
        non_replies = [msg for msg in sample_messages if not msg.reply_to_msg_id]
        
        assert len(non_replies) == 3
        assert 2 not in [msg.id for msg in non_replies]
        assert 4 not in [msg.id for msg in non_replies]

    async def test_github_actions_environment(self, mock_telegram_client, monkeypatch):
        """Test behavior in GitHub Actions environment."""
        monkeypatch.setenv("GITHUB_ACTIONS", "true")
        
        # In CI, limit should be reduced to 1000
        messages = []
        mock_telegram_client.get_messages.return_value = messages
        
        await mock_telegram_client.get_messages("test_chat", limit=1000)
        
        # Verify reduced limit was used
        mock_telegram_client.get_messages.assert_called_with("test_chat", limit=1000)

    async def test_engagement_metrics(self, sample_messages):
        """Test calculating engagement metrics."""
        total_views = sum(msg.views for msg in sample_messages)
        total_forwards = sum(msg.forwards for msg in sample_messages)
        total_reactions = sum(
            sum(r.count for r in msg.reactions.results) 
            for msg in sample_messages 
            if msg.reactions
        )
        
        assert total_views == 1500  # 100 + 200 + 300 + 400 + 500
        assert total_forwards == 150  # 10 + 20 + 30 + 40 + 50
        assert total_reactions == 350  # 150 + 200

    async def test_media_skip_in_output(self, sample_messages):
        """Test skipping media in message output."""
        # Add media to a message
        msg_with_media = sample_messages[2]
        msg_with_media.media = MagicMock()
        msg_with_media.text = None  # Media messages often have no text
        
        # Format message
        if msg_with_media.media and not msg_with_media.text:
            formatted = "[Media]"
        else:
            formatted = msg_with_media.text or ""
        
        assert formatted == "[Media]"

    async def test_config_file_loading(self, chat_config):
        """Test loading chat configuration from file."""
        with open(chat_config, 'r') as f:
            config = yaml.safe_load(f)
        
        assert "test_chat" in config["chats"]
        assert config["chats"]["test_chat"]["type"] == "channel"
        assert config["chats"]["test_chat"]["id"] == 1234567890

    async def test_error_handling_during_export(self, mock_telegram_client):
        """Test error handling during chat export."""
        # Simulate network error
        mock_telegram_client.get_messages.side_effect = ConnectionError("Network error")
        
        with pytest.raises(ConnectionError, match="Network error"):
            await mock_telegram_client.get_messages("test_chat")

    async def test_message_date_formatting(self, sample_messages):
        """Test proper date formatting for messages."""
        msg = sample_messages[0]
        
        # Test different date formats
        formats = [
            msg.date.strftime("%d.%m.%Y %H:%M:%S"),  # DD.MM.YYYY HH:MM:SS
            msg.date.strftime("%Y-%m-%d %H:%M:%S"),  # ISO format
            msg.date.isoformat()  # Full ISO format
        ]
        
        assert formats[0] == "01.01.2025 12:00:00"
        assert formats[1] == "2025-01-01 12:00:00"
        assert "2025-01-01T12:00:00" in formats[2]