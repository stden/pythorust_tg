"""Tests for OpenAI client integration."""

from unittest.mock import MagicMock, patch

import pytest

from integrations.openai_client import (
    DEFAULT_EMBEDDING_MODEL,
    DEFAULT_MODEL,
    DEFAULT_TTS_MODEL,
    DEFAULT_TTS_VOICE,
    OpenAIClient,
    analyze_image,
    create_client,
    generate_speech,
    get_chat_response,
    get_embeddings,
    moderate_content,
    transcribe_audio,
)


class TestOpenAIClient:
    """Test OpenAI client class."""

    @pytest.fixture
    def mock_openai(self):
        """Mock OpenAI client."""
        with patch("openai.OpenAI") as mock_class:
            client = MagicMock()
            mock_class.return_value = client

            # Mock chat completions
            response = MagicMock()
            response.choices = [MagicMock()]
            response.choices[0].message.content = "Test response"
            response.usage.total_tokens = 100
            client.chat.completions.create = MagicMock(return_value=response)

            # Mock embeddings
            embedding_response = MagicMock()
            embedding_response.data = [MagicMock(embedding=[0.1, 0.2, 0.3])]
            client.embeddings.create = MagicMock(return_value=embedding_response)

            # Mock audio
            client.audio.transcriptions.create = MagicMock(return_value=MagicMock(text="Transcribed text"))
            client.audio.speech.create = MagicMock(return_value=MagicMock(content=b"audio_data"))

            # Mock moderations
            moderation_response = MagicMock()
            moderation_response.results = [
                MagicMock(
                    flagged=False,
                    categories=MagicMock(harassment=False, hate=False, self_harm=False, sexual=False, violence=False),
                )
            ]
            client.moderations.create = MagicMock(return_value=moderation_response)

            yield client

    def test_init(self, mock_env):
        """Test client initialization."""
        client = OpenAIClient()
        assert client.api_key == "test_openai_key"
        assert client.model == DEFAULT_MODEL

    def test_init_custom_params(self):
        """Test client with custom parameters."""
        client = OpenAIClient(api_key="custom_key", model="gpt-4", max_retries=5, timeout=120)
        assert client.api_key == "custom_key"
        assert client.model == "gpt-4"
        assert client.max_retries == 5
        assert client.timeout == 120

    def test_init_missing_api_key(self):
        """Test initialization without API key."""
        with pytest.raises(ValueError, match="API key not provided"):
            OpenAIClient(api_key="")

    def test_get_response(self, mock_openai):
        """Test getting chat response."""
        client = OpenAIClient(api_key="test_key")

        response = client.get_response("Hello")

        assert response == "Test response"
        mock_openai.chat.completions.create.assert_called_once()

    def test_get_response_with_params(self, mock_openai):
        """Test chat response with custom parameters."""
        client = OpenAIClient(api_key="test_key")

        response = client.get_response("Hello", temperature=0.5, max_tokens=100, system_message="You are helpful")

        assert response == "Test response"

        # Check call parameters
        call_args = mock_openai.chat.completions.create.call_args[1]
        assert call_args["temperature"] == 0.5
        assert call_args["max_tokens"] == 100
        assert len(call_args["messages"]) == 2
        assert call_args["messages"][0]["role"] == "system"

    def test_get_response_streaming(self, mock_openai):
        """Test streaming chat response."""
        # Mock streaming response
        chunks = [
            MagicMock(choices=[MagicMock(delta=MagicMock(content="Hello"))]),
            MagicMock(choices=[MagicMock(delta=MagicMock(content=" world"))]),
            MagicMock(choices=[MagicMock(delta=MagicMock(content=None))]),
        ]
        mock_openai.chat.completions.create.return_value = iter(chunks)

        client = OpenAIClient(api_key="test_key")
        result = list(client.get_response("Test", stream=True))

        assert result == ["Hello", " world"]

    def test_get_embeddings(self, mock_openai):
        """Test getting embeddings."""
        client = OpenAIClient(api_key="test_key")

        embeddings = client.get_embeddings("Test text")

        assert embeddings == [0.1, 0.2, 0.3]
        mock_openai.embeddings.create.assert_called_once_with(model=DEFAULT_EMBEDDING_MODEL, input="Test text")

    def test_get_embeddings_batch(self, mock_openai):
        """Test getting embeddings for multiple texts."""
        # Mock batch response
        mock_openai.embeddings.create.return_value.data = [
            MagicMock(embedding=[0.1, 0.2]),
            MagicMock(embedding=[0.3, 0.4]),
        ]

        client = OpenAIClient(api_key="test_key")

        embeddings = client.get_embeddings(["Text 1", "Text 2"])

        assert len(embeddings) == 2
        assert embeddings[0] == [0.1, 0.2]
        assert embeddings[1] == [0.3, 0.4]

    def test_transcribe_audio_from_file(self, mock_openai, tmp_path):
        """Test transcribing audio from file."""
        # Create temp audio file
        audio_file = tmp_path / "test.mp3"
        audio_file.write_bytes(b"fake_audio_data")

        client = OpenAIClient(api_key="test_key")

        result = client.transcribe_audio(str(audio_file))

        assert result == "Transcribed text"
        mock_openai.audio.transcriptions.create.assert_called_once()

    def test_transcribe_audio_from_bytes(self, mock_openai):
        """Test transcribing audio from bytes."""
        client = OpenAIClient(api_key="test_key")

        result = client.transcribe_audio(b"audio_data", format="mp3")

        assert result == "Transcribed text"

    def test_generate_speech(self, mock_openai):
        """Test generating speech."""
        client = OpenAIClient(api_key="test_key")

        audio_data = client.generate_speech("Hello world")

        assert audio_data == b"audio_data"
        mock_openai.audio.speech.create.assert_called_once_with(
            model=DEFAULT_TTS_MODEL, voice=DEFAULT_TTS_VOICE, input="Hello world"
        )

    def test_generate_speech_custom_params(self, mock_openai):
        """Test generating speech with custom parameters."""
        client = OpenAIClient(api_key="test_key")

        audio_data = client.generate_speech("Test", voice="nova", speed=1.5, response_format="mp3")

        assert audio_data == b"audio_data"

        call_args = mock_openai.audio.speech.create.call_args[1]
        assert call_args["voice"] == "nova"
        assert call_args["speed"] == 1.5
        assert call_args["response_format"] == "mp3"

    def test_analyze_image_url(self, mock_openai):
        """Test analyzing image from URL."""
        client = OpenAIClient(api_key="test_key")

        result = client.analyze_image("https://example.com/image.jpg", "What's in this image?")

        assert result == "Test response"

        # Check message format
        call_args = mock_openai.chat.completions.create.call_args[1]
        messages = call_args["messages"]
        assert messages[0]["role"] == "user"
        assert len(messages[0]["content"]) == 2
        assert messages[0]["content"][0]["type"] == "text"
        assert messages[0]["content"][1]["type"] == "image_url"

    def test_analyze_image_base64(self, mock_openai):
        """Test analyzing base64 image."""
        import base64

        client = OpenAIClient(api_key="test_key")
        image_data = base64.b64encode(b"fake_image").decode()

        result = client.analyze_image(f"data:image/jpeg;base64,{image_data}", "Describe this")

        assert result == "Test response"

    def test_analyze_image_file(self, mock_openai, tmp_path):
        """Test analyzing image from file."""
        # Create temp image file
        image_file = tmp_path / "test.jpg"
        image_file.write_bytes(b"fake_image_data")

        client = OpenAIClient(api_key="test_key")

        result = client.analyze_image(str(image_file), "What's this?")

        assert result == "Test response"

    def test_moderate_content(self, mock_openai):
        """Test content moderation."""
        client = OpenAIClient(api_key="test_key")

        result = client.moderate_content("Safe content")

        assert result["flagged"] is False
        assert result["categories"]["harassment"] is False
        mock_openai.moderations.create.assert_called_once_with(input="Safe content")

    def test_moderate_content_flagged(self, mock_openai):
        """Test moderation with flagged content."""
        # Mock flagged response
        mock_openai.moderations.create.return_value.results[0].flagged = True
        mock_openai.moderations.create.return_value.results[0].categories.harassment = True

        client = OpenAIClient(api_key="test_key")

        result = client.moderate_content("Bad content")

        assert result["flagged"] is True
        assert result["categories"]["harassment"] is True

    def test_error_handling(self, mock_openai):
        """Test error handling."""
        mock_openai.chat.completions.create.side_effect = Exception("API Error")

        client = OpenAIClient(api_key="test_key")

        with pytest.raises(Exception, match="API Error"):
            client.get_response("Test")


