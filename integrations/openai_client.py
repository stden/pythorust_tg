"""
OpenAI API Client for voice AI salesman and other AI tasks.
"""
import os
from openai import OpenAI

# Initialize client - uses OPENAI_API_KEY env var by default
client = OpenAI()


def chat_completion(
    messages: list[dict],
    model: str = "gpt-4o-mini",
    temperature: float = 0.7,
    max_tokens: int = 1000
) -> str:
    """
    Send messages to OpenAI and get a response.

    Args:
        messages: List of message dicts with 'role' and 'content'
        model: Model to use (gpt-4o-mini, gpt-4o, gpt-3.5-turbo)
        temperature: Creativity (0-2)
        max_tokens: Max response length

    Returns:
        Assistant's response text
    """
    response = client.chat.completions.create(
        model=model,
        messages=messages,
        temperature=temperature,
        max_tokens=max_tokens
    )
    return response.choices[0].message.content


def sales_agent_response(user_message: str, context: str = "") -> str:
    """
    Generate a sales agent response for the voice AI salesman.

    Args:
        user_message: Customer's message
        context: Additional context (product info, customer history, etc.)

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

    messages = [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_message}
    ]

    return chat_completion(messages, model="gpt-4o-mini", temperature=0.8)


def transcribe_audio(audio_file_path: str, language: str = "ru") -> str:
    """
    Transcribe audio using OpenAI Whisper API.

    Args:
        audio_file_path: Path to audio file
        language: Language code (ru, en, etc.)

    Returns:
        Transcribed text
    """
    with open(audio_file_path, "rb") as audio_file:
        transcription = client.audio.transcriptions.create(
            model="whisper-1",
            file=audio_file,
            language=language
        )
    return transcription.text


def text_to_speech(text: str, output_path: str, voice: str = "alloy") -> str:
    """
    Convert text to speech using OpenAI TTS.

    Args:
        text: Text to convert
        output_path: Path for output audio file
        voice: Voice to use (alloy, echo, fable, onyx, nova, shimmer)

    Returns:
        Path to generated audio file
    """
    response = client.audio.speech.create(
        model="tts-1",
        voice=voice,
        input=text
    )

    response.stream_to_file(output_path)
    return output_path


# Example usage
if __name__ == "__main__":
    # Test chat completion
    print("Testing OpenAI connection...")

    # Simple test
    response = chat_completion([
        {"role": "user", "content": "Привет! Скажи одним словом, что ты работаешь."}
    ])
    print(f"Response: {response}")

    # Sales agent test
    print("\nTesting sales agent...")
    sales_response = sales_agent_response(
        "Мне это не нужно, слишком дорого",
        context="Продукт: онлайн-курс по программированию за 50,000 руб"
    )
    print(f"Sales response: {sales_response}")
