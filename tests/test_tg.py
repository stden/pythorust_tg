"""Tests for tg.py module functions."""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
pytest.importorskip("telethon")

from chat_export_utils import load_chats_from_config
from telethon.tl.types import PeerChannel, PeerChat


class TestLoadChatsFromConfig:
    """Tests for load_chats_from_config function."""

    def test_load_chats_channel(self, tmp_path):
        """Test loading channel from config."""
        config_content = """
chats:
  test_channel:
    type: channel
    id: 1234567890
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        result = load_chats_from_config(str(config_file))
        assert "test_channel" in result
        assert isinstance(result["test_channel"], PeerChannel)

    def test_load_chats_group(self, tmp_path):
        """Test loading group from config."""
        config_content = """
chats:
  test_group:
    type: group
    id: 9876543210
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        result = load_chats_from_config(str(config_file))
        assert "test_group" in result
        assert isinstance(result["test_group"], PeerChat)

    def test_load_chats_username(self, tmp_path):
        """Test loading username from config."""
        config_content = """
chats:
  test_user:
    type: username
    username: testuser
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        result = load_chats_from_config(str(config_file))
        assert result["test_user"] == "@testuser"

    def test_load_chats_user_id(self, tmp_path):
        """Test loading user by ID from config."""
        config_content = """
chats:
  test_user_id:
    type: user
    id: 111222333
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        result = load_chats_from_config(str(config_file))
        assert result["test_user_id"] == 111222333

    def test_load_chats_unknown_type(self, tmp_path):
        """Test loading unknown chat type raises error."""
        config_content = """
chats:
  test_unknown:
    type: unknown
    id: 123
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        with pytest.raises(ValueError, match="Unknown chat type"):
            load_chats_from_config(str(config_file))

    def test_load_chats_username_with_at(self, tmp_path):
        """Test loading username that already has @ prefix."""
        config_content = """
chats:
  test_user:
    type: username
    username: "@testuser"
"""
        config_file = tmp_path / "config.yml"
        config_file.write_text(config_content)

        result = load_chats_from_config(str(config_file))
        assert result["test_user"] == "@testuser"
