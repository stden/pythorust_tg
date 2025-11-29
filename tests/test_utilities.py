"""Tests for various utility modules."""

import pytest
import os
import json
from unittest.mock import MagicMock, patch, mock_open
from datetime import datetime, timedelta
from pathlib import Path

# Test chat_export_utils.py
from chat_export_utils import (
    format_message,
    save_to_markdown,
    extract_reactions,
    filter_messages,
    export_chat,
    MessageFormatter,
    ChatExporter,
)


class TestChatExportUtils:
    """Test chat export utilities."""

    def test_format_message_basic(self):
        """Test basic message formatting."""
        message = MagicMock()
        message.date = datetime(2025, 1, 1, 12, 0, 0)
        message.sender = MagicMock(first_name="John", last_name="Doe")
        message.text = "Hello world"
        message.reactions = None
        
        result = format_message(message)
        
        assert "01.01.2025 12:00:00" in result
        assert "John Doe" in result
        assert "Hello world" in result

    def test_format_message_with_reactions(self):
        """Test formatting message with reactions."""
        message = MagicMock()
        message.date = datetime(2025, 1, 1, 12, 0, 0)
        message.sender = MagicMock(first_name="User")
        message.text = "Test"
        
        # Mock reactions
        reaction = MagicMock()
        reaction.count = 5
        reaction.reaction.emoticon = "👍"
        message.reactions = MagicMock(results=[reaction])
        
        result = format_message(message)
        
        assert "👍" in result
        assert "5" in result

    def test_extract_reactions(self):
        """Test extracting reactions from message."""
        reaction1 = MagicMock()
        reaction1.count = 10
        reaction1.reaction.emoticon = "❤️"
        
        reaction2 = MagicMock()
        reaction2.count = 5
        reaction2.reaction.emoticon = "👍"
        
        message = MagicMock()
        message.reactions = MagicMock(results=[reaction1, reaction2])
        
        reactions = extract_reactions(message)
        
        assert reactions == {"❤️": 10, "👍": 5}

    def test_extract_reactions_none(self):
        """Test extracting reactions when none exist."""
        message = MagicMock()
        message.reactions = None
        
        reactions = extract_reactions(message)
        
        assert reactions == {}

    def test_filter_messages_by_reactions(self):
        """Test filtering messages by reaction count."""
        messages = []
        for i in range(5):
            msg = MagicMock()
            msg.id = i
            if i < 2:
                msg.reactions = None
            else:
                reaction = MagicMock()
                reaction.count = i * 10
                msg.reactions = MagicMock(results=[reaction])
            messages.append(msg)
        
        filtered = filter_messages(messages, min_reactions=15)
        
        assert len(filtered) == 2  # Messages with 30 and 40 reactions

    def test_save_to_markdown(self, tmp_path):
        """Test saving messages to markdown file."""
        messages = [
            "01.01.2025 12:00:00 User1: Hello",
            "01.01.2025 12:01:00 User2: Hi there"
        ]
        
        output_file = tmp_path / "chat.md"
        save_to_markdown(messages, str(output_file), title="Test Chat")
        
        content = output_file.read_text()
        assert "# Test Chat" in content
        assert "User1: Hello" in content
        assert "User2: Hi there" in content


# Test voice_utils.py
from voice_utils import (
    convert_audio_format,
    transcribe_audio,
    synthesize_speech,
    VoiceProcessor,
    AudioFormat,
)


class TestVoiceUtils:
    """Test voice processing utilities."""

    @patch("voice_utils.ffmpeg")
    def test_convert_audio_format(self, mock_ffmpeg, tmp_path):
        """Test audio format conversion."""
        input_file = tmp_path / "input.ogg"
        output_file = tmp_path / "output.mp3"
        input_file.write_bytes(b"fake_audio")
        
        # Mock ffmpeg
        mock_stream = MagicMock()
        mock_ffmpeg.input.return_value = mock_stream
        mock_stream.output.return_value = mock_stream
        mock_stream.run.return_value = None
        
        result = convert_audio_format(
            str(input_file),
            str(output_file),
            AudioFormat.MP3
        )
        
        assert result == str(output_file)
        mock_ffmpeg.input.assert_called_once()

    @patch("voice_utils.openai.OpenAI")
    def test_transcribe_audio(self, mock_openai_class, tmp_path):
        """Test audio transcription."""
        # Mock OpenAI client
        mock_client = MagicMock()
        mock_openai_class.return_value = mock_client
        mock_client.audio.transcriptions.create.return_value = MagicMock(
            text="Transcribed text"
        )
        
        audio_file = tmp_path / "audio.mp3"
        audio_file.write_bytes(b"fake_audio")
        
        result = transcribe_audio(str(audio_file), api_key="test_key")
        
        assert result == "Transcribed text"

    @patch("voice_utils.openai.OpenAI")
    def test_synthesize_speech(self, mock_openai_class):
        """Test speech synthesis."""
        # Mock OpenAI client
        mock_client = MagicMock()
        mock_openai_class.return_value = mock_client
        mock_client.audio.speech.create.return_value = MagicMock(
            content=b"audio_data"
        )
        
        result = synthesize_speech(
            "Hello world",
            voice="nova",
            api_key="test_key"
        )
        
        assert result == b"audio_data"


# Test ai_service.py
from ai_service import (
    AIService,
    AIProvider,
    get_ai_response,
    analyze_sentiment,
    summarize_text,
    extract_entities,
)


