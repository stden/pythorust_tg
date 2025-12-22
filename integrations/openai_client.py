"""
Async OpenAI helpers used by bots (BFL Sales, Credit Expert, Task Assistant).
"""

import os
from typing import Dict, List, Optional

from dotenv import load_dotenv
from openai import AsyncOpenAI, OpenAI

# Load environment once so local `.env` values are available
load_dotenv()

DEFAULT_MODEL = os.getenv("OPENAI_MODEL") or os.getenv("AI_CONSULTANT_MODEL") or "gpt-5.2-2025-12-11"
DEFAULT_TEMPERATURE = float(os.getenv("OPENAI_TEMPERATURE", "0.7"))
DEFAULT_TIMEOUT = float(os.getenv("OPENAI_TIMEOUT", "30"))


class OpenAIClient:
    """Async OpenAI client returning the raw completion response."""

    def __init__(
        self,
        api_key: Optional[str] = None,
        model: Optional[str] = None,
        temperature: Optional[float] = None,
        timeout: float = DEFAULT_TIMEOUT,
    ):
        self.api_key = api_key or os.getenv("OPENAI_API_KEY")
        if not self.api_key:
            raise ValueError("OPENAI_API_KEY is required for OpenAIClient")

        self.model = model or DEFAULT_MODEL
        self.temperature = DEFAULT_TEMPERATURE if temperature is None else temperature
        self.client = AsyncOpenAI(api_key=self.api_key, timeout=timeout)

    async def chat_completion(
        self,
        messages: List[Dict[str, str]],
        model: Optional[str] = None,
        temperature: Optional[float] = None,
        **kwargs,
    ):
        """Call OpenAI chat completion and return the response object."""
        return await self.client.chat.completions.create(
            model=model or self.model,
            messages=messages,
            temperature=self.temperature if temperature is None else temperature,
            **kwargs,
        )


async def chat_completion(
    messages: List[Dict[str, str]],
    model: Optional[str] = None,
    temperature: Optional[float] = None,
    max_tokens: Optional[int] = 1000,
    api_key: Optional[str] = None,
    timeout: Optional[float] = None,
    **kwargs,
) -> str:
    """Convenience helper that returns only the assistant text."""
    client = OpenAIClient(
        api_key=api_key,
        model=model or DEFAULT_MODEL,
        temperature=temperature if temperature is not None else DEFAULT_TEMPERATURE,
        timeout=timeout or DEFAULT_TIMEOUT,
    )
    response = await client.chat_completion(
        messages=messages,
        max_tokens=max_tokens,
        **kwargs,
    )
    return response.choices[0].message.content


async def sales_agent_response(user_message: str, context: str = "") -> str:
    """Generate a short sales response using the shared helper."""
    system_prompt = """Ты — профессиональный продавец-консультант.
Твоя задача — помочь клиенту и убедить его в ценности продукта.
Будь вежливым, но настойчивым. Отвечай кратко (1-3 предложения).
Используй техники продаж: SPIN, AIDA.
Если клиент возражает — обрабатывай возражения.
Если клиент согласен — закрывай сделку."""

    if context:
        system_prompt += f"\n\nКонтекст: {context}"

    messages = [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_message},
    ]

    return await chat_completion(messages, temperature=0.8)


def _sync_client(api_key: Optional[str] = None) -> OpenAI:
    key = api_key or os.getenv("OPENAI_API_KEY")
    if not key:
        raise ValueError("OPENAI_API_KEY is required for audio helpers")
    return OpenAI(api_key=key)


def transcribe_audio(audio_file_path: str, language: str = "ru", api_key: Optional[str] = None) -> str:
    """Transcribe audio using the Whisper API."""
    client = _sync_client(api_key)
    with open(audio_file_path, "rb") as audio_file:
        transcription = client.audio.transcriptions.create(
            model="whisper-1",
            file=audio_file,
            language=language,
        )
    return transcription.text


def text_to_speech(text: str, output_path: str, voice: str = "alloy", api_key: Optional[str] = None) -> str:
    """Convert text to speech using the TTS API."""
    client = _sync_client(api_key)
    response = client.audio.speech.create(
        model="tts-1",
        voice=voice,
        input=text,
    )

    response.stream_to_file(output_path)
    return output_path
