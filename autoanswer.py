import asyncio
import os
from pathlib import Path

import openai
import yaml
from telethon import events
from telegram_session import get_client, SessionLock

CONFIG_PATH = Path(__file__).resolve().parent / "config.yml"

# Получаем клиент с SQLite сессией
client = get_client()


def load_openai_config():
    try:
        with CONFIG_PATH.open("r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}
    except FileNotFoundError:
        return {}
    except Exception as exc:
        print(f"Не удалось загрузить config.yml: {exc}")
        return {}

    return config.get("openai") or {}


openai_config = load_openai_config()
api_key = os.getenv("OPENAI_API_KEY")
if not api_key:
    raise RuntimeError("Не задан ключ OpenAI. Укажите переменную окружения OPENAI_API_KEY.")

# Конфигурация OpenAI
openai.api_key = api_key
MODEL = openai_config.get("model", "o1-mini")

# Дополнительные инструкции ассистенту
system_instructions = (
    "Ты - полезный ассистент, который отвечает на вопросы в Telegram-чате. "
    "Старайся давать подробные, ясные и понятные ответы. "
    "Отвечай нейтральным тоном, при необходимости давай примеры кода и избегай ненормативной лексики. "
    "Если пользователь задаёт технический вопрос, постарайся дать максимально понятный и точный ответ. "
    "Если пользователь не указал иное, отвечай на русском языке."
)


@client.on(events.NewMessage)
async def handler(event):
    if event.out:
        return

    user_message = event.message.message.strip()
    if not user_message:
        return

    try:
        completion = openai.ChatCompletion.create(
            model=MODEL,
            messages=[
                {"role": "system", "content": system_instructions},
                {"role": "user", "content": user_message}
            ],
            temperature=0.7
        )

        bot_reply = completion.choices[0].message.content.strip()
        await event.respond(bot_reply)

    except Exception as e:
        print(f"Ошибка при генерации ответа: {e}")


async def main():
    print("Бот запущен. Ожидаю сообщения...")
    await client.run_until_disconnected()


if __name__ == '__main__':
    with SessionLock():  # Защита от параллельного запуска
        with client:
            client.loop.run_until_complete(main())
