"""LLM provider implementations using Strategy Pattern."""

import os
from abc import ABC, abstractmethod
from typing import Dict

from .config import AnalyzerConfig


class LLMProvider(ABC):
    """Abstract base class for LLM providers."""

    @abstractmethod
    def call(self, prompt: str, config: AnalyzerConfig) -> str:
        """Call LLM API with prompt.

        Args:
            prompt: Analysis prompt
            config: Analyzer configuration

        Returns:
            LLM response text
        """
        pass


class OpenAIProvider(LLMProvider):
    """OpenAI GPT provider."""

    def __init__(self, api_key: str):
        """Initialize OpenAI provider.

        Args:
            api_key: OpenAI API key
        """
        import openai

        self.client = openai.OpenAI(api_key=api_key)

    def call(self, prompt: str, config: AnalyzerConfig) -> str:
        """Call OpenAI API."""
        response = self.client.chat.completions.create(
            model=config.model,
            messages=[
                {"role": "system", "content": "Ты эксперт по анализу чатов. Всегда отвечай в валидном JSON формате."},
                {"role": "user", "content": prompt},
            ],
            temperature=config.temperature,
            max_tokens=config.max_tokens,
        )
        return response.choices[0].message.content


class ClaudeProvider(LLMProvider):
    """Anthropic Claude provider."""

    def __init__(self, api_key: str):
        """Initialize Claude provider.

        Args:
            api_key: Anthropic API key
        """
        import anthropic

        self.client = anthropic.Anthropic(api_key=api_key)

    def call(self, prompt: str, config: AnalyzerConfig) -> str:
        """Call Claude API."""
        response = self.client.messages.create(
            model=config.model,
            max_tokens=config.max_tokens,
            temperature=config.temperature,
            messages=[{"role": "user", "content": prompt}],
        )
        return response.content[0].text


class GeminiProvider(LLMProvider):
    """Google Gemini provider."""

    def __init__(self, api_key: str):
        """Initialize Gemini provider.

        Args:
            api_key: Google API key
        """
        import google.generativeai as genai

        genai.configure(api_key=api_key)
        self.model = None  # Will be set in call()

    def call(self, prompt: str, config: AnalyzerConfig) -> str:
        """Call Gemini API."""
        import google.generativeai as genai

        if not self.model or self.model._model_name != config.model:
            self.model = genai.GenerativeModel(config.model)

        response = self.model.generate_content(
            prompt, generation_config={"temperature": config.temperature, "max_output_tokens": config.max_tokens}
        )
        return response.text


class LLMProviderFactory:
    """Factory for creating LLM providers."""

    _providers: Dict[str, type] = {
        "openai": OpenAIProvider,
        "claude": ClaudeProvider,
        "gemini": GeminiProvider,
    }

    @classmethod
    def create(cls, provider_name: str) -> LLMProvider:
        """Create LLM provider instance.

        Args:
            provider_name: Provider name (openai, claude, gemini)

        Returns:
            LLM provider instance

        Raises:
            ValueError: If provider is not supported
        """
        if provider_name not in cls._providers:
            raise ValueError(
                f"Unsupported LLM provider: {provider_name}. Supported: {', '.join(cls._providers.keys())}"
            )

        provider_class = cls._providers[provider_name]

        # Get API key from environment
        api_key = cls._get_api_key(provider_name)
        if not api_key:
            raise ValueError(
                f"API key not found for {provider_name}. "
                f"Set {cls._get_env_var_name(provider_name)} environment variable."
            )

        return provider_class(api_key)

    @staticmethod
    def _get_api_key(provider_name: str) -> str:
        """Get API key from environment for provider."""
        env_vars = {
            "openai": "OPENAI_API_KEY",
            "claude": "ANTHROPIC_API_KEY",
            "gemini": "GOOGLE_API_KEY",
        }
        return os.getenv(env_vars.get(provider_name, ""))

    @staticmethod
    def _get_env_var_name(provider_name: str) -> str:
        """Get environment variable name for provider."""
        env_vars = {
            "openai": "OPENAI_API_KEY",
            "claude": "ANTHROPIC_API_KEY",
            "gemini": "GOOGLE_API_KEY",
        }
        return env_vars.get(provider_name, "")

    @classmethod
    def register_provider(cls, name: str, provider_class: type):
        """Register a new LLM provider.

        Args:
            name: Provider name
            provider_class: Provider class implementing LLMProvider
        """
        cls._providers[name] = provider_class
