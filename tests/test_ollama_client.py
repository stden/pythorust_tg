"""Tests for integrations/ollama_client.py module."""

import json
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch


sys.path.insert(0, str(Path(__file__).parent.parent))


class TestIsOllamaRunning:
    """Tests for is_ollama_running function."""

    def test_is_ollama_running_true(self):
        """Test when Ollama is running."""
        from integrations.ollama_client import is_ollama_running

        with patch("requests.get") as mock_get:
            mock_get.return_value = MagicMock(status_code=200)
            assert is_ollama_running() is True

    def test_is_ollama_running_false_error(self):
        """Test when Ollama is not running (connection error)."""
        import requests

        with patch("integrations.ollama_client.requests.get") as mock_get:
            mock_get.side_effect = requests.RequestException("Connection refused")

            from integrations.ollama_client import is_ollama_running

            assert is_ollama_running() is False

    def test_is_ollama_running_false_bad_status(self):
        """Test when Ollama returns non-200 status."""
        from integrations.ollama_client import is_ollama_running

        with patch("requests.get") as mock_get:
            mock_get.return_value = MagicMock(status_code=500)
            assert is_ollama_running() is False


class TestListModels:
    """Tests for list_models function."""

    def test_list_models(self):
        """Test listing available models."""
        from integrations.ollama_client import list_models

        with patch("requests.get") as mock_get:
            mock_get.return_value = MagicMock(
                status_code=200,
                json=MagicMock(
                    return_value={
                        "models": [
                            {"name": "qwen2.5:3b"},
                            {"name": "llama3:8b"},
                        ]
                    }
                ),
            )

            models = list_models()
            assert "qwen2.5:3b" in models
            assert "llama3:8b" in models

    def test_list_models_empty(self):
        """Test listing models when none available."""
        from integrations.ollama_client import list_models

        with patch("requests.get") as mock_get:
            mock_get.return_value = MagicMock(status_code=200, json=MagicMock(return_value={"models": []}))

            models = list_models()
            assert models == []


class TestGenerate:
    """Tests for generate function."""

    def test_generate_basic(self):
        """Test basic text generation."""
        from integrations.ollama_client import generate

        with patch("requests.post") as mock_post:
            mock_post.return_value = MagicMock(
                status_code=200, json=MagicMock(return_value={"response": "Hello there!"})
            )

            result = generate("Hello!")
            assert result == "Hello there!"

    def test_generate_with_system_prompt(self):
        """Test generation with system prompt."""
        from integrations.ollama_client import generate

        with patch("requests.post") as mock_post:
            mock_post.return_value = MagicMock(
                status_code=200, json=MagicMock(return_value={"response": "Ответ на русском"})
            )

            result = generate("Привет!", model="qwen2.5:3b", system="Отвечай на русском")
            assert result == "Ответ на русском"

            call_kwargs = mock_post.call_args.kwargs
            assert "system" in call_kwargs["json"]

    def test_generate_with_custom_params(self):
        """Test generation with custom parameters."""
        from integrations.ollama_client import generate

        with patch("requests.post") as mock_post:
            mock_post.return_value = MagicMock(
                status_code=200, json=MagicMock(return_value={"response": "Custom response"})
            )

            result = generate("Test", model="llama3:8b", temperature=0.5, max_tokens=1000)
            assert result == "Custom response"

            call_kwargs = mock_post.call_args.kwargs
            assert call_kwargs["json"]["model"] == "llama3:8b"
            assert call_kwargs["json"]["options"]["temperature"] == 0.5
            assert call_kwargs["json"]["options"]["num_predict"] == 1000

    def test_generate_stream(self):
        """Test streaming generation."""
        from integrations.ollama_client import generate

        with patch("requests.post") as mock_post:
            mock_response = MagicMock()
            mock_response.iter_lines.return_value = [
                json.dumps({"response": "Hello"}).encode(),
                json.dumps({"response": " world"}).encode(),
            ]
            mock_post.return_value = mock_response

            gen = generate("Hi", stream=True)
            chunks = list(gen)
            assert chunks == ["Hello", " world"]

    def test_generate_stream_skips_empty_and_non_response_chunks(self):
        """Test streaming generation ignores empty lines and chunks without 'response'."""
        from integrations.ollama_client import generate

        with patch("requests.post") as mock_post:
            mock_response = MagicMock()
            mock_response.iter_lines.return_value = [
                b"",
                json.dumps({"foo": "bar"}).encode(),
                json.dumps({"response": "Hello"}).encode(),
                json.dumps({"response": "!"}).encode(),
            ]
            mock_post.return_value = mock_response

            chunks = list(generate("Hi", stream=True))
            assert chunks == ["Hello", "!"]


