# -*- coding: utf-8 -*-
"""Tests for OpenAI API client."""
import os
import pytest
from unittest.mock import MagicMock, patch, mock_open

# Mock OpenAI before import
os.environ.setdefault("OPENAI_API_KEY", "test-api-key")


class TestOpenAIClient:
    """Tests for OpenAI client functions."""

    def test_chat_completion(self):
        with patch("integrations.openai_client.client") as mock_client:
            mock_response = MagicMock()
            mock_response.choices = [MagicMock(message=MagicMock(content="Hello!"))]
            mock_client.chat.completions.create.return_value = mock_response

            from integrations.openai_client import chat_completion

            result = chat_completion([{"role": "user", "content": "Hi"}])
            assert result == "Hello!"

    def test_chat_completion_custom_params(self):
        with patch("integrations.openai_client.client") as mock_client:
            mock_response = MagicMock()
            mock_response.choices = [MagicMock(message=MagicMock(content="Custom response"))]
            mock_client.chat.completions.create.return_value = mock_response

            from integrations.openai_client import chat_completion

            result = chat_completion(
                [{"role": "user", "content": "Test"}],
                model="gpt-4o",
                temperature=0.5,
                max_tokens=500,
            )
            assert result == "Custom response"

            # Verify the call was made with correct params
            mock_client.chat.completions.create.assert_called_once()
            call_kwargs = mock_client.chat.completions.create.call_args.kwargs
            assert call_kwargs["model"] == "gpt-4o"
            assert call_kwargs["temperature"] == 0.5
            assert call_kwargs["max_tokens"] == 500

    def test_sales_agent_response(self):
        with patch("integrations.openai_client.chat_completion") as mock_chat:
            mock_chat.return_value = "Sales response"

            from integrations.openai_client import sales_agent_response

            result = sales_agent_response("I need help")
            assert result == "Sales response"

    def test_sales_agent_response_with_context(self):
        with patch("integrations.openai_client.chat_completion") as mock_chat:
            mock_chat.return_value = "Contextual response"

            from integrations.openai_client import sales_agent_response

            result = sales_agent_response(
                "Is it expensive?",
                context="Product: Premium course for 50000 RUB"
            )
            assert result == "Contextual response"

            # Verify context was included
            call_args = mock_chat.call_args
            messages = call_args[0][0]
            system_content = messages[0]["content"]
            assert "Premium course" in system_content

    def test_transcribe_audio(self):
        with patch("integrations.openai_client.client") as mock_client:
            mock_transcription = MagicMock()
            mock_transcription.text = "Transcribed text"
            mock_client.audio.transcriptions.create.return_value = mock_transcription

            with patch("builtins.open", mock_open(read_data=b"audio data")):
                from integrations.openai_client import transcribe_audio

                result = transcribe_audio("/path/to/audio.mp3")
                assert result == "Transcribed text"

    def test_transcribe_audio_custom_language(self):
        with patch("integrations.openai_client.client") as mock_client:
            mock_transcription = MagicMock()
            mock_transcription.text = "English text"
            mock_client.audio.transcriptions.create.return_value = mock_transcription

            with patch("builtins.open", mock_open(read_data=b"audio data")):
                from integrations.openai_client import transcribe_audio

                result = transcribe_audio("/path/to/audio.mp3", language="en")
                assert result == "English text"

                # Verify language was passed
                call_kwargs = mock_client.audio.transcriptions.create.call_args.kwargs
                assert call_kwargs["language"] == "en"

    def test_text_to_speech(self):
        with patch("integrations.openai_client.client") as mock_client:
            mock_response = MagicMock()
            mock_response.stream_to_file = MagicMock()
            mock_client.audio.speech.create.return_value = mock_response

            from integrations.openai_client import text_to_speech

            result = text_to_speech("Hello world", "/tmp/output.mp3")
            assert result == "/tmp/output.mp3"
            mock_response.stream_to_file.assert_called_once_with("/tmp/output.mp3")

    def test_text_to_speech_custom_voice(self):
        with patch("integrations.openai_client.client") as mock_client:
            mock_response = MagicMock()
            mock_response.stream_to_file = MagicMock()
            mock_client.audio.speech.create.return_value = mock_response

            from integrations.openai_client import text_to_speech

            result = text_to_speech("Hello", "/tmp/out.mp3", voice="nova")
            assert result == "/tmp/out.mp3"

            # Verify voice was passed
            call_kwargs = mock_client.audio.speech.create.call_args.kwargs
            assert call_kwargs["voice"] == "nova"
