"""
Claude API клиент для интеграции с Anthropic Claude.

Поддерживает:
- Claude 3.5 Sonnet / Haiku / Opus
- Claude 4 Sonnet (claude-sonnet-4-5-20250929)
- Потоковые ответы (streaming)
- Vision (изображения)
- Системные промпты

Пример использования:
    client = ClaudeClient()
    response = await client.chat("Привет! Как дела?")
    print(response)

Переменные окружения:
    ANTHROPIC_API_KEY - API ключ от Anthropic
"""

import os
from dataclasses import dataclass, field
from typing import AsyncIterator, Optional

import httpx


@dataclass
class ClaudeMessage:
    """Сообщение для Claude API."""

    role: str  # "user" или "assistant"
    content: str


@dataclass
class ClaudeResponse:
    """Ответ от Claude API."""

    content: str
    model: str
    input_tokens: int
    output_tokens: int
    stop_reason: str


@dataclass
class ClaudeClient:
    """
    Клиент для работы с Anthropic Claude API.

    Атрибуты:
        api_key: API ключ (по умолчанию из ANTHROPIC_API_KEY)
        model: Модель для использования
        max_tokens: Максимальное количество токенов в ответе
        temperature: Температура генерации (0.0-1.0)
    """

    api_key: str = field(default_factory=lambda: os.getenv("ANTHROPIC_API_KEY", ""))
    model: str = "claude-sonnet-4-5-20250929"  # Последняя модель Claude
    max_tokens: int = 4096
    temperature: float = 0.7
    base_url: str = "https://api.anthropic.com/v1"

    def __post_init__(self):
        if not self.api_key:
            raise ValueError("ANTHROPIC_API_KEY не установлен. Получите ключ на https://console.anthropic.com/")

    def _get_headers(self) -> dict:
        """Заголовки для API запросов."""
        return {
            "x-api-key": self.api_key,
            "anthropic-version": "2023-06-01",
            "content-type": "application/json",
        }

    async def chat(
        self,
        message: str,
        system: Optional[str] = None,
        history: Optional[list[ClaudeMessage]] = None,
    ) -> str:
        """
        Отправить сообщение и получить ответ.

        Args:
            message: Текст сообщения пользователя
            system: Системный промпт (опционально)
            history: История сообщений (опционально)

        Returns:
            Текст ответа от Claude
        """
        response = await self.chat_full(message, system, history)
        return response.content

    async def chat_full(
        self,
        message: str,
        system: Optional[str] = None,
        history: Optional[list[ClaudeMessage]] = None,
    ) -> ClaudeResponse:
        """
        Отправить сообщение и получить полный ответ с метаданными.

        Args:
            message: Текст сообщения пользователя
            system: Системный промпт (опционально)
            history: История сообщений (опционально)

        Returns:
            ClaudeResponse с контентом и метаданными
        """
        messages = []

        # Добавляем историю
        if history:
            for msg in history:
                messages.append({"role": msg.role, "content": msg.content})

        # Добавляем текущее сообщение
        messages.append({"role": "user", "content": message})

        # Формируем запрос
        payload = {
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "messages": messages,
        }

        if system:
            payload["system"] = system

        async with httpx.AsyncClient(timeout=120.0) as client:
            response = await client.post(
                f"{self.base_url}/messages",
                headers=self._get_headers(),
                json=payload,
            )
            response.raise_for_status()
            data = response.json()

        return ClaudeResponse(
            content=data["content"][0]["text"],
            model=data["model"],
            input_tokens=data["usage"]["input_tokens"],
            output_tokens=data["usage"]["output_tokens"],
            stop_reason=data["stop_reason"],
        )

    async def chat_stream(
        self,
        message: str,
        system: Optional[str] = None,
        history: Optional[list[ClaudeMessage]] = None,
    ) -> AsyncIterator[str]:
        """
        Потоковый ответ от Claude (Server-Sent Events).

        Args:
            message: Текст сообщения
            system: Системный промпт (опционально)
            history: История сообщений (опционально)

        Yields:
            Частичные ответы по мере генерации
        """
        messages = []

        if history:
            for msg in history:
                messages.append({"role": msg.role, "content": msg.content})

        messages.append({"role": "user", "content": message})

        payload = {
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "messages": messages,
            "stream": True,
        }

        if system:
            payload["system"] = system

        async with httpx.AsyncClient(timeout=120.0) as client:
            async with client.stream(
                "POST",
                f"{self.base_url}/messages",
                headers=self._get_headers(),
                json=payload,
            ) as response:
                response.raise_for_status()
                async for line in response.aiter_lines():
                    if line.startswith("data: "):
                        import json

                        data = json.loads(line[6:])
                        if data["type"] == "content_block_delta":
                            yield data["delta"]["text"]

    async def analyze_image(
        self,
        image_url: str,
        prompt: str,
        system: Optional[str] = None,
    ) -> str:
        """
        Анализ изображения с помощью Claude Vision.

        Args:
            image_url: URL изображения или base64
            prompt: Вопрос о изображении
            system: Системный промпт (опционально)

        Returns:
            Ответ от Claude
        """
        # Определяем тип изображения
        if image_url.startswith("data:"):
            # Base64 encoded image
            media_type = image_url.split(";")[0].split(":")[1]
            data = image_url.split(",")[1]
            image_content = {
                "type": "image",
                "source": {"type": "base64", "media_type": media_type, "data": data},
            }
        else:
            # URL
            image_content = {
                "type": "image",
                "source": {"type": "url", "url": image_url},
            }

        messages = [
            {
                "role": "user",
                "content": [
                    image_content,
                    {"type": "text", "text": prompt},
                ],
            }
        ]

        payload = {
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": messages,
        }

        if system:
            payload["system"] = system

        async with httpx.AsyncClient(timeout=120.0) as client:
            response = await client.post(
                f"{self.base_url}/messages",
                headers=self._get_headers(),
                json=payload,
            )
            response.raise_for_status()
            data = response.json()

        return data["content"][0]["text"]


# Доступные модели Claude
CLAUDE_MODELS = {
    "claude-3-opus": "claude-3-opus-20240229",
    "claude-3-sonnet": "claude-3-sonnet-20240229",
    "claude-3-haiku": "claude-3-haiku-20240307",
    "claude-3.5-sonnet": "claude-3-5-sonnet-20241022",
    "claude-4-sonnet": "claude-sonnet-4-5-20250929",
}


async def quick_chat(message: str, model: str = "claude-4-sonnet") -> str:
    """
    Быстрый чат с Claude без создания клиента.

    Args:
        message: Сообщение пользователя
        model: Модель (по умолчанию claude-4-sonnet)

    Returns:
        Ответ от Claude
    """
    client = ClaudeClient(model=CLAUDE_MODELS.get(model, model))
    return await client.chat(message)


if __name__ == "__main__":
    import asyncio

    async def main():
        # Пример использования
        try:
            client = ClaudeClient()
            response = await client.chat(
                "Напиши короткое приветствие на русском языке.",
                system="Ты дружелюбный AI-ассистент.",
            )
            print(f"Claude: {response}")
        except ValueError as e:
            print(f"Ошибка: {e}")
            print("Установите ANTHROPIC_API_KEY для использования Claude API")

    asyncio.run(main())
