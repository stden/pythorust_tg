"""Tests for Yandex TTS integration."""

from datetime import datetime, timedelta
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from integrations.yandex_tts import (
    AudioFormat,
    SpeechRecognitionResult,
    SpeechSynthesisResult,
    Voice,
    YandexTTSClient,
    YandexTTSConfig,
    YandexTTSError,
)


class TestYandexTTSConfig:
    """Test YandexTTSConfig class."""

    def test_from_env_with_api_key(self, monkeypatch):
        """Test loading config with API key."""
        monkeypatch.setenv("YANDEX_API_KEY", "test_api_key")
        monkeypatch.setenv("YANDEX_FOLDER_ID", "test_folder")
        monkeypatch.delenv("YANDEX_IAM_TOKEN", raising=False)

        config = YandexTTSConfig.from_env()

        assert config.api_key == "test_api_key"
        assert config.folder_id == "test_folder"
        assert config.iam_token is None

    def test_from_env_with_iam_token(self, monkeypatch):
        """Test loading config with IAM token."""
        monkeypatch.setenv("YANDEX_IAM_TOKEN", "test_iam_token")
        monkeypatch.setenv("YANDEX_FOLDER_ID", "test_folder")
        monkeypatch.delenv("YANDEX_API_KEY", raising=False)

        config = YandexTTSConfig.from_env()

        assert config.api_key is None
        assert config.folder_id == "test_folder"
        assert config.iam_token == "test_iam_token"

    def test_from_env_missing_credentials(self, monkeypatch):
        """Test loading config without credentials."""
        monkeypatch.delenv("YANDEX_API_KEY", raising=False)
        monkeypatch.delenv("YANDEX_IAM_TOKEN", raising=False)
        monkeypatch.setenv("YANDEX_FOLDER_ID", "test_folder")

        with pytest.raises(ValueError, match="Either YANDEX_API_KEY or YANDEX_IAM_TOKEN"):
            YandexTTSConfig.from_env()


