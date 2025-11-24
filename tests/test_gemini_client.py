# -*- coding: utf-8 -*-
"""Tests for Gemini API client."""
import base64
import os
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

# Set API key before import
os.environ.setdefault("GOOGLE_API_KEY", "test-api-key")

from integrations.gemini_client import (
    GeminiClient,
    GeminiMessage,
    GeminiResponse,
    GEMINI_MODELS,
    quick_chat,
)


class TestGeminiMessage:
    """Tests for GeminiMessage dataclass."""

    def test_create_user_message(self):
        msg = GeminiMessage(role="user", content="Hello")
        assert msg.role == "user"
        assert msg.content == "Hello"

    def test_create_model_message(self):
        msg = GeminiMessage(role="model", content="Hi there!")
        assert msg.role == "model"
        assert msg.content == "Hi there!"


class TestGeminiResponse:
    """Tests for GeminiResponse dataclass."""

    def test_create_response(self):
        response = GeminiResponse(
            content="Test response",
            model="gemini-2.5-flash-preview-05-20",
            finish_reason="STOP",
            prompt_tokens=10,
            candidates_tokens=20,
        )
        assert response.content == "Test response"
        assert response.model == "gemini-2.5-flash-preview-05-20"
        assert response.finish_reason == "STOP"
        assert response.prompt_tokens == 10
        assert response.candidates_tokens == 20


class TestGeminiClient:
    """Tests for GeminiClient class."""

    def test_init_with_api_key(self):
        client = GeminiClient(api_key="test-key")
        assert client.api_key == "test-key"
        assert client.model == "gemini-2.5-flash-preview-05-20"
        assert client.temperature == 0.7
        assert client.max_output_tokens == 8192

    def test_init_without_api_key_raises(self):
        with patch.dict(os.environ, {"GOOGLE_API_KEY": ""}, clear=False):
            with pytest.raises(ValueError, match="GOOGLE_API_KEY"):
                GeminiClient(api_key="")

    def test_init_custom_params(self):
        client = GeminiClient(
            api_key="test-key",
            model="gemini-1.5-pro",
            temperature=0.5,
            max_output_tokens=4096,
        )
        assert client.model == "gemini-1.5-pro"
        assert client.temperature == 0.5
        assert client.max_output_tokens == 4096

    @pytest.mark.asyncio
    async def test_chat_success(self):
        client = GeminiClient(api_key="test-key")

        mock_response = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "Hello from Gemini!"}]},
                    "finishReason": "STOP",
                }
            ],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 10,
            },
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await client.chat("Hello")
            assert result == "Hello from Gemini!"

    @pytest.mark.asyncio
    async def test_chat_full_success(self):
        client = GeminiClient(api_key="test-key")

        mock_response = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "Full response"}]},
                    "finishReason": "STOP",
                }
            ],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 20,
            },
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await client.chat_full("Test message")
            assert isinstance(result, GeminiResponse)
            assert result.content == "Full response"
            assert result.prompt_tokens == 10
            assert result.candidates_tokens == 20
            assert result.finish_reason == "STOP"

    @pytest.mark.asyncio
    async def test_chat_with_system_prompt(self):
        client = GeminiClient(api_key="test-key")

        mock_response = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "System response"}]},
                    "finishReason": "STOP",
                }
            ],
            "usageMetadata": {},
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await client.chat("Hello", system="You are helpful")
            assert result == "System response"

    @pytest.mark.asyncio
    async def test_chat_with_history(self):
        client = GeminiClient(api_key="test-key")

        history = [
            GeminiMessage(role="user", content="Previous question"),
            GeminiMessage(role="model", content="Previous answer"),
        ]

        mock_response = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "With history"}]},
                    "finishReason": "STOP",
                }
            ],
            "usageMetadata": {},
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await client.chat("New message", history=history)
            assert result == "With history"

    @pytest.mark.asyncio
    @pytest.mark.skip(reason="Complex async context manager mocking requires real httpx")
    async def test_chat_stream(self):
        # This test requires real httpx async context manager behavior
        pass

    @pytest.mark.asyncio
    async def test_analyze_image(self):
        client = GeminiClient(api_key="test-key")

        mock_response = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "Image analysis result"}]}
                }
            ],
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await client.analyze_image(
                b"fake image data",
                "What's in this image?",
                mime_type="image/png"
            )
            assert result == "Image analysis result"

    @pytest.mark.asyncio
    async def test_analyze_image_with_system(self):
        client = GeminiClient(api_key="test-key")

        mock_response = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "Detailed analysis"}]}
                }
            ],
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await client.analyze_image(
                b"image bytes",
                "Analyze",
                system="Be detailed"
            )
            assert result == "Detailed analysis"

    @pytest.mark.asyncio
    async def test_generate_image(self):
        client = GeminiClient(api_key="test-key")

        # Create fake base64 image data
        fake_image = b"fake image bytes"
        fake_base64 = base64.b64encode(fake_image).decode("utf-8")

        mock_response = {
            "predictions": [{"bytesBase64Encoded": fake_base64}]
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await client.generate_image("A beautiful sunset")
            assert result == fake_image


class TestGeminiModels:
    """Tests for GEMINI_MODELS constant."""

    def test_models_dict_exists(self):
        assert isinstance(GEMINI_MODELS, dict)
        assert len(GEMINI_MODELS) > 0

    def test_gemini_flash_models_exist(self):
        assert "gemini-1.5-flash" in GEMINI_MODELS
        assert "gemini-2.0-flash" in GEMINI_MODELS
        assert "gemini-2.5-flash" in GEMINI_MODELS

    def test_gemini_pro_models_exist(self):
        assert "gemini-1.5-pro" in GEMINI_MODELS
        assert "gemini-2.5-pro" in GEMINI_MODELS


class TestQuickChat:
    """Tests for quick_chat function."""

    @pytest.mark.asyncio
    async def test_quick_chat_default_model(self):
        mock_response = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "Quick response"}]},
                    "finishReason": "STOP",
                }
            ],
            "usageMetadata": {},
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await quick_chat("Hello")
            assert result == "Quick response"

    @pytest.mark.asyncio
    async def test_quick_chat_custom_model(self):
        mock_response = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "Pro response"}]},
                    "finishReason": "STOP",
                }
            ],
            "usageMetadata": {},
        }

        with patch("httpx.AsyncClient") as mock_client:
            mock_instance = AsyncMock()
            mock_client.return_value.__aenter__.return_value = mock_instance
            mock_instance.post.return_value = MagicMock(
                json=MagicMock(return_value=mock_response),
                raise_for_status=MagicMock(),
            )

            result = await quick_chat("Hello", model="gemini-2.5-pro")
            assert result == "Pro response"
