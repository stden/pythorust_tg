import pytest
from datetime import datetime, timedelta
from unittest.mock import MagicMock, AsyncMock, patch
from telethon.tl.types import User, Channel, Message, MessageReactions, ReactionCount
from chat_analysis.fetcher import MessageFetcher, FormattedMessage, AnalyzerConfig

@pytest.fixture
def config():
    return AnalyzerConfig(
        message_limit=10,
        days_back=7,
        min_message_length=5,
        include_media=False,
        exclude_bots=True,
        verbose=False
    )

@pytest.fixture
def fetcher(config):
    with patch.dict("os.environ", {
        "TELEGRAM_API_ID": "12345",
        "TELEGRAM_API_HASH": "hash",
        "TELEGRAM_SESSION_FILE": "test_session"
    }):
        return MessageFetcher(config)

def test_fetcher_init(fetcher):
    assert fetcher.api_id == 12345
    assert fetcher.api_hash == "hash"
    assert fetcher.session_file == "test_session"

@pytest.mark.asyncio
async def test_fetcher_context_manager(fetcher):
    mock_client = MagicMock()
    mock_client.connect = AsyncMock()
    mock_client.disconnect = AsyncMock()
    mock_client.is_user_authorized = AsyncMock(return_value=True)
    
    with patch("chat_analysis.fetcher.TelegramClient", return_value=mock_client):
        async with fetcher as f:
            assert f.client == mock_client
            mock_client.connect.assert_called_once()
            mock_client.is_user_authorized.assert_called_once()
        
        mock_client.disconnect.assert_called_once()

@pytest.mark.asyncio
async def test_fetcher_context_manager_unauthorized(fetcher):
    mock_client = MagicMock()
    mock_client.connect = AsyncMock()
    mock_client.is_user_authorized = AsyncMock(return_value=False)
    
    with patch("chat_analysis.fetcher.TelegramClient", return_value=mock_client):
        with pytest.raises(RuntimeError, match="not authorized"):
            async with fetcher:
                pass

def test_get_offset_date(fetcher):
    fetcher.config.days_back = 7
    offset = fetcher._get_offset_date()
    assert isinstance(offset, datetime)
    # Check if it's roughly 7 days ago
    expected = datetime.now() - timedelta(days=7)
    assert abs((offset - expected).total_seconds()) < 10

    fetcher.config.days_back = 0
    assert fetcher._get_offset_date() is None

@pytest.mark.asyncio
async def test_should_include(fetcher):
    # Valid message
    msg = MagicMock(spec=Message)
    msg.message = "Hello world"
    msg.media = None
    msg.sender = MagicMock(spec=User)
    msg.sender.bot = False
    assert await fetcher._should_include(msg) is True

    # Empty message
    msg.message = ""
    assert await fetcher._should_include(msg) is False

    # Short message
    msg.message = "Hi"
    assert await fetcher._should_include(msg) is False

    # Bot message
    msg.message = "Hello world"
    msg.sender.bot = True
    assert await fetcher._should_include(msg) is False
    
    fetcher.config.exclude_bots = False
    assert await fetcher._should_include(msg) is True

    # Media message
    msg.sender.bot = False
    msg.media = MagicMock()
    fetcher.config.include_media = False
    assert await fetcher._should_include(msg) is True # Still has text "Hello world"
    
    msg.message = "Med" # Short text + media
    assert await fetcher._should_include(msg) is False

@pytest.mark.asyncio
async def test_get_sender_name(fetcher):
    # User with name
    user = MagicMock(spec=User)
    user.first_name = "John"
    user.last_name = "Doe"
    msg = MagicMock(spec=Message)
    msg.sender = user
    assert await fetcher._get_sender_name(msg) == "John Doe"

    # User with username
    user.first_name = None
    user.last_name = None
    user.username = "johndoe"
    assert await fetcher._get_sender_name(msg) == "@johndoe"

    # User with only ID
    user.username = None
    user.id = 12345
    assert await fetcher._get_sender_name(msg) == "User12345"

    # Channel
    channel = MagicMock(spec=Channel)
    channel.title = "My Channel"
    msg.sender = channel
    assert await fetcher._get_sender_name(msg) == "My Channel"

    # Unknown
    msg.sender = None
    assert await fetcher._get_sender_name(msg) == "Unknown"

@pytest.mark.asyncio
async def test_format_message(fetcher):
    msg = MagicMock(spec=Message)
    msg.date = datetime(2023, 1, 1, 12, 0)
    msg.message = "Test message"
    msg.id = 1
    msg.media = None
    msg.reactions = MagicMock(spec=MessageReactions)
    msg.reactions.results = [
        MagicMock(spec=ReactionCount, count=5),
        MagicMock(spec=ReactionCount, count=3)
    ]
    
    with patch.object(fetcher, "_get_sender_name", return_value="Sender"):
        formatted = await fetcher._format_message(msg)
        assert formatted.date == msg.date
        assert formatted.sender_name == "Sender"
        assert formatted.text == "Test message"
        assert formatted.message_id == 1
        assert formatted.reactions_count == 8
        assert formatted.has_media is False

def test_format_messages_for_llm(fetcher):
    msgs = [
        FormattedMessage(
            date=datetime(2023, 1, 1, 12, 0),
            sender_name="Alice",
            text="Hello",
            message_id=1,
            reactions_count=2,
            has_media=False
        ),
        FormattedMessage(
            date=datetime(2023, 1, 1, 12, 5),
            sender_name="Bob",
            text="World",
            message_id=2,
            reactions_count=0,
            has_media=True
        )
    ]
    
    formatted = fetcher.format_messages_for_llm(msgs)
    lines = formatted.split("\n")
    assert "[01.01.2023 12:00] Alice: Hello [2 reactions]" in lines[0]
    assert "[01.01.2023 12:05] Bob: World [media]" in lines[1]

def test_get_metadata(fetcher):
    msgs = [
        FormattedMessage(
            date=datetime(2023, 1, 1, 12, 0),
            sender_name="Alice",
            text="Hello",
            message_id=1,
            reactions_count=2,
            has_media=False
        ),
        FormattedMessage(
            date=datetime(2023, 1, 1, 12, 5),
            sender_name="Bob",
            text="World",
            message_id=2,
            reactions_count=3,
            has_media=True
        )
    ]
    
    metadata = fetcher.get_metadata(msgs)
    assert metadata["total_messages"] == 2
    assert metadata["unique_senders"] == 2
    assert metadata["total_reactions"] == 5
    assert metadata["has_media"] is True
    assert metadata["date_range"]["start"] == msgs[0].date.isoformat()
    assert metadata["date_range"]["end"] == msgs[1].date.isoformat()

    # Empty case
    assert fetcher.get_metadata([])["total_messages"] == 0

@pytest.mark.asyncio
async def test_get_messages(fetcher):
    mock_client = MagicMock()
    mock_client.get_entity = AsyncMock(return_value="entity")
    
    # Mock iter_messages
    async def mock_iter(*args, **kwargs):
        yield MagicMock(spec=Message, message="msg1")
        yield MagicMock(spec=Message, message="msg2")

    mock_client.iter_messages = mock_iter
    fetcher.client = mock_client
    
    with patch.object(fetcher, "_should_include", side_effect=[True, False]), \
         patch.object(fetcher, "_format_message", return_value=FormattedMessage(
             date=datetime.now(), sender_name="S", text="T", message_id=1
         )):
        messages = await fetcher.get_messages("test_chat")
        assert len(messages) == 1
        assert messages[0].text == "T"
        mock_client.get_entity.assert_called_with("test_chat")
