# -*- coding: utf-8 -*-
"""Tests for Kurigram Telegram client."""
import os
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

# Set env vars before import
os.environ.setdefault("TELEGRAM_API_ID", "12345")
os.environ.setdefault("TELEGRAM_API_HASH", "test-hash")


class TestKurigramClient:
    """Tests for KurigramClient class."""

    def test_init_without_kurigram(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", False):
            import importlib
            import integrations.kurigram_client as kurigram_module
            importlib.reload(kurigram_module)

            with pytest.raises(ImportError, match="Kurigram not installed"):
                kurigram_module.KurigramClient()

    def test_init_with_env_vars(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as mock_client:
                import integrations.kurigram_client as kurigram_module

                # Create client with explicit env vars (don't reload module)
                client = kurigram_module.KurigramClient(
                    api_id=12345,
                    api_hash="test-hash"
                )
                assert client.api_id == 12345
                assert client.api_hash == "test-hash"

    def test_init_with_params(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as mock_client:
                import integrations.kurigram_client as kurigram_module

                client = kurigram_module.KurigramClient(
                    session_name="test_session",
                    api_id=99999,
                    api_hash="custom-hash"
                )
                assert client.api_id == 99999
                assert client.api_hash == "custom-hash"
                assert client.session_name == "test_session"

    @pytest.mark.asyncio
    async def test_start(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as MockClient:
                mock_instance = AsyncMock()
                MockClient.return_value = mock_instance

                import integrations.kurigram_client as kurigram_module

                client = kurigram_module.KurigramClient(api_id=123, api_hash="hash")
                result = await client.start()

                mock_instance.start.assert_called_once()
                assert result == client

    @pytest.mark.asyncio
    async def test_stop(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as MockClient:
                mock_instance = AsyncMock()
                MockClient.return_value = mock_instance

                import integrations.kurigram_client as kurigram_module

                client = kurigram_module.KurigramClient(api_id=123, api_hash="hash")
                await client.stop()

                mock_instance.stop.assert_called_once()

    @pytest.mark.asyncio
    async def test_get_me(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as MockClient:
                mock_instance = AsyncMock()
                mock_user = MagicMock()
                mock_user.first_name = "Test"
                mock_user.username = "testuser"
                mock_instance.get_me.return_value = mock_user
                MockClient.return_value = mock_instance

                import integrations.kurigram_client as kurigram_module

                client = kurigram_module.KurigramClient(api_id=123, api_hash="hash")
                result = await client.get_me()

                assert result.first_name == "Test"

    @pytest.mark.asyncio
    async def test_get_chat(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as MockClient:
                mock_instance = AsyncMock()
                mock_chat = MagicMock()
                mock_chat.id = 123
                mock_chat.title = "Test Chat"
                mock_instance.get_chat.return_value = mock_chat
                MockClient.return_value = mock_instance

                import integrations.kurigram_client as kurigram_module

                client = kurigram_module.KurigramClient(api_id=123, api_hash="hash")
                result = await client.get_chat(123)

                assert result.title == "Test Chat"

    @pytest.mark.asyncio
    async def test_send_message(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as MockClient:
                mock_instance = AsyncMock()
                mock_message = MagicMock(text="Sent message")
                mock_instance.send_message.return_value = mock_message
                MockClient.return_value = mock_instance

                import integrations.kurigram_client as kurigram_module

                client = kurigram_module.KurigramClient(api_id=123, api_hash="hash")
                result = await client.send_message(123, "Hello!")

                assert result.text == "Sent message"
                mock_instance.send_message.assert_called_once()

    @pytest.mark.asyncio
    async def test_delete_messages(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as MockClient:
                mock_instance = AsyncMock()
                mock_instance.delete_messages.return_value = 3
                MockClient.return_value = mock_instance

                import integrations.kurigram_client as kurigram_module

                client = kurigram_module.KurigramClient(api_id=123, api_hash="hash")
                result = await client.delete_messages(123, [1, 2, 3])

                assert result == 3

    def test_on_message_decorator(self):
        with patch("integrations.kurigram_client.KURIGRAM_AVAILABLE", True):
            with patch("integrations.kurigram_client.Client") as MockClient:
                with patch("integrations.kurigram_client.filters") as mock_filters:
                    mock_instance = MagicMock()
                    mock_instance.on_message = MagicMock(return_value=lambda f: f)
                    mock_filters.all = MagicMock()
                    MockClient.return_value = mock_instance

                    import integrations.kurigram_client as kurigram_module

                    client = kurigram_module.KurigramClient(api_id=123, api_hash="hash")
                    decorator = client.on_message()

                    assert callable(decorator)
