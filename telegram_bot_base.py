"""Base class for Telegram bots following SOLID principles."""

import os
from abc import ABC, abstractmethod
from pathlib import Path
from typing import List, Optional, Set

from dotenv import load_dotenv
from telethon import TelegramClient

from telegram_session import SessionLock, get_client


class TelegramBotBase(ABC):
    """
    Base class for Telegram bots implementing common patterns (SOLID: SRP, OCP).
    
    Responsibilities:
    - Client initialization and lifecycle management
    - Session locking
    - Access control
    - Environment configuration
    
    Subclasses implement specific bot behavior via abstract methods.
    """

    def __init__(
        self,
        *,
        bot_token: Optional[str] = None,
        allowed_users: Optional[List[int]] = None,
        use_session_lock: bool = True,
    ):
        """
        Initialize bot with common setup.
        
        Args:
            bot_token: Telegram bot token (if None, uses user session)
            allowed_users: List of allowed user IDs (if None, allows all)
            use_session_lock: Whether to use session lock for concurrency control
        """
        load_dotenv()
        
        self.bot_token = bot_token
        self.allowed_users: Set[int] = set(allowed_users) if allowed_users else set()
        self.use_session_lock = use_session_lock
        self.client: Optional[TelegramClient] = None
        self._session_lock: Optional[SessionLock] = None

    def _init_client(self) -> TelegramClient:
        """Initialize Telegram client (user session or bot)."""
        if self.bot_token:
            # Bot mode
            api_id = int(os.getenv("TELEGRAM_API_ID"))
            api_hash = os.getenv("TELEGRAM_API_HASH")
            bot_name = self.__class__.__name__.lower()
            return TelegramClient(bot_name, api_id, api_hash)
        else:
            # User session mode
            return get_client()

    def check_access(self, user_id: int) -> bool:
        """
        Check if user has access to bot (SOLID: SRP).
        
        Args:
            user_id: Telegram user ID
            
        Returns:
            True if user is allowed, False otherwise
        """
        if not self.allowed_users:
            return True  # No restrictions
        return user_id in self.allowed_users

    @abstractmethod
    async def setup_handlers(self):
        """
        Setup bot event handlers (SOLID: OCP - open for extension).
        
        Subclasses must implement this to register their specific handlers.
        """
        pass

    @abstractmethod
    async def on_start(self):
        """
        Called when bot starts (SOLID: OCP - open for extension).
        
        Use for initialization, logging, announcements, etc.
        """
        pass

    async def run(self):
        """
        Main entry point to run the bot (Template Method pattern).
        
        Handles:
        1. Client initialization
        2. Session locking (if enabled)
        3. Handler setup
        4. Startup callback
        5. Running until disconnected
        """
        self.client = self._init_client()

        if self.use_session_lock:
            with SessionLock():
                await self._run_with_client()
        else:
            await self._run_with_client()

    async def _run_with_client(self):
        """Internal method to run bot with initialized client."""
        async with self.client:
            if self.bot_token:
                await self.client.start(bot_token=self.bot_token)
            
            await self.setup_handlers()
            await self.on_start()
            await self.client.run_until_disconnected()


class ConfigurableTelegramBot(TelegramBotBase):
    """
    Extended base class with config file support (SOLID: SRP).
    
    Adds configuration management separate from bot logic.
    """

    def __init__(
        self,
        *,
        config_path: str = "config.yml",
        **kwargs
    ):
        """
        Initialize bot with config file support.
        
        Args:
            config_path: Path to YAML config file
            **kwargs: Passed to TelegramBotBase
        """
        super().__init__(**kwargs)
        self.config_path = Path(config_path)
        self.config: dict = {}

    def load_config(self) -> dict:
        """
        Load configuration from YAML file (SOLID: SRP).
        
        Returns:
            Configuration dictionary
        """
        if not self.config_path.exists():
            return {}
        
        import yaml
        with self.config_path.open("r", encoding="utf-8") as f:
            self.config = yaml.safe_load(f) or {}
        
        return self.config

    def get_config_value(self, key: str, default=None):
        """
        Get configuration value by key (SOLID: SRP).
        
        Args:
            key: Configuration key (supports dot notation, e.g., "openai.model")
            default: Default value if key not found
            
        Returns:
            Configuration value or default
        """
        if not self.config:
            self.load_config()
        
        keys = key.split(".")
        value = self.config
        
        for k in keys:
            if isinstance(value, dict):
                value = value.get(k)
            else:
                return default
        
        return value if value is not None else default
