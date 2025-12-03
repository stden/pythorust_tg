"""Tests for Telegram session management."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from telegram_session import (
    SessionConfig,
    SessionError,
    TelegramSession,
    create_session,
    delete_messages,
    download_media,
    get_authorized_client,
    get_chat,
    get_messages,
    send_message,
)


class TestSessionConfig:
    """Test SessionConfig class."""

    def test_from_env(self, monkeypatch):
        """Test loading config from environment."""
        monkeypatch.setenv("TELEGRAM_API_ID", "12345")
        monkeypatch.setenv("TELEGRAM_API_HASH", "test_hash")
        monkeypatch.setenv("TELEGRAM_PHONE", "+1234567890")
        monkeypatch.setenv("TELEGRAM_SESSION_NAME", "test_session")

        config = SessionConfig.from_env()

        assert config.api_id == 12345
        assert config.api_hash == "test_hash"
        assert config.phone == "+1234567890"
        assert config.session_name == "test_session"

    def test_from_env_defaults(self, monkeypatch):
        """Test loading config with defaults."""
        monkeypatch.setenv("TELEGRAM_API_ID", "12345")
        monkeypatch.setenv("TELEGRAM_API_HASH", "test_hash")
        monkeypatch.setenv("TELEGRAM_PHONE", "+1234567890")
        monkeypatch.delenv("TELEGRAM_SESSION_NAME", raising=False)

        config = SessionConfig.from_env()

        assert config.session_name == "telethon"

    def test_from_env_missing_required(self, monkeypatch):
        """Test error when missing required config."""
        monkeypatch.delenv("TELEGRAM_API_ID", raising=False)

        with pytest.raises(ValueError, match="TELEGRAM_API_ID"):
            SessionConfig.from_env()


class TestTelegramSession:
    """Test TelegramSession class."""

    @pytest.fixture
    def config(self):
        """Create test config."""
        return SessionConfig(api_id=12345, api_hash="test_hash", phone="+1234567890", session_name="test_session")

    @pytest.fixture
    def mock_telethon_client(self):
        """Mock Telethon client."""
        with patch("telegram_session.TelegramClient") as mock_class:
            client = AsyncMock()
            mock_class.return_value = client

            # Set up default mock behaviors
            client.is_user_authorized = AsyncMock(return_value=True)
            client.start = AsyncMock()
            client.disconnect = AsyncMock()
            client.__aenter__ = AsyncMock(return_value=client)
            client.__aexit__ = AsyncMock()

            yield client

    @pytest.fixture
    def session(self, config, mock_telethon_client):
        """Create test session."""
        return TelegramSession(config)

    @pytest.mark.asyncio
    async def test_init(self, session, config):
        """Test session initialization."""
        assert session.config == config
        assert session._client is not None

    @pytest.mark.asyncio
    async def test_connect_authorized(self, session, mock_telethon_client):
        """Test connecting when already authorized."""
        await session.connect()

        mock_telethon_client.is_user_authorized.assert_called_once()
        mock_telethon_client.start.assert_not_called()

    @pytest.mark.asyncio
    async def test_connect_not_authorized(self, session, mock_telethon_client):
        """Test connecting when not authorized."""
        mock_telethon_client.is_user_authorized.return_value = False

        await session.connect()

        mock_telethon_client.start.assert_called_once_with(phone="+1234567890")

    @pytest.mark.asyncio
    async def test_disconnect(self, session, mock_telethon_client):
        """Test disconnecting."""
        await session.disconnect()

        mock_telethon_client.disconnect.assert_called_once()

    @pytest.mark.asyncio
    async def test_context_manager(self, config, mock_telethon_client):
        """Test using session as context manager."""
        async with TelegramSession(config) as session:
            assert session._client is not None

        mock_telethon_client.disconnect.assert_called_once()

    @pytest.mark.asyncio
    async def test_get_messages(self, session, mock_telethon_client):
        """Test getting messages."""
        # Mock messages
        messages = [MagicMock(id=1, text="Message 1"), MagicMock(id=2, text="Message 2")]
        mock_telethon_client.get_messages.return_value = messages

        result = await session.get_messages("test_chat", limit=10)

        assert len(result) == 2
        assert result[0].text == "Message 1"
        mock_telethon_client.get_messages.assert_called_once_with("test_chat", limit=10)

    @pytest.mark.asyncio
    async def test_send_message(self, session, mock_telethon_client):
        """Test sending a message."""
        mock_message = MagicMock(id=123)
        mock_telethon_client.send_message.return_value = mock_message

        result = await session.send_message("test_chat", "Hello")

        assert result.id == 123
        mock_telethon_client.send_message.assert_called_once_with("test_chat", "Hello")

    @pytest.mark.asyncio
    async def test_send_message_with_reply(self, session, mock_telethon_client):
        """Test sending a reply message."""
        mock_message = MagicMock(id=124)
        mock_telethon_client.send_message.return_value = mock_message

        result = await session.send_message("test_chat", "Reply text", reply_to=100)

        assert result.id == 124
        mock_telethon_client.send_message.assert_called_once_with("test_chat", "Reply text", reply_to=100)

    @pytest.mark.asyncio
    async def test_get_entity(self, session, mock_telethon_client):
        """Test getting an entity."""
        mock_entity = MagicMock(id=12345, title="Test Chat", username="test_chat")
        mock_telethon_client.get_entity.return_value = mock_entity

        result = await session.get_entity("test_chat")

        assert result.username == "test_chat"
        mock_telethon_client.get_entity.assert_called_once_with("test_chat")

    @pytest.mark.asyncio
    async def test_download_media(self, session, mock_telethon_client):
        """Test downloading media."""
        mock_message = MagicMock(media=True)
        mock_telethon_client.download_media.return_value = "path/to/file.jpg"

        result = await session.download_media(mock_message)

        assert result == "path/to/file.jpg"
        mock_telethon_client.download_media.assert_called_once()

    @pytest.mark.asyncio
    async def test_download_media_to_path(self, session, mock_telethon_client, tmp_path):
        """Test downloading media to specific path."""
        mock_message = MagicMock(media=True)
        output_path = tmp_path / "download.jpg"
        mock_telethon_client.download_media.return_value = str(output_path)

        result = await session.download_media(mock_message, file=str(output_path))

        assert result == str(output_path)

    @pytest.mark.asyncio
    async def test_delete_messages(self, session, mock_telethon_client):
        """Test deleting messages."""
        await session.delete_messages("test_chat", [1, 2, 3])

        mock_telethon_client.delete_messages.assert_called_once_with("test_chat", [1, 2, 3])

    @pytest.mark.asyncio
    async def test_iter_messages(self, session, mock_telethon_client):
        """Test iterating messages."""
        # Mock async iterator
        messages = [MagicMock(id=1, text="Message 1"), MagicMock(id=2, text="Message 2")]

        async def mock_iter():
            for msg in messages:
                yield msg

        mock_telethon_client.iter_messages.return_value = mock_iter()

        result = []
        async for msg in session.iter_messages("test_chat", limit=2):
            result.append(msg)

        assert len(result) == 2
        assert result[0].text == "Message 1"

    @pytest.mark.asyncio
    async def test_get_dialogs(self, session, mock_telethon_client):
        """Test getting dialogs."""
        dialogs = [MagicMock(name="Chat 1", unread_count=5), MagicMock(name="Chat 2", unread_count=0)]
        mock_telethon_client.get_dialogs.return_value = dialogs

        result = await session.get_dialogs()

        assert len(result) == 2
        assert result[0].unread_count == 5

    @pytest.mark.asyncio
    async def test_error_handling(self, session, mock_telethon_client):
        """Test error handling."""
        mock_telethon_client.send_message.side_effect = Exception("Network error")

        with pytest.raises(SessionError, match="Failed to send message"):
            await session.send_message("test_chat", "Hello")


class TestModuleFunctions:
    """Test module-level functions."""

    @patch("telegram_session.TelegramSession")
    def test_create_session(self, mock_session_class, mock_env):
        """Test create_session function."""
        session = create_session()

        mock_session_class.assert_called_once()
        assert session is not None

    @patch("telegram_session.TelegramSession")
    @pytest.mark.asyncio
    async def test_get_authorized_client(self, mock_session_class):
        """Test get_authorized_client function."""
        mock_session = AsyncMock()
        mock_session_class.return_value = mock_session
        mock_session.__aenter__.return_value = mock_session

        async with get_authorized_client() as client:
            assert client is not None

        mock_session.connect.assert_called_once()

    @patch("telegram_session.TelegramSession")
    @pytest.mark.asyncio
    async def test_send_message_function(self, mock_session_class):
        """Test send_message function."""
        mock_session = AsyncMock()
        mock_session_class.return_value = mock_session
        mock_session.__aenter__.return_value = mock_session
        mock_session.send_message.return_value = MagicMock(id=123)

        result = await send_message("test_chat", "Hello")

        assert result.id == 123
        mock_session.send_message.assert_called_once_with("test_chat", "Hello")

    @patch("telegram_session.TelegramSession")
    @pytest.mark.asyncio
    async def test_get_messages_function(self, mock_session_class):
        """Test get_messages function."""
        mock_session = AsyncMock()
        mock_session_class.return_value = mock_session
        mock_session.__aenter__.return_value = mock_session
        mock_session.get_messages.return_value = [MagicMock(), MagicMock()]

        result = await get_messages("test_chat", limit=10)

        assert len(result) == 2
        mock_session.get_messages.assert_called_once_with("test_chat", limit=10)

    @patch("telegram_session.TelegramSession")
    @pytest.mark.asyncio
    async def test_get_chat_function(self, mock_session_class):
        """Test get_chat function."""
        mock_session = AsyncMock()
        mock_session_class.return_value = mock_session
        mock_session.__aenter__.return_value = mock_session
        mock_session.get_entity.return_value = MagicMock(title="Test Chat")

        result = await get_chat("test_chat")

        assert result.title == "Test Chat"

    @patch("telegram_session.TelegramSession")
    @pytest.mark.asyncio
    async def test_download_media_function(self, mock_session_class):
        """Test download_media function."""
        mock_session = AsyncMock()
        mock_session_class.return_value = mock_session
        mock_session.__aenter__.return_value = mock_session
        mock_session.download_media.return_value = "path/to/media.jpg"

        mock_message = MagicMock()
        result = await download_media(mock_message)

        assert result == "path/to/media.jpg"

    @patch("telegram_session.TelegramSession")
    @pytest.mark.asyncio
    async def test_delete_messages_function(self, mock_session_class):
        """Test delete_messages function."""
        mock_session = AsyncMock()
        mock_session_class.return_value = mock_session
        mock_session.__aenter__.return_value = mock_session

        await delete_messages("test_chat", [1, 2, 3])

        mock_session.delete_messages.assert_called_once_with("test_chat", [1, 2, 3])


class TestSessionPersistence:
    """Test session file persistence."""

    @pytest.mark.asyncio
    async def test_session_file_creation(self, tmp_path, monkeypatch):
        """Test that session file is created."""
        session_file = tmp_path / "test.session"

        config = SessionConfig(api_id=12345, api_hash="test_hash", phone="+1234567890", session_name=str(session_file))

        with patch("telegram_session.TelegramClient") as mock_client_class:
            client = AsyncMock()
            mock_client_class.return_value = client
            client.is_user_authorized.return_value = True

            session = TelegramSession(config)
            await session.connect()

            # Verify client was created with session file path
            mock_client_class.assert_called_once_with(str(session_file), 12345, "test_hash")
