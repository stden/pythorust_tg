"""
Reusable module for recognizing voice messages through OpenAI Whisper.

Usage:
    from voice_utils import VoiceRecognizer

    recognizer = VoiceRecognizer()
    text = await recognizer.transcribe_file("/path/to/audio.ogg")
    text = await recognizer.transcribe_bytes(audio_bytes, "ogg")
"""

import os
import logging
import tempfile
from typing import Optional, Tuple
from pathlib import Path

try:
    import openai
    from openai import AsyncOpenAI

    OPENAI_AVAILABLE = True
except ImportError:
    OPENAI_AVAILABLE = False
    logging.warning("openai package not installed. Install with: pip install openai")

# Audio formats supported by Whisper.
SUPPORTED_FORMATS = {"mp3", "mp4", "mpeg", "mpga", "m4a", "wav", "webm", "ogg", "oga", "flac"}


class VoiceRecognizer:
    """
    Reusable class for recognizing voice messages.
    Supports the OpenAI Whisper API.
    """

    def __init__(
        self,
        api_key: Optional[str] = None,
        model: str = "whisper-1",
        language: Optional[str] = None,
        prompt: Optional[str] = None,
    ):
        """
        Initialize the recognizer.

        Args:
            api_key: OpenAI API key (defaults to OPENAI_API_KEY)
            model: Whisper model (defaults to whisper-1)
            language: Language code (ru, en, uz, etc.) or None for auto-detection
            prompt: Hint to improve recognition
        """
        if not OPENAI_AVAILABLE:
            raise ImportError("openai package required. Install with: pip install openai")

        self.api_key = api_key or os.environ.get("OPENAI_API_KEY")
        if not self.api_key:
            raise ValueError("OPENAI_API_KEY not set. Provide api_key or set environment variable.")

        self.model = model
        self.language = language
        self.prompt = prompt
        self.client = AsyncOpenAI(api_key=self.api_key)

        logging.info(f"VoiceRecognizer initialized with model={model}, language={language}")

    async def transcribe_file(
        self, file_path: str, language: Optional[str] = None, prompt: Optional[str] = None
    ) -> str:
        """
        Recognize audio from a file.

        Args:
            file_path: Path to the audio file
            language: Language code (overrides the default setting)
            prompt: Hint (overrides the default setting)

        Returns:
            Recognized text
        """
        path = Path(file_path)
        if not path.exists():
            raise FileNotFoundError(f"Audio file not found: {file_path}")

        suffix = path.suffix.lstrip(".").lower()
        if suffix not in SUPPORTED_FORMATS:
            logging.warning(f"Format '{suffix}' may not be supported. Supported: {SUPPORTED_FORMATS}")

        with open(file_path, "rb") as audio_file:
            return await self._transcribe(audio_file, language, prompt)

    async def transcribe_bytes(
        self, audio_data: bytes, format: str = "ogg", language: Optional[str] = None, prompt: Optional[str] = None
    ) -> str:
        """
        Recognize audio from bytes.

        Args:
            audio_data: Audio bytes
            format: File format (ogg, mp3, wav, etc.)
            language: Language code
            prompt: Hint

        Returns:
            Recognized text
        """
        format = format.lower().lstrip(".")
        if format not in SUPPORTED_FORMATS:
            logging.warning(f"Format '{format}' may not be supported. Supported: {SUPPORTED_FORMATS}")

        # Create a temporary file because the Whisper API requires a file.
        with tempfile.NamedTemporaryFile(suffix=f".{format}", delete=False) as tmp:
            tmp.write(audio_data)
            tmp_path = tmp.name

        try:
            with open(tmp_path, "rb") as audio_file:
                return await self._transcribe(audio_file, language, prompt)
        finally:
            os.unlink(tmp_path)

    async def _transcribe(self, audio_file, language: Optional[str], prompt: Optional[str]) -> str:
        """Internal transcription method."""
        lang = language or self.language
        prm = prompt or self.prompt

        kwargs = {"model": self.model, "file": audio_file}
        if lang:
            kwargs["language"] = lang
        if prm:
            kwargs["prompt"] = prm

        try:
            result = await self.client.audio.transcriptions.create(**kwargs)
            text = result.text.strip()
            logging.info(f"Transcribed {len(text)} chars, language={lang}")
            return text
        except openai.APIError as e:
            logging.error(f"OpenAI API error: {e}")
            raise
        except Exception as e:
            logging.error(f"Transcription error: {e}")
            raise


