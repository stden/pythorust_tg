"""
Google Gemini API client for integration with Google AI.

Supports:
- Gemini 2.5 Flash / Pro
- Gemini 3 Pro (preview)
- Streaming responses
- Vision (images)
- Image generation (Nano Banana)

Usage example:
    client = GeminiClient()
    response = await client.chat("Hello! How are you?")
    print(response)

Environment variables:
    GOOGLE_API_KEY - API key from Google AI Studio
"""

import os
from dataclasses import dataclass, field
from typing import AsyncIterator, Optional

import httpx


@dataclass
class GeminiMessage:
    """Message for the Gemini API."""

    role: str  # "user" or "model"
    content: str


@dataclass
class GeminiResponse:
    """Response from the Gemini API."""

    content: str
    model: str
    finish_reason: str
    prompt_tokens: int
    candidates_tokens: int


@dataclass
class GeminiClient:
    """
    Client for working with the Google Gemini API.

    Attributes:
        api_key: API key (defaults to GOOGLE_API_KEY)
        model: Model to use
        temperature: Generation temperature (0.0-2.0)
        max_output_tokens: Maximum number of response tokens
    """

    api_key: str = field(default_factory=lambda: os.getenv("GOOGLE_API_KEY", ""))
    model: str = "gemini-2.0-flash"  # Current stable model.
    temperature: float = 0.7
    max_output_tokens: int = 8192
    base_url: str = "https://generativelanguage.googleapis.com/v1beta"

    def __post_init__(self):
        if not self.api_key:
            raise ValueError("GOOGLE_API_KEY не установлен. Получите ключ на https://aistudio.google.com/")

    async def chat(
        self,
        message: str,
        system: Optional[str] = None,
        history: Optional[list[GeminiMessage]] = None,
    ) -> str:
        """
        Send a message and get a response.

        Args:
            message: User message text
            system: System prompt (optional)
            history: Message history (optional)

        Returns:
            Gemini response text
        """
        response = await self.chat_full(message, system, history)
        return response.content

    async def chat_full(
        self,
        message: str,
        system: Optional[str] = None,
        history: Optional[list[GeminiMessage]] = None,
    ) -> GeminiResponse:
        """
        Send a message and get a full response with metadata.

        Args:
            message: User message text
            system: System prompt (optional)
            history: Message history (optional)

        Returns:
            GeminiResponse with content and metadata
        """
        contents = []

        # Add history.
        if history:
            for msg in history:
                contents.append({"role": msg.role, "parts": [{"text": msg.content}]})

        # Add the current message.
        contents.append({"role": "user", "parts": [{"text": message}]})

        # Build the request.
        payload = {
            "contents": contents,
            "generationConfig": {
                "temperature": self.temperature,
                "maxOutputTokens": self.max_output_tokens,
            },
        }

        # System prompt.
        if system:
            payload["systemInstruction"] = {"parts": [{"text": system}]}

        url = f"{self.base_url}/models/{self.model}:generateContent"

        async with httpx.AsyncClient(timeout=120.0) as client:
            response = await client.post(
                url,
                params={"key": self.api_key},
                json=payload,
            )
            response.raise_for_status()
            data = response.json()

        # Extract the response.
        candidate = data["candidates"][0]
        content = candidate["content"]["parts"][0]["text"]
        usage = data.get("usageMetadata", {})

        return GeminiResponse(
            content=content,
            model=self.model,
            finish_reason=candidate.get("finishReason", "STOP"),
            prompt_tokens=usage.get("promptTokenCount", 0),
            candidates_tokens=usage.get("candidatesTokenCount", 0),
        )

    async def chat_stream(
        self,
        message: str,
        system: Optional[str] = None,
        history: Optional[list[GeminiMessage]] = None,
    ) -> AsyncIterator[str]:
        """
        Streaming response from Gemini.

        Args:
            message: Message text
            system: System prompt (optional)
            history: Message history (optional)

        Yields:
            Partial responses as they are generated
        """
        contents = []

        if history:
            for msg in history:
                contents.append({"role": msg.role, "parts": [{"text": msg.content}]})

        contents.append({"role": "user", "parts": [{"text": message}]})

        payload = {
            "contents": contents,
            "generationConfig": {
                "temperature": self.temperature,
                "maxOutputTokens": self.max_output_tokens,
            },
        }

        if system:
            payload["systemInstruction"] = {"parts": [{"text": system}]}

        url = f"{self.base_url}/models/{self.model}:streamGenerateContent"

        async with httpx.AsyncClient(timeout=120.0) as client:
            async with client.stream(
                "POST",
                url,
                params={"key": self.api_key, "alt": "sse"},
                json=payload,
            ) as response:
                response.raise_for_status()
                async for line in response.aiter_lines():
                    if line.startswith("data: "):
                        import json

                        data = json.loads(line[6:])
                        if "candidates" in data:
                            parts = data["candidates"][0]["content"]["parts"]
                            if parts and "text" in parts[0]:
                                yield parts[0]["text"]

    async def analyze_image(
        self,
        image_data: bytes,
        prompt: str,
        mime_type: str = "image/jpeg",
        system: Optional[str] = None,
    ) -> str:
        """
        Analyze an image with Gemini Vision.

        Args:
            image_data: Image bytes
            prompt: Question about the image
            mime_type: Image MIME type
            system: System prompt (optional)

        Returns:
            Gemini response
        """
        import base64

        image_base64 = base64.b64encode(image_data).decode("utf-8")

        contents = [
            {
                "role": "user",
                "parts": [
                    {"inlineData": {"mimeType": mime_type, "data": image_base64}},
                    {"text": prompt},
                ],
            }
        ]

        payload = {
            "contents": contents,
            "generationConfig": {
                "temperature": self.temperature,
                "maxOutputTokens": self.max_output_tokens,
            },
        }

        if system:
            payload["systemInstruction"] = {"parts": [{"text": system}]}

        url = f"{self.base_url}/models/{self.model}:generateContent"

        async with httpx.AsyncClient(timeout=120.0) as client:
            response = await client.post(
                url,
                params={"key": self.api_key},
                json=payload,
            )
            response.raise_for_status()
            data = response.json()

        return data["candidates"][0]["content"]["parts"][0]["text"]

    async def generate_image(
        self,
        prompt: str,
        aspect_ratio: str = "1:1",
    ) -> bytes:
        """
        Generate an image with Imagen / Nano Banana.

        Args:
            prompt: Image description
            aspect_ratio: Aspect ratio (1:1, 16:9, 9:16, 4:3, 3:4)

        Returns:
            Generated image bytes
        """
        # Use the image generation model.
        image_model = "imagen-3.0-generate-002"

        payload = {
            "instances": [{"prompt": prompt}],
            "parameters": {
                "aspectRatio": aspect_ratio,
                "sampleCount": 1,
            },
        }

        url = f"{self.base_url}/models/{image_model}:predict"

        async with httpx.AsyncClient(timeout=180.0) as client:
            response = await client.post(
                url,
                params={"key": self.api_key},
                json=payload,
            )
            response.raise_for_status()
            data = response.json()

        import base64

        image_base64 = data["predictions"][0]["bytesBase64Encoded"]
        return base64.b64decode(image_base64)


