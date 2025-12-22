"""OpenAI service abstraction following SOLID principles (SRP, DIP)."""

import os
from abc import ABC, abstractmethod
from typing import List, Dict, Optional

import openai
from dotenv import load_dotenv


class AIService(ABC):
    """
    Abstract interface for AI services (SOLID: DIP - Dependency Inversion).

    Allows swapping AI providers without changing dependent code.
    """

    @abstractmethod
    async def chat_completion(
        self, messages: List[Dict[str, str]], model: Optional[str] = None, temperature: float = 0.7, **kwargs
    ) -> str:
        """
        Get AI completion for a conversation.

        Args:
            messages: List of message dicts with 'role' and 'content'
            model: AI model to use (provider-specific)
            temperature: Sampling temperature (0.0 to 2.0)
            **kwargs: Additional provider-specific parameters

        Returns:
            AI response text
        """
        pass


class OpenAIService(AIService):
    """
    OpenAI API service implementation (SOLID: SRP).

    Responsibilities:
    - OpenAI API authentication
    - Chat completions
    - Error handling
    """

    def __init__(
        self,
        api_key: Optional[str] = None,
        default_model: str = "gpt-4o-mini",
        default_temperature: float = 0.7,
    ):
        """
        Initialize OpenAI service.

        Args:
            api_key: OpenAI API key (if None, reads from env)
            default_model: Default model to use
            default_temperature: Default temperature
        """
        load_dotenv()

        self.api_key = api_key or os.getenv("OPENAI_API_KEY")
        if not self.api_key:
            raise ValueError(
                "OpenAI API key required. Set OPENAI_API_KEY environment variable or pass api_key parameter."
            )

        openai.api_key = self.api_key
        self.default_model = default_model
        self.default_temperature = default_temperature

    async def chat_completion(
        self, messages: List[Dict[str, str]], model: Optional[str] = None, temperature: Optional[float] = None, **kwargs
    ) -> str:
        """
        Get OpenAI chat completion (SOLID: SRP).

        Args:
            messages: List of message dicts with 'role' and 'content'
            model: Model to use (overrides default)
            temperature: Temperature to use (overrides default)
            **kwargs: Additional OpenAI-specific parameters

        Returns:
            AI response text

        Raises:
            Exception: If API call fails
        """
        try:
            response = openai.ChatCompletion.create(
                model=model or self.default_model,
                messages=messages,
                temperature=temperature if temperature is not None else self.default_temperature,
                **kwargs,
            )
            return response.choices[0].message.content.strip()
        except Exception as e:
            raise Exception(f"OpenAI API error: {e}") from e

    def build_system_message(self, instructions: str) -> Dict[str, str]:
        """
        Build system message dict (SOLID: KISS - Keep It Simple).

        Args:
            instructions: System instructions text

        Returns:
            Message dict
        """
        return {"role": "system", "content": instructions}

    def build_user_message(self, content: str) -> Dict[str, str]:
        """
        Build user message dict (SOLID: KISS).

        Args:
            content: User message content

        Returns:
            Message dict
        """
        return {"role": "user", "content": content}

    def build_assistant_message(self, content: str) -> Dict[str, str]:
        """
        Build assistant message dict (SOLID: KISS).

        Args:
            content: Assistant message content

        Returns:
            Message dict
        """
        return {"role": "assistant", "content": content}


class ConversationManager:
    """
    Manages conversation history (SOLID: SRP).

    Responsibilities:
    - Storing message history
    - Context window management
    - History persistence (future)
    """

    def __init__(
        self,
        system_instructions: Optional[str] = None,
        max_history: int = 50,
    ):
        """
        Initialize conversation manager.

        Args:
            system_instructions: System instructions for AI
            max_history: Maximum messages to keep in history
        """
        self.system_instructions = system_instructions
        self.max_history = max_history
        self.messages: List[Dict[str, str]] = []

        if system_instructions:
            self.messages.append({"role": "system", "content": system_instructions})

    def add_user_message(self, content: str):
        """Add user message to history."""
        self.messages.append({"role": "user", "content": content})
        self._trim_history()

    def add_assistant_message(self, content: str):
        """Add assistant message to history."""
        self.messages.append({"role": "assistant", "content": content})
        self._trim_history()

    def get_messages(self) -> List[Dict[str, str]]:
        """Get current conversation history."""
        return self.messages.copy()

    def clear_history(self, keep_system: bool = True):
        """
        Clear conversation history.

        Args:
            keep_system: If True, keeps system message
        """
        if keep_system and self.system_instructions:
            self.messages = [{"role": "system", "content": self.system_instructions}]
        else:
            self.messages = []

    def _trim_history(self):
        """Trim history to max_history, keeping system message."""
        if len(self.messages) <= self.max_history:
            return

        # Keep system message if present
        system_msg = None
        if self.messages and self.messages[0]["role"] == "system":
            system_msg = self.messages[0]
            messages = self.messages[1:]
        else:
            messages = self.messages

        # Trim to max
        messages = messages[-(self.max_history - (1 if system_msg else 0)) :]

        # Rebuild
        if system_msg:
            self.messages = [system_msg] + messages
        else:
            self.messages = messages
