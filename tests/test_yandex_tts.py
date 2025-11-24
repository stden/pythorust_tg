# -*- coding: utf-8 -*-
"""Tests for Yandex TTS client."""
import os
import pytest
from unittest.mock import MagicMock, patch, mock_open


class TestYandexTTS:
    """Tests for Yandex SpeechKit functions."""

    def test_get_headers_with_iam_token(self):
        with patch.dict(os.environ, {
            "YANDEX_IAM_TOKEN": "test-iam-token",
            "YANDEX_API_KEY": "",
        }):
            # Re-import to get updated env vars
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            headers = yandex_module._get_headers()
            assert headers == {"Authorization": "Bearer test-iam-token"}

    def test_get_headers_with_api_key(self):
        with patch.dict(os.environ, {
            "YANDEX_IAM_TOKEN": "",
            "YANDEX_API_KEY": "test-api-key",
        }):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            headers = yandex_module._get_headers()
            assert headers == {"Authorization": "Api-Key test-api-key"}

    def test_get_headers_no_credentials(self):
        with patch.dict(os.environ, {
            "YANDEX_IAM_TOKEN": "",
            "YANDEX_API_KEY": "",
        }):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with pytest.raises(ValueError, match="YANDEX_API_KEY"):
                yandex_module._get_headers()

    def test_text_to_speech_success(self):
        with patch.dict(os.environ, {"YANDEX_API_KEY": "test-key", "YANDEX_FOLDER_ID": "folder123"}):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with patch("integrations.yandex_tts.requests") as mock_requests:
                mock_response = MagicMock()
                mock_response.status_code = 200
                mock_response.content = b"audio data"
                mock_requests.post.return_value = mock_response

                m = mock_open()
                with patch("builtins.open", m):
                    result = yandex_module.text_to_speech(
                        "Hello world",
                        "/tmp/output.mp3"
                    )
                    assert result == "/tmp/output.mp3"
                    m.assert_called_once_with("/tmp/output.mp3", "wb")

    def test_text_to_speech_custom_params(self):
        with patch.dict(os.environ, {"YANDEX_API_KEY": "test-key", "YANDEX_FOLDER_ID": "folder123"}):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with patch("integrations.yandex_tts.requests") as mock_requests:
                mock_response = MagicMock()
                mock_response.status_code = 200
                mock_response.content = b"audio"
                mock_requests.post.return_value = mock_response

                m = mock_open()
                with patch("builtins.open", m):
                    result = yandex_module.text_to_speech(
                        "Test",
                        "/tmp/out.mp3",
                        voice="filipp",
                        emotion="good",
                        speed=1.5,
                        format="oggopus"
                    )
                    assert result == "/tmp/out.mp3"

                    call_kwargs = mock_requests.post.call_args.kwargs
                    assert call_kwargs["data"]["voice"] == "filipp"
                    assert call_kwargs["data"]["emotion"] == "good"
                    assert call_kwargs["data"]["speed"] == "1.5"
                    assert call_kwargs["data"]["format"] == "oggopus"

    def test_text_to_speech_error(self):
        with patch.dict(os.environ, {"YANDEX_API_KEY": "test-key", "YANDEX_FOLDER_ID": "folder123"}):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with patch("integrations.yandex_tts.requests") as mock_requests:
                mock_response = MagicMock()
                mock_response.status_code = 400
                mock_response.text = "Bad request"
                mock_requests.post.return_value = mock_response

                with pytest.raises(Exception, match="Yandex TTS error"):
                    yandex_module.text_to_speech("Test", "/tmp/out.mp3")

    def test_speech_to_text_success(self):
        with patch.dict(os.environ, {"YANDEX_API_KEY": "test-key", "YANDEX_FOLDER_ID": "folder123"}):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with patch("integrations.yandex_tts.requests") as mock_requests:
                mock_response = MagicMock()
                mock_response.status_code = 200
                mock_response.json.return_value = {"result": "Recognized text"}
                mock_requests.post.return_value = mock_response

                with patch("builtins.open", mock_open(read_data=b"audio data")):
                    result = yandex_module.speech_to_text("/tmp/audio.ogg")
                    assert result == "Recognized text"

    def test_speech_to_text_custom_params(self):
        with patch.dict(os.environ, {"YANDEX_API_KEY": "test-key", "YANDEX_FOLDER_ID": "folder123"}):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with patch("integrations.yandex_tts.requests") as mock_requests:
                mock_response = MagicMock()
                mock_response.status_code = 200
                mock_response.json.return_value = {"result": "Numbers"}
                mock_requests.post.return_value = mock_response

                with patch("builtins.open", mock_open(read_data=b"audio")):
                    result = yandex_module.speech_to_text(
                        "/tmp/audio.ogg",
                        language="en-US",
                        topic="numbers"
                    )
                    assert result == "Numbers"

                    call_kwargs = mock_requests.post.call_args.kwargs
                    assert call_kwargs["params"]["lang"] == "en-US"
                    assert call_kwargs["params"]["topic"] == "numbers"

    def test_speech_to_text_error(self):
        with patch.dict(os.environ, {"YANDEX_API_KEY": "test-key", "YANDEX_FOLDER_ID": "folder123"}):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with patch("integrations.yandex_tts.requests") as mock_requests:
                mock_response = MagicMock()
                mock_response.status_code = 401
                mock_response.text = "Unauthorized"
                mock_requests.post.return_value = mock_response

                with patch("builtins.open", mock_open(read_data=b"audio")):
                    with pytest.raises(Exception, match="Yandex STT error"):
                        yandex_module.speech_to_text("/tmp/audio.ogg")

    def test_text_to_speech_ssml(self):
        with patch.dict(os.environ, {"YANDEX_API_KEY": "test-key", "YANDEX_FOLDER_ID": "folder123"}):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with patch("integrations.yandex_tts.requests") as mock_requests:
                mock_response = MagicMock()
                mock_response.status_code = 200
                mock_response.content = b"ssml audio"
                mock_requests.post.return_value = mock_response

                m = mock_open()
                with patch("builtins.open", m):
                    result = yandex_module.text_to_speech_ssml(
                        "<speak>Hello <break time='500ms'/> World</speak>",
                        "/tmp/ssml.mp3"
                    )
                    assert result == "/tmp/ssml.mp3"

                    call_kwargs = mock_requests.post.call_args.kwargs
                    assert "ssml" in call_kwargs["data"]

    def test_text_to_speech_ssml_error(self):
        with patch.dict(os.environ, {"YANDEX_API_KEY": "test-key", "YANDEX_FOLDER_ID": "folder123"}):
            import importlib
            import integrations.yandex_tts as yandex_module
            importlib.reload(yandex_module)

            with patch("integrations.yandex_tts.requests") as mock_requests:
                mock_response = MagicMock()
                mock_response.status_code = 400
                mock_response.text = "Invalid SSML"
                mock_requests.post.return_value = mock_response

                with pytest.raises(Exception, match="Yandex TTS error"):
                    yandex_module.text_to_speech_ssml("<speak>Bad</speak>", "/tmp/out.mp3")

    def test_list_voices(self, capsys):
        from integrations.yandex_tts import list_voices, VOICES_RU

        list_voices()
        captured = capsys.readouterr()

        assert "alena" in captured.out
        assert "filipp" in captured.out


class TestVoicesRU:
    """Tests for VOICES_RU constant."""

    def test_voices_dict_exists(self):
        from integrations.yandex_tts import VOICES_RU
        assert isinstance(VOICES_RU, dict)
        assert len(VOICES_RU) > 0

    def test_alena_voice_exists(self):
        from integrations.yandex_tts import VOICES_RU
        assert "alena" in VOICES_RU

    def test_filipp_voice_exists(self):
        from integrations.yandex_tts import VOICES_RU
        assert "filipp" in VOICES_RU
