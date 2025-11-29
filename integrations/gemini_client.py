"""
Google Gemini API клиент для интеграции с Google AI.

Поддерживает:
- Gemini 2.5 Flash / Pro
- Gemini 3 Pro (preview)
- Потоковые ответы (streaming)
- Vision (изображения)
- Генерация изображений (Nano Banana)

Пример использования:
    client = GeminiClient()
    response = await client.chat("Привет! Как дела?")
    print(response)

Переменные окружения:
    GOOGLE_API_KEY - API ключ от Google AI Studio
"""

import os
from dataclasses import dataclass, field
from typing import AsyncIterator, Optional

import httpx


@dataclass
class GeminiMessage:
    """Сообщение для Gemini API."""

    role: str  # "user" или "model"
    content: str


@dataclass
class GeminiResponse:
    """Ответ от Gemini API."""

    content: str
    model: str
    finish_reason: str
    prompt_tokens: int
    candidates_tokens: int


@dataclass
class GeminiClient:
    """
    Клиент для работы с Google Gemini API.

    Атрибуты:
        api_key: API ключ (по умолчанию из GOOGLE_API_KEY)
        model: Модель для использования
        temperature: Температура генерации (0.0-2.0)
        max_output_tokens: Максимальное количество токенов в ответе
    """

    api_key: str = field(default_factory=lambda: os.getenv("GOOGLE_API_KEY", ""))
    model: str = "gemini-2.0-flash"  # Актуальная стабильная модель
    temperature: float = 0.7
    max_output_tokens: int = 8192
    base_url: str = "https://generativelanguage.googleapis.com/v1beta"

    def __post_init__(self):
        if not self.api_key:
            raise ValueError(
                "GOOGLE_API_KEY не установлен. "
                "Получите ключ на https://aistudio.google.com/"
            )

    async def chat(
        self,
        message: str,
        system: Optional[str] = None,
        history: Optional[list[GeminiMessage]] = None,
    ) -> str:
        """
        Отправить сообщение и получить ответ.

        Args:
            message: Текст сообщения пользователя
            system: Системный промпт (опционально)
            history: История сообщений (опционально)

        Returns:
            Текст ответа от Gemini
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
        Отправить сообщение и получить полный ответ с метаданными.

        Args:
            message: Текст сообщения пользователя
            system: Системный промпт (опционально)
            history: История сообщений (опционально)

        Returns:
            GeminiResponse с контентом и метаданными
        """
        contents = []

        # Добавляем историю
        if history:
            for msg in history:
                contents.append(
                    {"role": msg.role, "parts": [{"text": msg.content}]}
                )

        # Добавляем текущее сообщение
        contents.append({"role": "user", "parts": [{"text": message}]})

        # Формируем запрос
        payload = {
            "contents": contents,
            "generationConfig": {
                "temperature": self.temperature,
                "maxOutputTokens": self.max_output_tokens,
            },
        }

        # Системный промпт
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

        # Извлекаем ответ
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
        Потоковый ответ от Gemini.

        Args:
            message: Текст сообщения
            system: Системный промпт (опционально)
            history: История сообщений (опционально)

        Yields:
            Частичные ответы по мере генерации
        """
        contents = []

        if history:
            for msg in history:
                contents.append(
                    {"role": msg.role, "parts": [{"text": msg.content}]}
                )

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
        Анализ изображения с помощью Gemini Vision.

        Args:
            image_data: Байты изображения
            prompt: Вопрос о изображении
            mime_type: MIME тип изображения
            system: Системный промпт (опционально)

        Returns:
            Ответ от Gemini
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
        Генерация изображения с помощью Imagen / Nano Banana.

        Args:
            prompt: Описание изображения
            aspect_ratio: Соотношение сторон (1:1, 16:9, 9:16, 4:3, 3:4)

        Returns:
            Байты сгенерированного изображения
        """
        # Используем модель для генерации изображений
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


# Доступные модели Gemini (ноябрь 2025)
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
    Быстрый чат с Gemini без создания клиента.

    Args:
        message: Сообщение пользователя
        model: Модель (по умолчанию gemini-2.5-flash)

    Returns:
        Ответ от Gemini
    """
    client = GeminiClient(model=GEMINI_MODELS.get(model, model))
    return await client.chat(message)


if __name__ == "__main__":
    import asyncio

    async def main():
        # Пример использования
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
