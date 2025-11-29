"""
Yandex SpeechKit TTS (Text-to-Speech) Client.
Для использования нужен IAM-токен или API-ключ от Yandex Cloud.

Документация: https://cloud.yandex.ru/docs/speechkit/tts/
"""
import os
import requests
from typing import Literal

# Yandex Cloud credentials
# Можно использовать либо API_KEY, либо IAM_TOKEN
YANDEX_API_KEY = os.getenv("YANDEX_API_KEY", "")
YANDEX_IAM_TOKEN = os.getenv("YANDEX_IAM_TOKEN", "")
YANDEX_FOLDER_ID = os.getenv("YANDEX_FOLDER_ID", "")

TTS_URL = "https://tts.api.cloud.yandex.net/speech/v1/tts:synthesize"
STT_URL = "https://stt.api.cloud.yandex.net/speech/v1/stt:recognize"

# Доступные голоса для русского языка
VOICES_RU = {
    "alena": "Алёна (нейтральный женский)",
    "filipp": "Филипп (нейтральный мужской)",
    "ermil": "Ермиль (нейтральный мужской)",
    "jane": "Джейн (нейтральный женский)",
    "madirus": "Мадирус (нейтральный мужской)",
    "omazh": "Омаж (нейтральный женский)",
    "zahar": "Захар (нейтральный мужской)",
    "dasha": "Даша (нейтральный женский)",
    "julia": "Юлия (строгий женский)",
    "lera": "Лера (дружелюбный женский)",
    "marina": "Марина (мягкий женский)",
    "alexander": "Александр (хороший мужской)",
    "kirill": "Кирилл (строгий мужской)",
    "anton": "Антон (добрый мужской)",
}


def _get_headers() -> dict:
    """Get authorization headers for Yandex API."""
    if YANDEX_IAM_TOKEN:
        return {"Authorization": f"Bearer {YANDEX_IAM_TOKEN}"}
    elif YANDEX_API_KEY:
        return {"Authorization": f"Api-Key {YANDEX_API_KEY}"}
    else:
        raise ValueError(
            "Установите YANDEX_API_KEY или YANDEX_IAM_TOKEN в переменных окружения"
        )


def text_to_speech(
    text: str,
    output_path: str,
    voice: str = "alena",
    emotion: Literal["neutral", "good", "evil"] = "neutral",
    speed: float = 1.0,
    format: Literal["lpcm", "oggopus", "mp3"] = "mp3",
) -> str:
    """
    Синтез речи через Yandex SpeechKit.

    Args:
        text: Текст для синтеза (до 5000 символов)
        output_path: Путь для сохранения аудио
        voice: Голос (см. VOICES_RU)
        emotion: Эмоция (neutral, good, evil)
        speed: Скорость речи (0.1 - 3.0)
        format: Формат аудио (lpcm, oggopus, mp3)

    Returns:
        Путь к сохранённому файлу
    """
    headers = _get_headers()

    data = {
        "text": text,
        "lang": "ru-RU",
        "voice": voice,
        "emotion": emotion,
        "speed": str(speed),
        "format": format,
        "folderId": YANDEX_FOLDER_ID,
    }

    response = requests.post(TTS_URL, headers=headers, data=data)

    if response.status_code != 200:
        raise Exception(f"Yandex TTS error: {response.status_code} - {response.text}")

    with open(output_path, "wb") as f:
        f.write(response.content)

    return output_path


def speech_to_text(
    audio_path: str,
    language: str = "ru-RU",
    topic: Literal["general", "maps", "dates", "names", "numbers"] = "general",
) -> str:
    """
    Распознавание речи через Yandex SpeechKit.

    Args:
        audio_path: Путь к аудиофайлу (OGG/Opus)
        language: Язык распознавания
        topic: Тема для улучшения распознавания

    Returns:
        Распознанный текст
    """
    headers = _get_headers()

    params = {
        "lang": language,
        "topic": topic,
        "folderId": YANDEX_FOLDER_ID,
    }

    with open(audio_path, "rb") as f:
        audio_data = f.read()

    response = requests.post(
        STT_URL,
        headers=headers,
        params=params,
        data=audio_data,
    )

    if response.status_code != 200:
        raise Exception(f"Yandex STT error: {response.status_code} - {response.text}")

    result = response.json()
    return result.get("result", "")


def text_to_speech_ssml(
    ssml: str,
    output_path: str,
    voice: str = "alena",
) -> str:
    """
    Синтез речи с SSML разметкой для контроля интонации.

    Args:
        ssml: Текст в формате SSML
        output_path: Путь для сохранения

    Returns:
        Путь к файлу

    Example SSML:
        <speak>
            Привет! <break time="500ms"/>
            Меня зовут <emphasis>Алёна</emphasis>.
            <prosody rate="slow">Говорю медленно.</prosody>
        </speak>
    """
    headers = _get_headers()

    data = {
        "ssml": ssml,
        "lang": "ru-RU",
        "voice": voice,
        "format": "mp3",
        "folderId": YANDEX_FOLDER_ID,
    }

    response = requests.post(TTS_URL, headers=headers, data=data)

    if response.status_code != 200:
        raise Exception(f"Yandex TTS error: {response.status_code} - {response.text}")

    with open(output_path, "wb") as f:
        f.write(response.content)

    return output_path


def list_voices():
    """Показать доступные голоса."""
    print("Доступные голоса для русского языка:")
    print("-" * 40)
    for voice_id, description in VOICES_RU.items():
        print(f"  {voice_id}: {description}")


# Example usage
if __name__ == "__main__":
    print("Yandex SpeechKit TTS Client")
    print("=" * 40)

    list_voices()

    print("\n" + "=" * 40)
    print("Для использования установите переменные окружения:")
    print("  export YANDEX_API_KEY='ваш_api_ключ'")
    print("  export YANDEX_FOLDER_ID='ваш_folder_id'")
    print("\nПолучить ключи: https://console.cloud.yandex.ru/")

    # Test if credentials are set
    if YANDEX_API_KEY or YANDEX_IAM_TOKEN:
        print("\n✓ Credentials найдены, тестируем...")
        try:
            output = text_to_speech(
                "Привет! Это тест синтеза речи от Яндекса.",
                "/tmp/yandex_tts_test.mp3",
                voice="alena"
            )
            print(f"✓ Аудио сохранено: {output}")
        except Exception as e:
            print(f"✗ Ошибка: {e}")
    else:
        print("\n✗ Credentials не установлены")
