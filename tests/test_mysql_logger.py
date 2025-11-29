"""Unit tests for MySQLLogger in bfl_sales_bot."""

from unittest.mock import MagicMock, patch
import pytest
from bfl_sales_bot import MySQLLogger
from types import SimpleNamespace

@pytest.fixture
def logger_with_mocks():
    """Create MySQLLogger with mocked pymysql connection."""
    with patch("bfl_sales_bot.pymysql") as mock_pymysql:
        mock_conn = MagicMock()
        mock_cursor = MagicMock()
        mock_conn.cursor.return_value.__enter__.return_value = mock_cursor
        mock_pymysql.connect.return_value = mock_conn
        
        logger = MySQLLogger()
        logger.connect()
        
        return logger, mock_cursor, mock_conn

def test_save_user(logger_with_mocks):
    """Test saving a user executes the correct SQL."""
    logger, cursor, conn = logger_with_mocks
    
    user = SimpleNamespace(
        id=123,
        username="testuser",
        first_name="Test",
        last_name="User",
        lang_code="en",
        premium=True,
        bot=False
    )
    
    logger.save_user(user)
    
    cursor.execute.assert_called_once()
    args = cursor.execute.call_args[0]
    assert "INSERT INTO bot_users" in args[0]
    assert args[1] == (123, "testuser", "Test", "User", "en", True, False)
    conn.commit.assert_called_once()

def test_save_message(logger_with_mocks):
    """Test saving a message executes the correct SQL."""
    logger, cursor, conn = logger_with_mocks
    
    logger.save_message(
        user_id=123,
        message_id=456,
        text="Hello",
        direction="incoming"
    )
    
    cursor.execute.assert_called_once()
    args = cursor.execute.call_args[0]
    assert "INSERT INTO bot_messages" in args[0]
    # Check params (message_id, user_id, bot_name, direction, text, reply_to)
    # Note: bot_name defaults to "BFL_sales_bot"
    assert args[1] == (456, 123, "BFL_sales_bot", "incoming", "Hello", None)
    conn.commit.assert_called_once()

def test_create_session(logger_with_mocks):
    """Test creating a session ends old ones and starts a new one."""
    logger, cursor, conn = logger_with_mocks
    cursor.lastrowid = 100
    
    session_id = logger.create_session(user_id=123)
    
    assert session_id == 100
    assert cursor.execute.call_count == 2
    
    # First call updates old sessions
    update_call = cursor.execute.call_args_list[0]
    assert "UPDATE bot_sessions" in update_call[0][0]
    assert update_call[0][1] == (123, "BFL_sales_bot")
    
    # Second call inserts new session
    insert_call = cursor.execute.call_args_list[1]
    assert "INSERT INTO bot_sessions" in insert_call[0][0]
    assert insert_call[0][1] == (123, "BFL_sales_bot")
    
    conn.commit.assert_called_once()

def test_get_conversation_history(logger_with_mocks):
    """Test fetching history returns reversed list of messages."""
    logger, cursor, _ = logger_with_mocks
    
    # Mock return values from DB (direction, text, created_at)
    # DB usually returns oldest first if ordered by created_at ASC, 
    # but the query in code orders by created_at DESC (newest first).
    # The code then reverses it to get chronological order.
    
    mock_rows = [
        {"direction": "outgoing", "message_text": "Reply", "created_at": "2023-01-01 10:01:00"},
        {"direction": "incoming", "message_text": "Hi", "created_at": "2023-01-01 10:00:00"},
    ]
    cursor.fetchall.return_value = mock_rows
    
    history = logger.get_conversation_history(user_id=123, limit=5)
    
    cursor.execute.assert_called_once()
    assert "SELECT direction, message_text" in cursor.execute.call_args[0][0]
    assert "ORDER BY created_at DESC" in cursor.execute.call_args[0][0]
    
    # Verify the result is reversed (chronological order: Incoming -> Outgoing)
    assert len(history) == 2
    assert history[0]["message_text"] == "Hi"
    assert history[1]["message_text"] == "Reply"
