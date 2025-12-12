"""
Ollama Client for local LLM inference.
Позволяет использовать локальные модели вместо OpenAI API.
"""

import requests
import json
from typing import Generator, Optional

OLLAMA_URL = "http://localhost:11434"


def is_ollama_running() -> bool:
    """Check if Ollama server is running."""
    try:
        response = requests.get(f"{OLLAMA_URL}/api/tags", timeout=2)
        return response.status_code == 200
    except requests.RequestException:
        return False


def list_models() -> list[str]:
    """List available models."""
    response = requests.get(f"{OLLAMA_URL}/api/tags")
    response.raise_for_status()
    models = response.json().get("models", [])
    return [m["name"] for m in models]


def generate(
    prompt: str,
    model: str = "qwen2.5:3b",
    system: Optional[str] = None,
    temperature: float = 0.7,
    max_tokens: int = 500,
    stream: bool = False,
) -> str | Generator[str, None, None]:
    """
    Generate text using Ollama.

    Args:
        prompt: User prompt
        model: Model name (qwen2.5:3b, llama3.1:8b, etc.)
        system: System prompt
        temperature: Creativity (0-2)
        max_tokens: Max response tokens
        stream: If True, yields chunks

    Returns:
        Generated text or generator of chunks
    """
    data = {
        "model": model,
        "prompt": prompt,
        "stream": stream,
        "options": {
            "temperature": temperature,
            "num_predict": max_tokens,
        },
    }

    if system:
        data["system"] = system

    if stream:
        return _generate_stream(data)
    else:
        response = requests.post(f"{OLLAMA_URL}/api/generate", json=data, timeout=120)
        response.raise_for_status()
        return response.json()["response"]


def _generate_stream(data: dict) -> Generator[str, None, None]:
    """Stream generation response."""
    response = requests.post(f"{OLLAMA_URL}/api/generate", json=data, stream=True, timeout=120)
    response.raise_for_status()

    for line in response.iter_lines():
        if line:
            chunk = json.loads(line)
            if "response" in chunk:
                yield chunk["response"]


def chat(
    messages: list[dict],
    model: str = "qwen2.5:3b",
    temperature: float = 0.7,
) -> str:
    """
    Chat with model using message history.

    Args:
        messages: List of {"role": "user/assistant/system", "content": "..."}
        model: Model name
        temperature: Creativity

    Returns:
        Assistant response
    """
    response = requests.post(
        f"{OLLAMA_URL}/api/chat",
        json={"model": model, "messages": messages, "stream": False, "options": {"temperature": temperature}},
        timeout=120,
    )
    response.raise_for_status()
    return response.json()["message"]["content"]


def sales_agent_response(user_message: str, context: str = "", model: str = "qwen2.5:3b") -> str:
    """
    Generate sales agent response (local alternative to OpenAI).

    Args:
        user_message: Customer's message
        context: Additional context
        model: Model to use

    Returns:
        Sales agent response
    """
    system_prompt = """Ты — профессиональный продавец-консультант.
Твоя задача — помочь клиенту и убедить его в ценности продукта.
Будь вежливым, но настойчивым. Отвечай кратко (1-3 предложения).
Используй техники продаж: SPIN, AIDA.
Если клиент возражает — обрабатывай возражения.
Если клиент согласен — закрывай сделку."""

    if context:
        system_prompt += f"\n\nКонтекст: {context}"

    return generate(prompt=user_message, model=model, system=system_prompt, temperature=0.8)


def pull_model(model: str) -> bool:
    """
    Download a model.

    Args:
        model: Model name (e.g., "qwen2.5:3b")

    Returns:
        True if successful
    """
    print(f"Downloading {model}...")
    response = requests.post(
        f"{OLLAMA_URL}/api/pull",
        json={"name": model},
        stream=True,
        timeout=3600,  # 1 hour for large models
    )

    for line in response.iter_lines():
        if line:
            status = json.loads(line)
            if "status" in status:
                print(f"  {status['status']}")

    return response.status_code == 200


# Example usage
if __name__ == "__main__":
    print("Ollama Client for Local LLM")
    print("=" * 40)

    if not is_ollama_running():
        print("✗ Ollama не запущена!")
        print("  Запустите: ollama serve")
        print("  Или установите: bash integrations/ollama_setup.sh")
        exit(1)

    print("✓ Ollama работает")

    # List models
    models = list_models()
    print(f"\nУстановленные модели: {models}")

    if not models:
        print("\n⚠ Нет установленных моделей")
        print("  Загружаем qwen2.5:3b...")
        pull_model("qwen2.5:3b")
        models = list_models()

    # Test generation
    if models:
        model = models[0]
        print(f"\nТест модели {model}...")

        response = generate("Привет! Скажи одним словом что работаешь.", model=model)
        print(f"Ответ: {response}")

        # Test sales agent
        print("\nТест продавца...")
        sales_response = sales_agent_response(
            "Мне это дорого", context="Курс программирования за 50,000 руб", model=model
        )
        print(f"Продавец: {sales_response}")
