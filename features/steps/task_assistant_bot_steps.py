# -*- coding: utf-8 -*-
"""Шаги Behave для Task Assistant Bot."""

import asyncio
import os
import sys
import types
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

from behave import given, when, then

# Настраиваем окружение до импорта модуля
os.environ.setdefault("TELEGRAM_API_ID", "1")
os.environ.setdefault("TELEGRAM_API_HASH", "hash")
os.environ.setdefault("TASK_ASSISTANT_BOT_TOKEN", "token")
os.environ.setdefault("ALLOWED_USERS", "")

# Добавляем корень проекта в путь
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

import task_assistant_bot
from task_assistant_bot import TaskAssistantBot


class MockEvent:
    """Упрощённый mock события Telethon."""

    def __init__(self, sender_id: int):
        self.sender_id = sender_id
        self.responses = []

    async def respond(self, text, buttons=None):
        self.responses.append({"text": text, "buttons": buttons})


def _fake_aiohttp_module(status_code: int):
    """Возвращает заглушку aiohttp с нужным статусом ответа."""

    class FakeTimeout:
        def __init__(self, total=None):
            self.total = total

    class FakeResponse:
        def __init__(self, status: int):
            self.status = status

        async def __aenter__(self):
            return self

        async def __aexit__(self, *exc):
            return False

    class FakeSession:
        def __init__(self, status: int):
            self.status = status

        async def __aenter__(self):
            return self

        async def __aexit__(self, *exc):
            return False

        def get(self, *_args, **_kwargs):
            return FakeResponse(self.status)

    return types.SimpleNamespace(
        ClientSession=lambda: FakeSession(status_code), ClientTimeout=lambda total=None: FakeTimeout(total)
    )


def _get_loop():
    """Возвращает event loop, создавая новый при необходимости."""
    try:
        return asyncio.get_event_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        return loop


@given("Task Assistant Bot инициализирован")
def step_init_bot(context):
    """Создаём инстанс бота и фиксируем дефолтный список допущенных."""
    task_assistant_bot.ALLOWED_USERS = []
    with patch("task_assistant_bot.TelegramClient", return_value=MagicMock()):
        context.bot = TaskAssistantBot()


@given('разрешённые пользователи: "{user_ids}"')
def step_allowed_users(context, user_ids):
    """Устанавливаем список разрешённых пользователей."""
    ids = [int(part.strip()) for part in user_ids.split(",") if part.strip()]
    task_assistant_bot.ALLOWED_USERS = ids


@when("пользователь {user_id:d} отправляет /start")
def step_user_sends_start(context, user_id):
    """Отправляем команду /start и сохраняем ответ."""
    event = MockEvent(user_id)
    context.last_event = event

    # Подменяем Button.inline, чтобы проще проверять payload
    original_inline = task_assistant_bot.Button.inline
    task_assistant_bot.Button.inline = lambda text, data: {"text": text, "data": data}

    loop = _get_loop()
    loop.run_until_complete(context.bot.start_handler(event))

    # Возвращаем оригинальную реализацию
    task_assistant_bot.Button.inline = original_inline


@when("бот проверяет здоровье N8N")
def step_check_n8n(context):
    """Выполняем health-check N8N с подменённым aiohttp."""
    fake_aiohttp = _fake_aiohttp_module(status_code=200)
    loop = _get_loop()
    with patch.dict(sys.modules, {"aiohttp": fake_aiohttp}):
        context.health_result = loop.run_until_complete(context.bot.check_n8n_health())


@when("я перезапускаю N8N сервис")
def step_restart_n8n(context):
    """Мокаем перезапуск N8N и последующий health-check."""
    process = MagicMock()
    process.returncode = 0
    process.communicate = AsyncMock(return_value=(b"ok", b""))

    context.bot.check_n8n_health = AsyncMock(return_value={"status": "✅ Работает", "code": 200})

    loop = _get_loop()
    with patch("task_assistant_bot.asyncio.create_subprocess_shell", AsyncMock(return_value=process)):
        context.restart_result = loop.run_until_complete(context.bot.restart_n8n_service())


@when("бот создаёт бэкап N8N")
def step_create_backup(context):
    """Мокаем создание бэкапа через внешнюю команду."""
    process = MagicMock()
    process.returncode = 0
    process.communicate = AsyncMock(return_value=(b"backup complete", b""))

    loop = _get_loop()
    with patch("task_assistant_bot.asyncio.create_subprocess_shell", AsyncMock(return_value=process)):
        context.backup_result = loop.run_until_complete(context.bot.create_n8n_backup())


@when("бот запрашивает статус серверов")
def step_server_status(context):
    """Возвращаем фейковые метрики ресурсов."""

    def make_process(output: bytes):
        proc = MagicMock()
        proc.communicate = AsyncMock(return_value=(output, b""))
        proc.returncode = 0
        return proc

    processes = [
        make_process(b"12.5\n"),  # CPU
        make_process(b"42.0\n"),  # Memory
        make_process(b"55\n"),  # Disk
    ]

    loop = _get_loop()
    with patch("task_assistant_bot.asyncio.create_subprocess_shell", side_effect=processes):
        context.server_status = loop.run_until_complete(context.bot.get_server_status())


@then("бот показывает {count:d} кнопок")
def step_check_buttons_count(context, count):
    """Проверяем количество inline кнопок."""
    response = context.last_event.responses[0]
    buttons = response.get("buttons") or []
    flat = [btn for row in buttons for btn in row]
    assert len(flat) == count, f"Ожидалось {count} кнопок, получено {len(flat)}"


@then('кнопки содержат действие "{payload}"')
def step_buttons_have_payload(context, payload):
    """Проверяем наличие конкретного payload."""
    response = context.last_event.responses[0]
    flat = [btn for row in response.get("buttons") or [] for btn in row]
    found = any(
        (btn.get("data").decode() if isinstance(btn.get("data"), bytes) else btn.get("data")) == payload for btn in flat
    )
    assert found, f"Не найдена кнопка с payload {payload}"


@then('бот отвечает "{text}"')
def step_bot_replies_with_text(context, text):
    """Проверяем текст ответа бота."""
    response = context.last_event.responses[0]
    assert text in response["text"], f"Ожидали '{text}', получили '{response['text']}'"


@then('статус проверки равен "{status}"')
def step_health_status(context, status):
    """Проверяем статус health-check."""
    assert context.health_result["status"] == status


@then("код ответа равен {code:d}")
def step_health_code(context, code):
    """Проверяем HTTP код health-check."""
    assert context.health_result["code"] == code


@then("результат перезапуска успешный")
def step_restart_success(context):
    """Перезапуск завершился успешно."""
    assert context.restart_result["success"] is True


@then("результат содержит здоровье с кодом {code:d}")
def step_restart_health(context, code):
    """В ответе на перезапуск есть результат health-check."""
    health = context.restart_result.get("health", {})
    assert health.get("code") == code


@then('ответ бэкапа содержит "{text}"')
def step_backup_output(context, text):
    """Проверяем вывод команды бэкапа."""
    output = context.backup_result.get("output", "") or context.backup_result.get("error", "")
    assert text in output, f"'{text}' не найдено в выводе: {output}"


@then("возвращаются значения CPU, памяти и диска")
def step_server_status_result(context):
    """Проверяем, что статус серверов содержит все метрики."""
    data = context.server_status
    assert set(data.keys()) == {"cpu", "memory", "disk"}, f"Некорректные ключи статуса: {data.keys()}"
    assert data["cpu"] == 12.5
    assert data["memory"] == 42.0
    assert data["disk"] == 55.0