class TestModuleFunctions:
    """Test module-level convenience functions."""

    @patch("integrations.openai_client.OpenAIClient")
    def test_create_client(self, mock_client_class, mock_env):
        """Test create_client function."""
        create_client()

        mock_client_class.assert_called_once_with(api_key="test_openai_key", model=DEFAULT_MODEL)

    @patch("integrations.openai_client.OpenAIClient")
    def test_get_chat_response(self, mock_client_class):
        """Test get_chat_response function."""
        mock_instance = MagicMock()
        mock_instance.get_response.return_value = "Response"
        mock_client_class.return_value = mock_instance

        result = get_chat_response("Hello", api_key="test_key")

        assert result == "Response"
        mock_instance.get_response.assert_called_once_with(
            "Hello", temperature=0.7, max_tokens=None, system_message=None
        )

    @patch("integrations.openai_client.OpenAIClient")
    def test_get_embeddings_function(self, mock_client_class):
        """Test get_embeddings function."""
        mock_instance = MagicMock()
        mock_instance.get_embeddings.return_value = [0.1, 0.2]
        mock_client_class.return_value = mock_instance

        result = get_embeddings("Text", api_key="test_key")

        assert result == [0.1, 0.2]

    @patch("integrations.openai_client.OpenAIClient")
    def test_transcribe_audio_function(self, mock_client_class, tmp_path):
        """Test transcribe_audio function."""
        mock_instance = MagicMock()
        mock_instance.transcribe_audio.return_value = "Transcribed"
        mock_client_class.return_value = mock_instance

        audio_file = tmp_path / "test.mp3"
        audio_file.write_bytes(b"audio")

        result = transcribe_audio(str(audio_file), api_key="test_key")

        assert result == "Transcribed"

    @patch("integrations.openai_client.OpenAIClient")
    def test_generate_speech_function(self, mock_client_class):
        """Test generate_speech function."""
        mock_instance = MagicMock()
        mock_instance.generate_speech.return_value = b"audio"
        mock_client_class.return_value = mock_instance

        result = generate_speech("Hello", api_key="test_key", voice="nova")

        assert result == b"audio"
        mock_instance.generate_speech.assert_called_once_with("Hello", voice="nova", speed=1.0)

    @patch("integrations.openai_client.OpenAIClient")
    def test_analyze_image_function(self, mock_client_class):
        """Test analyze_image function."""
        mock_instance = MagicMock()
        mock_instance.analyze_image.return_value = "Description"
        mock_client_class.return_value = mock_instance

        result = analyze_image("image.jpg", "What's this?", api_key="test_key")

        assert result == "Description"

    @patch("integrations.openai_client.OpenAIClient")
    def test_moderate_content_function(self, mock_client_class):
        """Test moderate_content function."""
        mock_instance = MagicMock()
        mock_instance.moderate_content.return_value = {"flagged": False}
        mock_client_class.return_value = mock_instance

        result = moderate_content("Text", api_key="test_key")

        assert result == {"flagged": False}
