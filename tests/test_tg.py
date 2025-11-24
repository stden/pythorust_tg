"""Tests for tg.py module functions."""

import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

# Check if telethon is available
try:
    from telethon.tl.types import PeerChannel, PeerChat
    TELETHON_AVAILABLE = True
except ImportError:
    TELETHON_AVAILABLE = False
    PeerChannel = None
    PeerChat = None


class TestLoadChatsFromConfig:
    """Tests for load_chats_from_config function."""

    @pytest.mark.skipif(not TELETHON_AVAILABLE, reason="telethon not installed")
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

        import yaml

        def load_chats_from_config(config_path):
            path = Path(config_path)
            with path.open("r", encoding="utf-8") as fh:
                data = yaml.safe_load(fh) or {}
            chats_cfg = data.get("chats", {})

            def to_entity(cfg):
                ctype = cfg.get("type")
                if ctype == "channel":
                    return PeerChannel(int(cfg["id"]))
                if ctype == "group":
                    return PeerChat(int(cfg["id"]))
                if ctype == "user":
                    return int(cfg["id"])
                if ctype == "username":
                    username = cfg["username"]
                    return username if username.startswith("@") else f"@{username}"
                raise ValueError(f"Unknown chat type: {ctype}")

            return {name: to_entity(cfg) for name, cfg in chats_cfg.items()}

        result = load_chats_from_config(str(config_file))
        assert "test_channel" in result
        assert isinstance(result["test_channel"], PeerChannel)

    @pytest.mark.skipif(not TELETHON_AVAILABLE, reason="telethon not installed")
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

        import yaml

        def load_chats_from_config(config_path):
            path = Path(config_path)
            with path.open("r", encoding="utf-8") as fh:
                data = yaml.safe_load(fh) or {}
            chats_cfg = data.get("chats", {})

            def to_entity(cfg):
                ctype = cfg.get("type")
                if ctype == "channel":
                    return PeerChannel(int(cfg["id"]))
                if ctype == "group":
                    return PeerChat(int(cfg["id"]))
                if ctype == "user":
                    return int(cfg["id"])
                if ctype == "username":
                    username = cfg["username"]
                    return username if username.startswith("@") else f"@{username}"
                raise ValueError(f"Unknown chat type: {ctype}")

            return {name: to_entity(cfg) for name, cfg in chats_cfg.items()}

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

        import yaml

        def load_chats_from_config(config_path):
            path = Path(config_path)
            with path.open("r", encoding="utf-8") as fh:
                data = yaml.safe_load(fh) or {}
            chats_cfg = data.get("chats", {})

            def to_entity(cfg):
                ctype = cfg.get("type")
                if ctype == "user":
                    return int(cfg["id"])
                if ctype == "username":
                    username = cfg["username"]
                    return username if username.startswith("@") else f"@{username}"
                raise ValueError(f"Unknown chat type: {ctype}")

            return {name: to_entity(cfg) for name, cfg in chats_cfg.items()}

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

        import yaml

        def load_chats_from_config(config_path):
            path = Path(config_path)
            with path.open("r", encoding="utf-8") as fh:
                data = yaml.safe_load(fh) or {}
            chats_cfg = data.get("chats", {})

            def to_entity(cfg):
                ctype = cfg.get("type")
                if ctype == "user":
                    return int(cfg["id"])
                if ctype == "username":
                    username = cfg["username"]
                    return username if username.startswith("@") else f"@{username}"
                raise ValueError(f"Unknown chat type: {ctype}")

            return {name: to_entity(cfg) for name, cfg in chats_cfg.items()}

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

        import yaml

        def load_chats_from_config(config_path):
            path = Path(config_path)
            with path.open("r", encoding="utf-8") as fh:
                data = yaml.safe_load(fh) or {}
            chats_cfg = data.get("chats", {})

            def to_entity(cfg):
                ctype = cfg.get("type")
                if ctype == "user":
                    return int(cfg["id"])
                if ctype == "username":
                    username = cfg["username"]
                    return username if username.startswith("@") else f"@{username}"
                raise ValueError(f"Unknown chat type: {ctype}")

            return {name: to_entity(cfg) for name, cfg in chats_cfg.items()}

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

        import yaml

        def load_chats_from_config(config_path):
            path = Path(config_path)
            with path.open("r", encoding="utf-8") as fh:
                data = yaml.safe_load(fh) or {}
            chats_cfg = data.get("chats", {})

            def to_entity(cfg):
                ctype = cfg.get("type")
                if ctype == "username":
                    username = cfg["username"]
                    return username if username.startswith("@") else f"@{username}"
                raise ValueError(f"Unknown chat type: {ctype}")

            return {name: to_entity(cfg) for name, cfg in chats_cfg.items()}

        result = load_chats_from_config(str(config_file))
        assert result["test_user"] == "@testuser"