class TestYandexTTSClient:
    """Test YandexTTSClient class."""

    @pytest.fixture
    def config(self):
        """Create test config."""
        return YandexTTSConfig(api_key="test_api_key", folder_id="test_folder")

    @pytest.fixture
    def client(self, config):
        """Create test client."""
        return YandexTTSClient(config)

    @pytest.fixture
    def mock_httpx_client(self):
        """Mock httpx client."""
        with patch("integrations.yandex_tts.httpx.AsyncClient") as mock_class:
            client = AsyncMock()
            mock_class.return_value = client
            client.__aenter__.return_value = client
            client.__aexit__.return_value = None
            yield client

    @pytest.mark.asyncio
    async def test_get_iam_token_with_api_key(self, client, mock_httpx_client):
        """Test getting IAM token with API key."""
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {"iamToken": "new_iam_token", "expiresAt": "2025-12-31T23:59:59Z"}
        mock_httpx_client.post.return_value = mock_response

        token = await client._get_iam_token()

        assert token == "new_iam_token"
        assert client._iam_token == "new_iam_token"
        mock_httpx_client.post.assert_called_once()

    @pytest.mark.asyncio
    async def test_get_iam_token_cached(self, client):
        """Test using cached IAM token."""
        client._iam_token = "cached_token"
        client._token_expires_at = datetime.utcnow() + timedelta(hours=1)

        token = await client._get_iam_token()

        assert token == "cached_token"

    @pytest.mark.asyncio
    async def test_get_iam_token_error(self, client, mock_httpx_client):
        """Test error when getting IAM token."""
        mock_response = MagicMock()
        mock_response.status_code = 401
        mock_response.json.return_value = {"message": "Invalid API key"}
        mock_httpx_client.post.return_value = mock_response

        with pytest.raises(YandexTTSError, match="Failed to get IAM token"):
            await client._get_iam_token()

    @pytest.mark.asyncio
    async def test_synthesize_speech(self, client, mock_httpx_client):
        """Test speech synthesis."""
        # Mock IAM token
        client._iam_token = "test_token"
        client._token_expires_at = datetime.utcnow() + timedelta(hours=1)

        # Mock synthesis response
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.content = b"audio_data"
        mock_httpx_client.post.return_value = mock_response

        result = await client.synthesize_speech(text="Hello world", voice=Voice.ALENA, format=AudioFormat.MP3)

        assert isinstance(result, SpeechSynthesisResult)
        assert result.audio_content == b"audio_data"
        assert result.format == AudioFormat.MP3

        # Verify API call
        mock_httpx_client.post.assert_called_with(
            "https://tts.api.cloud.yandex.net/speech/v1/tts:synthesize",
            headers={"Authorization": "Bearer test_token"},
            data={"text": "Hello world", "voice": "alena", "format": "mp3", "folderId": "test_folder"},
        )

    @pytest.mark.asyncio
    async def test_synthesize_speech_with_params(self, client, mock_httpx_client):
        """Test speech synthesis with additional parameters."""
        client._iam_token = "test_token"
        client._token_expires_at = datetime.utcnow() + timedelta(hours=1)

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.content = b"audio_data"
        mock_httpx_client.post.return_value = mock_response

        result = await client.synthesize_speech(
            text="Test text", voice=Voice.FILIPP, format=AudioFormat.OGG_OPUS, speed=1.5, emotion="friendly"
        )

        assert result.audio_content == b"audio_data"

        # Verify parameters
        call_data = mock_httpx_client.post.call_args[1]["data"]
        assert call_data["speed"] == "1.5"
        assert call_data["emotion"] == "friendly"

    @pytest.mark.asyncio
    async def test_synthesize_speech_error(self, client, mock_httpx_client):
        """Test error in speech synthesis."""
        client._iam_token = "test_token"
        client._token_expires_at = datetime.utcnow() + timedelta(hours=1)

        mock_response = MagicMock()
        mock_response.status_code = 400
        mock_response.json.return_value = {"message": "Invalid text"}
        mock_httpx_client.post.return_value = mock_response

        with pytest.raises(YandexTTSError, match="Speech synthesis failed"):
            await client.synthesize_speech("", Voice.ALENA)

    @pytest.mark.asyncio
    async def test_recognize_speech(self, client, mock_httpx_client):
        """Test speech recognition."""
        client._iam_token = "test_token"
        client._token_expires_at = datetime.utcnow() + timedelta(hours=1)

        # Mock recognition response
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "result": "recognized text",
            "alternatives": [
                {"text": "recognized text", "confidence": 0.95},
                {"text": "alternative text", "confidence": 0.85},
            ],
        }
        mock_httpx_client.post.return_value = mock_response

        audio_data = b"test_audio_data"
        result = await client.recognize_speech(audio_content=audio_data, format=AudioFormat.OGG_OPUS)

        assert isinstance(result, SpeechRecognitionResult)
        assert result.text == "recognized text"
        assert len(result.alternatives) == 2
        assert result.alternatives[0]["confidence"] == 0.95

        # Verify API call
        mock_httpx_client.post.assert_called_with(
            "https://stt.api.cloud.yandex.net/speech/v1/stt:recognize",
            headers={"Authorization": "Bearer test_token", "Content-Type": "audio/ogg"},
            params={"format": "oggopus", "folderId": "test_folder", "lang": "ru-RU"},
            content=audio_data,
        )

    @pytest.mark.asyncio
    async def test_recognize_speech_with_params(self, client, mock_httpx_client):
        """Test speech recognition with additional parameters."""
        client._iam_token = "test_token"
        client._token_expires_at = datetime.utcnow() + timedelta(hours=1)

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {"result": "test"}
        mock_httpx_client.post.return_value = mock_response

        await client.recognize_speech(audio_content=b"audio", format=AudioFormat.PCM, language="en-US", model="general")

        # Verify parameters
        call_params = mock_httpx_client.post.call_args[1]["params"]
        assert call_params["lang"] == "en-US"
        assert call_params["model"] == "general"

    @pytest.mark.asyncio
    async def test_recognize_speech_error(self, client, mock_httpx_client):
        """Test error in speech recognition."""
        client._iam_token = "test_token"
        client._token_expires_at = datetime.utcnow() + timedelta(hours=1)

        mock_response = MagicMock()
        mock_response.status_code = 400
        mock_response.json.return_value = {"message": "Invalid audio format"}
        mock_httpx_client.post.return_value = mock_response

        with pytest.raises(YandexTTSError, match="Speech recognition failed"):
            await client.recognize_speech(b"invalid", AudioFormat.MP3)

    @pytest.mark.asyncio
    async def test_save_audio(self, client, tmp_path):
        """Test saving audio to file."""
        audio_data = b"test_audio_data"
        file_path = tmp_path / "test.mp3"

        await client.save_audio(audio_data, str(file_path))

        assert file_path.exists()
        assert file_path.read_bytes() == audio_data

    @pytest.mark.asyncio
    async def test_load_audio(self, client, tmp_path):
        """Test loading audio from file."""
        audio_data = b"test_audio_data"
        file_path = tmp_path / "test.mp3"
        file_path.write_bytes(audio_data)

        loaded_data = await client.load_audio(str(file_path))

        assert loaded_data == audio_data

    @pytest.mark.asyncio
    async def test_load_audio_not_found(self, client):
        """Test loading non-existent audio file."""
        with pytest.raises(FileNotFoundError):
            await client.load_audio("/non/existent/file.mp3")

    def test_format_conversion(self):
        """Test audio format conversion for API."""
        assert YandexTTSClient._get_api_format(AudioFormat.MP3) == "mp3"
        assert YandexTTSClient._get_api_format(AudioFormat.OGG_OPUS) == "oggopus"
        assert YandexTTSClient._get_api_format(AudioFormat.PCM) == "lpcm"

    def test_content_type_mapping(self):
        """Test content type mapping for formats."""
        assert YandexTTSClient._get_content_type(AudioFormat.MP3) == "audio/mp3"
        assert YandexTTSClient._get_content_type(AudioFormat.OGG_OPUS) == "audio/ogg"
        assert YandexTTSClient._get_content_type(AudioFormat.PCM) == "audio/x-pcm"


class TestVoiceEnum:
    """Test Voice enum."""

    def test_voice_values(self):
        """Test voice enum values."""
        assert Voice.ALENA.value == "alena"
        assert Voice.FILIPP.value == "filipp"
        assert Voice.ERMIL.value == "ermil"
        assert Voice.JANE.value == "jane"
        assert Voice.MADIRUS.value == "madirus"
        assert Voice.OMAZH.value == "omazh"
        assert Voice.ZAHAR.value == "zahar"


class TestAudioFormatEnum:
    """Test AudioFormat enum."""

    def test_format_values(self):
        """Test format enum values."""
        assert AudioFormat.MP3.value == "mp3"
        assert AudioFormat.OGG_OPUS.value == "oggopus"
        assert AudioFormat.PCM.value == "lpcm"
