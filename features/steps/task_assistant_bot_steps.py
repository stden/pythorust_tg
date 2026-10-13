# -*- coding: utf-8 -*-
"""Behave steps for Task Assistant Bot."""

import asyncio
import os
import sys
import types
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

from behave import given, when, then

# Configure environment before importing the module
os.environ.setdefault("TELEGRAM_API_ID", "1")
os.environ.setdefault("TELEGRAM_API_HASH", "hash")
os.environ.setdefault("TASK_ASSISTANT_BOT_TOKEN", "token")
os.environ.setdefault("ALLOWED_USERS", "")

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

import task_assistant_bot
from task_assistant_bot import TaskAssistantBot


class MockEvent:
    """Simplified mock of a Telethon event."""

    def __init__(self, sender_id: int):
        self.sender_id = sender_id
        self.responses = []

    async def respond(self, text, buttons=None):
        self.responses.append({"text": text, "buttons": buttons})


def _fake_aiohttp_module(status_code: int):
    """Return an aiohttp stub with the specified response status."""

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
    """Return an event loop, creating a new one if needed."""
    try:
        return asyncio.get_event_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        return loop


@given("Task Assistant Bot is initialized")
def step_init_bot(context):
    """Create a bot instance and set the default allowlist."""
    task_assistant_bot.ALLOWED_USERS = []
    with patch("task_assistant_bot.TelegramClient", return_value=MagicMock()):
        context.bot = TaskAssistantBot()


@given('allowed users are "{user_ids}"')
def step_allowed_users(context, user_ids):
    """Set the allowlist."""
    ids = [int(part.strip()) for part in user_ids.split(",") if part.strip()]
    task_assistant_bot.ALLOWED_USERS = ids


@when("user {user_id:d} sends /start")
def step_user_sends_start(context, user_id):
    """Send /start and capture the response."""
    event = MockEvent(user_id)
    context.last_event = event

    # Replace Button.inline to simplify payload assertions
    original_inline = task_assistant_bot.Button.inline
    task_assistant_bot.Button.inline = lambda text, data: {"text": text, "data": data}

    loop = _get_loop()
    loop.run_until_complete(context.bot.start_handler(event))

    # Restore original implementation
    task_assistant_bot.Button.inline = original_inline


@when("the bot checks N8N health")
def step_check_n8n(context):
    """Run N8N health-check with a stubbed aiohttp."""
    fake_aiohttp = _fake_aiohttp_module(status_code=200)
    loop = _get_loop()
    with patch.dict(sys.modules, {"aiohttp": fake_aiohttp}):
        context.health_result = loop.run_until_complete(context.bot.check_n8n_health())


@when("I restart the N8N service")
def step_restart_n8n(context):
    """Mock N8N restart and subsequent health-check."""
    process = MagicMock()
    process.returncode = 0
    process.communicate = AsyncMock(return_value=(b"ok", b""))

    context.bot.check_n8n_health = AsyncMock(return_value={"status": "✅ Работает", "code": 200})

    loop = _get_loop()
    with patch("task_assistant_bot.asyncio.create_subprocess_shell", AsyncMock(return_value=process)):
        context.restart_result = loop.run_until_complete(context.bot.restart_n8n_service())


@when("the bot creates an N8N backup")
def step_create_backup(context):
    """Mock backup creation via an external command."""
    process = MagicMock()
    process.returncode = 0
    process.communicate = AsyncMock(return_value=(b"backup complete", b""))

    loop = _get_loop()
    with patch("task_assistant_bot.asyncio.create_subprocess_shell", AsyncMock(return_value=process)):
        context.backup_result = loop.run_until_complete(context.bot.create_n8n_backup())


@when("the bot requests server status")
def step_server_status(context):
    """Return fake resource metrics."""

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


@then("the bot shows {count:d} buttons")
def step_check_buttons_count(context, count):
    """Assert the number of inline buttons."""
    response = context.last_event.responses[0]
    buttons = response.get("buttons") or []
    flat = [btn for row in buttons for btn in row]
    assert len(flat) == count, f"Expected {count} buttons, got {len(flat)}"


@then('the buttons include action "{payload}"')
def step_buttons_have_payload(context, payload):
    """Assert a specific payload exists."""
    response = context.last_event.responses[0]
    flat = [btn for row in response.get("buttons") or [] for btn in row]
    found = any(
        (btn.get("data").decode() if isinstance(btn.get("data"), bytes) else btn.get("data")) == payload for btn in flat
    )
    assert found, f"Button with payload {payload} not found"


@then('the bot replies "{text}"')
def step_bot_replies_with_text(context, text):
    """Assert bot response text."""
    response = context.last_event.responses[0]
    assert text in response["text"], f"Expected '{text}', got '{response['text']}'"


@then('the health status is "{status}"')
def step_health_status(context, status):
    """Assert health-check status."""
    assert context.health_result["status"] == status


@then("the HTTP status code is {code:d}")
def step_health_code(context, code):
    """Assert health-check HTTP code."""
    assert context.health_result["code"] == code


@then("the restart result is successful")
def step_restart_success(context):
    """Assert restart was successful."""
    assert context.restart_result["success"] is True


@then("the result includes health with status code {code:d}")
def step_restart_health(context, code):
    """Assert restart response includes health-check data."""
    health = context.restart_result.get("health", {})
    assert health.get("code") == code


@then('the backup output contains "{text}"')
def step_backup_output(context, text):
    """Assert backup command output."""
    output = context.backup_result.get("output", "") or context.backup_result.get("error", "")
    assert text in output, f"'{text}' not found in output: {output}"


@then("CPU, memory, and disk values are returned")
def step_server_status_result(context):
    """Assert server status contains all metrics."""
    data = context.server_status
    assert set(data.keys()) == {"cpu", "memory", "disk"}, f"Unexpected status keys: {data.keys()}"
    assert data["cpu"] == 12.5
    assert data["memory"] == 42.0
    assert data["disk"] == 55.0