class TelegramVoiceHandler:
    """
    Voice message handler for Telegram bots.
    Supports aiogram, python-telegram-bot, and telethon.
    """

    def __init__(self, recognizer: Optional[VoiceRecognizer] = None, **kwargs):
        """
        Args:
            recognizer: VoiceRecognizer instance or None to create a new one
            **kwargs: Arguments for VoiceRecognizer when recognizer=None
        """
        self.recognizer = recognizer or VoiceRecognizer(**kwargs)

    async def process_voice_aiogram(self, message, bot) -> Tuple[str, float]:
        """
        Process a voice message for aiogram.

        Args:
            message: aiogram Message object
            bot: aiogram Bot object

        Returns:
            Tuple[text, duration_seconds]
        """
        voice = message.voice
        if not voice:
            raise ValueError("Message has no voice")

        # Download the file.
        file = await bot.get_file(voice.file_id)
        with tempfile.NamedTemporaryFile(suffix=".ogg", delete=False) as tmp:
            await bot.download_file(file.file_path, tmp.name)
            tmp_path = tmp.name

        try:
            text = await self.recognizer.transcribe_file(tmp_path)
            return text, voice.duration
        finally:
            os.unlink(tmp_path)

    async def process_voice_ptb(self, update, context) -> Tuple[str, float]:
        """
        Process a voice message for python-telegram-bot.

        Args:
            update: PTB Update object
            context: PTB Context object

        Returns:
            Tuple[text, duration_seconds]
        """
        voice = update.message.voice
        if not voice:
            raise ValueError("Message has no voice")

        # Download the file.
        file = await context.bot.get_file(voice.file_id)
        with tempfile.NamedTemporaryFile(suffix=".ogg", delete=False) as tmp:
            await file.download_to_drive(tmp.name)
            tmp_path = tmp.name

        try:
            text = await self.recognizer.transcribe_file(tmp_path)
            return text, voice.duration
        finally:
            os.unlink(tmp_path)

    async def process_audio_bytes(self, audio_data: bytes, format: str = "ogg") -> str:
        """
        Process audio from bytes (generic method).

        Args:
            audio_data: Audio bytes
            format: File format

        Returns:
            Recognized text
        """
        return await self.recognizer.transcribe_bytes(audio_data, format)


# Convenience functions for quick use.
_default_recognizer: Optional[VoiceRecognizer] = None


def get_recognizer(**kwargs) -> VoiceRecognizer:
    """Get or create the global VoiceRecognizer instance."""
    global _default_recognizer
    if _default_recognizer is None:
        _default_recognizer = VoiceRecognizer(**kwargs)
    return _default_recognizer


async def transcribe(file_path_or_bytes, format: str = "ogg", **kwargs) -> str:
    """
    Quick transcription function.

    Args:
        file_path_or_bytes: Path to a file or audio bytes
        format: Format (for bytes)
        **kwargs: Arguments for VoiceRecognizer

    Returns:
        Recognized text
    """
    recognizer = get_recognizer(**kwargs)

    if isinstance(file_path_or_bytes, bytes):
        return await recognizer.transcribe_bytes(file_path_or_bytes, format)
    else:
        return await recognizer.transcribe_file(str(file_path_or_bytes))


# Usage example.
if __name__ == "__main__":
    import asyncio

    async def main():
        # Create a recognizer.
        VoiceRecognizer(language="ru")

        # Test with a file.
        # text = await recognizer.transcribe_file("/path/to/audio.ogg")
        # print(f"Recognized: {text}")

        print("VoiceRecognizer ready!")
        print(f"Supported formats: {SUPPORTED_FORMATS}")

    asyncio.run(main())
