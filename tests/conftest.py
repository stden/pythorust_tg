"""Shared test fixtures and configuration."""

import os
import sys
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

# Add project root to path
PROJECT_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(PROJECT_ROOT))


@pytest.fixture(autouse=True)
def clean_module_cache():
    """Clean up module imports between tests to avoid import side effects."""
    # List of modules that may have import side effects
    modules_to_clean = [
        "integrations.prompts",
        "integrations.openai_client",
        "integrations.claude_client",
        "integrations.gemini_client",
        "integrations.ollama_client",
        "integrations.yandex_tts",
        "integrations.kurigram_client",
        "integrations.aws_client",
        "linear_client",
    ]

    # Store modules to clean later
    stored = {}
    for mod in modules_to_clean:
        if mod in sys.modules:
            stored[mod] = sys.modules.pop(mod)

    yield

    # Restore after test
    for mod in modules_to_clean:
        if mod in sys.modules:
            del sys.modules[mod]
    for mod, module in stored.items():
        sys.modules[mod] = module


@pytest.fixture
def mock_env(monkeypatch):
    """Set up common environment variables for testing."""
    env_vars = {
        "TELEGRAM_PHONE": "+1234567890",
        "TELEGRAM_API_ID": "12345",
        "TELEGRAM_API_HASH": "test_hash_abc123",
        "TELEGRAM_SESSION_NAME": "test_session",
        "OPENAI_API_KEY": "test_openai_key",
        "ANTHROPIC_API_KEY": "test_anthropic_key",
        "GOOGLE_API_KEY": "test_google_key",
        "LINEAR_API_KEY": "test_linear_key",
        "AWS_ACCESS_KEY_ID": "test_aws_key",
        "AWS_SECRET_ACCESS_KEY": "test_aws_secret",
        "AWS_DEFAULT_REGION": "us-east-1",
        "YANDEX_API_KEY": "test_yandex_key",
        "YANDEX_IAM_TOKEN": "",
        "YANDEX_FOLDER_ID": "test_folder_id",
        "MY_ID": "123456789",
        "MY_NAME": "Test User",
        "USER_ID": "123456789",
        "USER_NAME": "Test User",
    }
    for key, value in env_vars.items():
        monkeypatch.setenv(key, value)
    return env_vars


@pytest.fixture
def temp_prompts_dir(tmp_path):
    """Create a temporary prompts directory with test files."""
    prompts_dir = tmp_path / "prompts"
    prompts_dir.mkdir()

    # Create test prompt files
    test_prompts = {
        "sales_agent.md": "You are a sales agent. Use SPIN and AIDA techniques.",
        "calculator.md": "You are a calculator assistant.",
        "friendly_ai.md": "You are a friendly AI assistant.",
        "moderator.md": "You are a chat moderator.",
        "digest.md": "You create chat digests.",
        "crm_parser.md": "You parse CRM data.",
    }

    for filename, content in test_prompts.items():
        (prompts_dir / filename).write_text(content, encoding="utf-8")

    return prompts_dir


@pytest.fixture
def temp_config_file(tmp_path):
    """Create a temporary config.yml file."""
    config_content = """
chats:
  test_channel:
    type: channel
    id: 1234567890
    title: Test Channel
  test_group:
    type: group
    id: 9876543210
    title: Test Group
  test_user:
    type: username
    username: testuser
  test_user_id:
    type: user
    id: 111222333

openai:
  model: gpt-4o-mini
"""
    config_file = tmp_path / "config.yml"
    config_file.write_text(config_content, encoding="utf-8")
    return config_file


@pytest.fixture
def mock_telegram_client():
    """Mock Telethon TelegramClient."""
    client = MagicMock()
    client.get_messages = AsyncMock(return_value=[])
    client.get_entity = AsyncMock()
    client.send_message = AsyncMock()
    client.delete_messages = AsyncMock()
    client.is_user_authorized = AsyncMock(return_value=True)
    client.start = AsyncMock()
    client.disconnect = AsyncMock()
    client.__aenter__ = AsyncMock(return_value=client)
    client.__aexit__ = AsyncMock()
    return client


@pytest.fixture
def mock_httpx_client():
    """Mock httpx.AsyncClient for API tests."""
    client = MagicMock()
    client.post = AsyncMock()
    client.get = AsyncMock()
    client.__aenter__ = AsyncMock(return_value=client)
    client.__aexit__ = AsyncMock()
    return client


@pytest.fixture
def sample_telegram_message():
    """Create a sample Telegram message for testing."""
    message = MagicMock()
    message.id = 1
    message.sender_id = 123456789
    message.text = "Test message"
    message.message = "Test message"
    message.raw_text = "Test message"
    message.date = MagicMock()
    message.date.strftime = MagicMock(return_value="01.01.2025 12:00:00")
    message.date.isoformat = MagicMock(return_value="2025-01-01T12:00:00")
    message.reply_to_msg_id = None
    message.media = None
    message.views = 100
    message.forwards = 10
    message.reactions = None
    message.chat_id = 1234567890
    message.get_sender = AsyncMock(return_value=MagicMock(
        first_name="Test",
        last_name="User",
        username="testuser"
    ))
    return message


@pytest.fixture
def sample_telegram_message_with_reactions(sample_telegram_message):
    """Create a sample Telegram message with reactions."""
    from unittest.mock import MagicMock

    reaction = MagicMock()
    reaction.count = 5
    reaction.reaction = MagicMock()
    reaction.reaction.emoticon = "👍"

    reactions_obj = MagicMock()
    reactions_obj.results = [reaction]

    sample_telegram_message.reactions = reactions_obj
    return sample_telegram_message


@pytest.fixture
def mock_openai_client():
    """Mock OpenAI client."""
    with patch("openai.OpenAI") as mock_class:
        client = MagicMock()
        mock_class.return_value = client

        # Mock chat completions
        response = MagicMock()
        response.choices = [MagicMock()]
        response.choices[0].message.content = "Test response"
        client.chat.completions.create = MagicMock(return_value=response)

        # Mock audio
        client.audio.transcriptions.create = MagicMock(
            return_value=MagicMock(text="Transcribed text")
        )
        client.audio.speech.create = MagicMock(
            return_value=MagicMock(stream_to_file=MagicMock())
        )

        yield client


@pytest.fixture
def mock_requests():
    """Mock requests library."""
    with patch("requests.Session") as mock_session_class:
        session = MagicMock()
        mock_session_class.return_value = session

        response = MagicMock()
        response.status_code = 200
        response.json.return_value = {}
        response.text = ""
        response.content = b""
        session.post.return_value = response
        session.get.return_value = response

        yield session


@pytest.fixture
def mock_boto3():
    """Mock boto3 for AWS tests."""
    with patch.dict("sys.modules", {"boto3": MagicMock(), "botocore": MagicMock()}):
        import boto3
        session = MagicMock()
        boto3.Session = MagicMock(return_value=session)
        yield session