class TestAIService:
    """Test AI service utilities."""

    @patch("ai_service.openai.OpenAI")
    def test_ai_service_openai(self, mock_openai_class):
        """Test AI service with OpenAI provider."""
        mock_client = MagicMock()
        mock_openai_class.return_value = mock_client
        
        response = MagicMock()
        response.choices = [MagicMock(message=MagicMock(content="AI response"))]
        mock_client.chat.completions.create.return_value = response
        
        service = AIService(
            provider=AIProvider.OPENAI,
            api_key="test_key"
        )
        
        result = service.get_response("Hello")
        
        assert result == "AI response"

    def test_analyze_sentiment(self):
        """Test sentiment analysis."""
        with patch("ai_service.AIService") as mock_service_class:
            mock_service = MagicMock()
            mock_service_class.return_value = mock_service
            mock_service.analyze_sentiment.return_value = {
                "sentiment": "positive",
                "score": 0.85
            }
            
            result = analyze_sentiment("I love this!")
            
            assert result["sentiment"] == "positive"
            assert result["score"] == 0.85

    def test_summarize_text(self):
        """Test text summarization."""
        with patch("ai_service.AIService") as mock_service_class:
            mock_service = MagicMock()
            mock_service_class.return_value = mock_service
            mock_service.summarize.return_value = "Short summary"
            
            long_text = "This is a very long text " * 50
            result = summarize_text(long_text, max_length=100)
            
            assert result == "Short summary"

    def test_extract_entities(self):
        """Test entity extraction."""
        with patch("ai_service.AIService") as mock_service_class:
            mock_service = MagicMock()
            mock_service_class.return_value = mock_service
            mock_service.extract_entities.return_value = {
                "people": ["John Doe"],
                "organizations": ["OpenAI"],
                "locations": ["San Francisco"]
            }
            
            text = "John Doe from OpenAI in San Francisco"
            result = extract_entities(text)
            
            assert "John Doe" in result["people"]
            assert "OpenAI" in result["organizations"]


# Test bot_analytics.py
from bot_analytics import (
    MessageAnalytics,
    UserActivity,
    ChatStatistics,
    calculate_engagement_rate,
    find_peak_hours,
    generate_report,
)


class TestBotAnalytics:
    """Test bot analytics utilities."""

    def test_calculate_engagement_rate(self):
        """Test engagement rate calculation."""
        messages_sent = 100
        reactions_received = 250
        replies_received = 50
        
        rate = calculate_engagement_rate(
            messages_sent,
            reactions_received,
            replies_received
        )
        
        assert rate == 3.0  # (250 + 50) / 100

    def test_find_peak_hours(self):
        """Test finding peak activity hours."""
        timestamps = [
            datetime(2025, 1, 1, 9, 0, 0),
            datetime(2025, 1, 1, 9, 30, 0),
            datetime(2025, 1, 1, 10, 0, 0),
            datetime(2025, 1, 1, 14, 0, 0),
            datetime(2025, 1, 1, 14, 30, 0),
            datetime(2025, 1, 1, 14, 45, 0),
        ]
        
        peak_hours = find_peak_hours(timestamps)
        
        assert 14 in peak_hours  # Hour 14 has 3 messages
        assert 9 in peak_hours   # Hour 9 has 2 messages

    def test_message_analytics(self):
        """Test MessageAnalytics class."""
        analytics = MessageAnalytics()
        
        # Add messages
        analytics.add_message(
            sender_id=1,
            timestamp=datetime.now(),
            reactions=5,
            is_reply=False
        )
        analytics.add_message(
            sender_id=2,
            timestamp=datetime.now(),
            reactions=10,
            is_reply=True
        )
        
        stats = analytics.get_statistics()
        
        assert stats["total_messages"] == 2
        assert stats["total_reactions"] == 15
        assert stats["reply_rate"] == 0.5

    def test_generate_report(self, tmp_path):
        """Test report generation."""
        stats = {
            "total_messages": 1000,
            "unique_users": 50,
            "engagement_rate": 2.5,
            "peak_hours": [9, 14, 20]
        }
        
        report_file = tmp_path / "report.json"
        generate_report(stats, str(report_file))
        
        assert report_file.exists()
        
        loaded = json.loads(report_file.read_text())
        assert loaded["total_messages"] == 1000
        assert loaded["unique_users"] == 50


# Test telegram_bot_base.py
from telegram_bot_base import (
    BotCommand,
    BotHandler,
    TelegramBotBase,
    command_handler,
    message_handler,
    callback_handler,
)


class TestTelegramBotBase:
    """Test Telegram bot base class."""

    def test_bot_command(self):
        """Test BotCommand dataclass."""
        cmd = BotCommand(
            name="start",
            description="Start the bot",
            handler=lambda msg: "Started"
        )
        
        assert cmd.name == "start"
        assert cmd.description == "Start the bot"
        assert cmd.handler("test") == "Started"

    def test_command_handler_decorator(self):
        """Test command handler decorator."""
        @command_handler("help")
        def help_handler(message):
            return "Help message"
        
        assert hasattr(help_handler, "_command")
        assert help_handler._command == "help"

    def test_message_handler_decorator(self):
        """Test message handler decorator."""
        @message_handler(lambda msg: "test" in msg.text)
        def test_handler(message):
            return "Test found"
        
        assert hasattr(test_handler, "_filter")
        assert test_handler._filter(MagicMock(text="test message"))

    @patch("telegram_bot_base.TelegramClient")
    def test_telegram_bot_base(self, mock_client_class):
        """Test TelegramBotBase class."""
        bot = TelegramBotBase(
            api_id=12345,
            api_hash="test_hash",
            bot_token="bot_token"
        )
        
        # Register handler
        @bot.command("start")
        async def start_handler(event):
            await event.reply("Started!")
        
        assert "start" in bot._commands
        assert bot._commands["start"] == start_handler