# Available Gemini models (November 2025).
GEMINI_MODELS = {
    "gemini-2.0-flash": "gemini-2.0-flash",
    "gemini-2.0-flash-lite": "gemini-2.0-flash-lite",
    "gemini-2.5-flash": "gemini-2.5-flash",
    "gemini-2.5-flash-lite": "gemini-2.5-flash-lite",
    "gemini-2.5-pro": "gemini-2.5-pro",
    "gemini-3-pro": "gemini-3.0-pro",  # Latest
}


async def quick_chat(message: str, model: str = "gemini-2.5-flash") -> str:
    """
    Quick Gemini chat without creating a client.

    Args:
        message: User message
        model: Model (defaults to gemini-2.5-flash)

    Returns:
        Gemini response
    """
    client = GeminiClient(model=GEMINI_MODELS.get(model, model))
    return await client.chat(message)


if __name__ == "__main__":
    import asyncio

    async def main():
        # Usage example.
        try:
            client = GeminiClient()
            response = await client.chat(
                "Напиши короткое приветствие на русском языке.",
                system="Ты дружелюбный AI-ассистент.",
            )
            print(f"Gemini: {response}")
        except ValueError as e:
            print(f"Ошибка: {e}")
            print("Установите GOOGLE_API_KEY для использования Gemini API")

    asyncio.run(main())