class TestChat:
    """Tests for chat function."""

    def test_chat_basic(self):
        """Test basic chat functionality."""
        from integrations.ollama_client import chat

        with patch("requests.post") as mock_post:
            mock_post.return_value = MagicMock(
                status_code=200, json=MagicMock(return_value={"message": {"content": "Hello! How can I help?"}})
            )

            result = chat([{"role": "user", "content": "Hello"}])
            assert result == "Hello! How can I help?"

    def test_chat_with_history(self):
        """Test chat with message history."""
        from integrations.ollama_client import chat

        with patch("requests.post") as mock_post:
            mock_post.return_value = MagicMock(
                status_code=200, json=MagicMock(return_value={"message": {"content": "Based on our conversation..."}})
            )

            messages = [
                {"role": "system", "content": "Be helpful"},
                {"role": "user", "content": "Hi"},
                {"role": "assistant", "content": "Hello!"},
                {"role": "user", "content": "Continue"},
            ]
            result = chat(messages)
            assert result == "Based on our conversation..."


class TestSalesAgentResponse:
    """Tests for sales_agent_response function."""

    def test_sales_agent_response_basic(self):
        """Test basic sales agent response."""
        from integrations.ollama_client import sales_agent_response

        with patch("integrations.ollama_client.generate") as mock_generate:
            mock_generate.return_value = "Понимаю ваше беспокойство..."

            result = sales_agent_response("Это дорого")
            assert "беспокойство" in result

    def test_sales_agent_response_with_context(self):
        """Test sales agent response with context."""
        from integrations.ollama_client import sales_agent_response

        with patch("integrations.ollama_client.generate") as mock_generate:
            mock_generate.return_value = "Курс окупится!"

            result = sales_agent_response("Мне нужно подумать", context="Курс программирования за 50,000 руб")
            assert result == "Курс окупится!"

            call_kwargs = mock_generate.call_args.kwargs
            assert "Контекст:" in call_kwargs["system"]


class TestPullModel:
    """Tests for pull_model function."""

    def test_pull_model_success(self):
        """Test successful model download."""
        from integrations.ollama_client import pull_model

        with patch("requests.post") as mock_post:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.iter_lines.return_value = [
                json.dumps({"status": "downloading"}).encode(),
                json.dumps({"status": "success"}).encode(),
            ]
            mock_post.return_value = mock_response

            result = pull_model("qwen2.5:3b")
            assert result is True

    def test_pull_model_failure(self):
        """Test failed model download."""
        from integrations.ollama_client import pull_model

        with patch("requests.post") as mock_post:
            mock_response = MagicMock()
            mock_response.status_code = 500
            mock_response.iter_lines.return_value = []
            mock_post.return_value = mock_response

            result = pull_model("nonexistent:model")
            assert result is False

    def test_pull_model_ignores_empty_and_unexpected_lines(self):
        """Test model download handles empty lines and unexpected JSON objects."""
        from integrations.ollama_client import pull_model

        with patch("builtins.print"), patch("requests.post") as mock_post:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.iter_lines.return_value = [
                b"",
                json.dumps({"progress": "50%"}).encode(),
                json.dumps({"status": "downloading"}).encode(),
            ]
            mock_post.return_value = mock_response

            assert pull_model("qwen2.5:3b") is True
