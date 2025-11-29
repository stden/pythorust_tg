"""Unit tests for BFLSalesBot logic."""

from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from bfl_sales_bot import BFLSalesBot, MySQLLogger, PromptVariant


@pytest.fixture
def bot_with_mocks():
    """Create BFLSalesBot with mocked dependencies."""
    with patch("bfl_sales_bot.OpenAIClient") as mock_ai_class:
        ai_instance = MagicMock()
        ai_instance.chat_completion = AsyncMock()
        mock_ai_class.return_value = ai_instance

        bot = BFLSalesBot()

    bot.ai = ai_instance
    bot.db = MagicMock(spec=MySQLLogger)
    bot.db.create_session.return_value = 999
    bot.db.get_session.return_value = None
    bot.db.get_conversation_history.return_value = []

    bot.experiments = MagicMock()
    variant = PromptVariant(
        name="fast_close",
        prompt="system test prompt",
        temperature=0.4,
        model="gpt-fast",
    )
    bot.experiments.get_or_assign_variant.return_value = variant
    bot.experiments.detect_and_mark_conversion.return_value = None

    return bot, ai_instance, variant


@pytest.mark.asyncio
async def test_handle_start_saves_user_and_sends_onboarding(bot_with_mocks):
    """handle_start should persist user/session and send three onboarding messages."""
    bot, _, _ = bot_with_mocks

    user = SimpleNamespace(
        id=42,
        username="alex",
        first_name="Alex",
        last_name="Smith",
        premium=True,
        bot=False,
    )
    event = MagicMock()
    event.message = SimpleNamespace(id=321)
    event.get_sender = AsyncMock(return_value=user)
    event.respond = AsyncMock(
        side_effect=[
            SimpleNamespace(id=101),
            SimpleNamespace(id=102),
            SimpleNamespace(id=103),
        ]
    )

    await bot.handle_start(event)

    bot.db.save_user.assert_called_once_with(user)
    bot.db.create_session.assert_called_once_with(user.id)
    bot.experiments.get_or_assign_variant.assert_called_once_with(
        user.id, bot.db.create_session.return_value
    )

    # First saved message is /start, remaining three are bot responses
    directions = [call.kwargs["direction"] for call in bot.db.save_message.call_args_list]
    assert directions.count("incoming") == 1
    assert directions.count("outgoing") == 3
    bot.db.save_message.assert_any_call(
        user_id=user.id,
        message_id=event.message.id,
        text="/start",
        direction="incoming",
    )

    assert event.respond.await_count == 3
    greeting_text = event.respond.await_args_list[0].args[0]
    assert user.first_name in greeting_text
    assert greeting_text.startswith("Здравствуйте")


@pytest.mark.asyncio
async def test_handle_message_builds_ai_payload_and_logs(bot_with_mocks):
    """handle_message should use history, call AI, and log incoming/outgoing messages."""
    bot, ai, variant = bot_with_mocks

    bot.db.get_session.return_value = {"id": 555}
    bot.db.get_conversation_history.return_value = [
        {"direction": "incoming", "message_text": "Нужна консультация"},
        {"direction": "outgoing", "message_text": "Расскажите про бюджет"},
    ]

    ai_response = SimpleNamespace(
        choices=[SimpleNamespace(message=SimpleNamespace(content="AI reply text"))]
    )
    ai.chat_completion.return_value = ai_response

    user = SimpleNamespace(
        id=77,
        username="irina",
        first_name="Ирина",
        last_name="Петрова",
        premium=False,
        bot=False,
    )
    event = MagicMock()
    event.message = SimpleNamespace(id=700, text="Хочу купить R9 завтра")
    event.get_sender = AsyncMock(return_value=user)
    event.respond = AsyncMock(return_value=SimpleNamespace(id=888))

    await bot.handle_message(event)

    bot.db.save_user.assert_called_once_with(user)
    bot.db.get_conversation_history.assert_called_once_with(user.id)
    bot.experiments.get_or_assign_variant.assert_called_once_with(user.id, 555)
    bot.experiments.detect_and_mark_conversion.assert_called_once_with(
        555, event.message.text
    )

    ai_call = ai.chat_completion.await_args
    messages = ai_call.args[0]
    assert ai_call.kwargs["model"] == variant.model
    assert ai_call.kwargs["temperature"] == variant.temperature
    assert messages[0] == {"role": "system", "content": variant.prompt}
    assert messages[1] == {"role": "user", "content": "Нужна консультация"}
    assert messages[2] == {"role": "assistant", "content": "Расскажите про бюджет"}
    assert messages[-1]["content"] == event.message.text

    assert bot.db.save_message.call_args_list[0].kwargs["direction"] == "incoming"
    assert bot.db.save_message.call_args_list[1].kwargs["direction"] == "outgoing"
    assert bot.db.save_message.call_args_list[1].kwargs["text"] == "AI reply text"
    event.respond.assert_awaited_once_with("AI reply text")


@pytest.mark.asyncio
async def test_handle_message_handles_ai_error(bot_with_mocks):
    """If AI fails, bot should send an apology message."""
    bot, ai, _ = bot_with_mocks

    # Simulate AI exception
    ai.chat_completion.side_effect = Exception("OpenAI Down")

    user = SimpleNamespace(id=123, first_name="User")
    event = MagicMock()
    event.message = SimpleNamespace(id=111, text="Hello")
    event.get_sender = AsyncMock(return_value=user)
    event.respond = AsyncMock(return_value=SimpleNamespace(id=222))

    await bot.handle_message(event)

    # Verify fallback message
    event.respond.assert_awaited_once_with("Извините, произошла ошибка. Попробуйте ещё раз.")
    
    # Verify it was logged to DB
    bot.db.save_message.assert_called_with(
        user_id=user.id,
        message_id=222,
        text="Извините, произошла ошибка. Попробуйте ещё раз.",
        direction="outgoing"
    )


@pytest.mark.asyncio
async def test_handle_message_uses_default_prompt_if_no_experiment(bot_with_mocks):
    """If experiments manager is missing or returns None, use default prompt."""
    bot, ai, _ = bot_with_mocks
    
    # Disable experiments
    bot.experiments = None
    
    # Mock default prompt import
    with patch("bfl_sales_bot.SALES_SYSTEM_PROMPT", "DEFAULT_PROMPT"):
        ai.chat_completion.return_value = SimpleNamespace(
            choices=[SimpleNamespace(message=SimpleNamespace(content="Default Reply"))]
        )

        user = SimpleNamespace(id=999, first_name="Test")
        event = MagicMock()
        event.message = SimpleNamespace(id=1, text="Hi")
        event.get_sender = AsyncMock(return_value=user)
        event.respond = AsyncMock(return_value=SimpleNamespace(id=2))

        await bot.handle_message(event)

        # Check that default prompt was used
        ai_call = ai.chat_completion.await_args
        messages = ai_call.args[0]
        assert messages[0] == {"role": "system", "content": "DEFAULT_PROMPT"}
