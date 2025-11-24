# -*- coding: utf-8 -*-
"""Tests for Ollama client."""
import json
import pytest
from unittest.mock import MagicMock, patch


class TestOllamaClient:
    """Tests for Ollama client functions."""

    def test_is_ollama_running_true(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_requests.get.return_value = mock_response

            from integrations.ollama_client import is_ollama_running

            result = is_ollama_running()
            assert result is True

    def test_is_ollama_running_false(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.status_code = 500
            mock_requests.get.return_value = mock_response

            from integrations.ollama_client import is_ollama_running

            result = is_ollama_running()
            assert result is False

    def test_is_ollama_running_connection_error(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            import requests
            mock_requests.RequestException = requests.RequestException
            mock_requests.get.side_effect = requests.RequestException("Connection refused")

            from integrations.ollama_client import is_ollama_running

            result = is_ollama_running()
            assert result is False

    def test_list_models(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.json.return_value = {
                "models": [
                    {"name": "qwen2.5:3b"},
                    {"name": "llama3:8b"},
                ]
            }
            mock_response.raise_for_status = MagicMock()
            mock_requests.get.return_value = mock_response

            from integrations.ollama_client import list_models

            result = list_models()
            assert result == ["qwen2.5:3b", "llama3:8b"]

    def test_list_models_empty(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.json.return_value = {"models": []}
            mock_response.raise_for_status = MagicMock()
            mock_requests.get.return_value = mock_response

            from integrations.ollama_client import list_models

            result = list_models()
            assert result == []

    def test_generate_no_stream(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.json.return_value = {"response": "Generated text"}
            mock_response.raise_for_status = MagicMock()
            mock_requests.post.return_value = mock_response

            from integrations.ollama_client import generate

            result = generate("Test prompt")
            assert result == "Generated text"

    def test_generate_with_system(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.json.return_value = {"response": "System response"}
            mock_response.raise_for_status = MagicMock()
            mock_requests.post.return_value = mock_response

            from integrations.ollama_client import generate

            result = generate("Prompt", system="You are helpful")
            assert result == "System response"

            # Verify system was included in request
            call_kwargs = mock_requests.post.call_args.kwargs
            assert "system" in call_kwargs["json"]
            assert call_kwargs["json"]["system"] == "You are helpful"

    def test_generate_custom_params(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.json.return_value = {"response": "Custom response"}
            mock_response.raise_for_status = MagicMock()
            mock_requests.post.return_value = mock_response

            from integrations.ollama_client import generate

            result = generate(
                "Test",
                model="llama3:8b",
                temperature=0.5,
                max_tokens=200,
            )
            assert result == "Custom response"

            call_kwargs = mock_requests.post.call_args.kwargs
            assert call_kwargs["json"]["model"] == "llama3:8b"
            assert call_kwargs["json"]["options"]["temperature"] == 0.5
            assert call_kwargs["json"]["options"]["num_predict"] == 200

    def test_generate_stream(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            # Create mock response with iter_lines
            mock_response = MagicMock()
            mock_response.raise_for_status = MagicMock()
            mock_response.iter_lines.return_value = [
                json.dumps({"response": "Hello"}).encode(),
                json.dumps({"response": " World"}).encode(),
            ]
            mock_requests.post.return_value = mock_response

            from integrations.ollama_client import generate

            result = generate("Test", stream=True)
            chunks = list(result)
            assert chunks == ["Hello", " World"]

    def test_chat(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.json.return_value = {"message": {"content": "Chat response"}}
            mock_response.raise_for_status = MagicMock()
            mock_requests.post.return_value = mock_response

            from integrations.ollama_client import chat

            result = chat([{"role": "user", "content": "Hello"}])
            assert result == "Chat response"

    def test_chat_custom_model(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.json.return_value = {"message": {"content": "Response"}}
            mock_response.raise_for_status = MagicMock()
            mock_requests.post.return_value = mock_response

            from integrations.ollama_client import chat

            result = chat(
                [{"role": "user", "content": "Hi"}],
                model="llama3:8b",
                temperature=0.9,
            )
            assert result == "Response"

            call_kwargs = mock_requests.post.call_args.kwargs
            assert call_kwargs["json"]["model"] == "llama3:8b"

    def test_sales_agent_response(self):
        with patch("integrations.ollama_client.generate") as mock_generate:
            mock_generate.return_value = "Sales pitch"

            from integrations.ollama_client import sales_agent_response

            result = sales_agent_response("Too expensive")
            assert result == "Sales pitch"

    def test_sales_agent_response_with_context(self):
        with patch("integrations.ollama_client.generate") as mock_generate:
            mock_generate.return_value = "Contextual pitch"

            from integrations.ollama_client import sales_agent_response

            result = sales_agent_response(
                "Too expensive",
                context="Course: 50000 RUB",
                model="llama3:8b"
            )
            assert result == "Contextual pitch"

            # Verify context was included in system prompt
            call_kwargs = mock_generate.call_args.kwargs
            assert "50000 RUB" in call_kwargs["system"]

    def test_pull_model(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.status_code = 200
            mock_response.iter_lines.return_value = [
                json.dumps({"status": "downloading"}).encode(),
                json.dumps({"status": "complete"}).encode(),
            ]
            mock_requests.post.return_value = mock_response

            from integrations.ollama_client import pull_model

            with patch("builtins.print"):  # Suppress print output
                result = pull_model("qwen2.5:3b")
            assert result is True

    def test_pull_model_failure(self):
        with patch("integrations.ollama_client.requests") as mock_requests:
            mock_response = MagicMock()
            mock_response.status_code = 500
            mock_response.iter_lines.return_value = []
            mock_requests.post.return_value = mock_response

            from integrations.ollama_client import pull_model

            with patch("builtins.print"):
                result = pull_model("nonexistent:model")
            assert result is False


class TestOllamaURL:
    """Tests for OLLAMA_URL constant."""

    def test_default_url(self):
        from integrations.ollama_client import OLLAMA_URL
        assert OLLAMA_URL == "http://localhost:11434"
