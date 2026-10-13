"""Configuration for chat analyzer."""

from dataclasses import dataclass
from pathlib import Path
import os


@dataclass
class AnalyzerConfig:
    """Configuration for ChatAnalyzer."""

    # Sampling parameters
    message_limit: int = 1000
    days_back: int = 30

    # LLM parameters
    llm_provider: str = "openai"
    model: str = None
    temperature: float = 0.3
    max_tokens: int = 2000

    # Filters
    min_message_length: int = 10
    include_media: bool = False
    exclude_bots: bool = True

    # Output
    output_format: str = "both"  # json, markdown, both
    output_dir: Path = None
    verbose: bool = True

    def __post_init__(self):
        """Load configuration from environment."""
        # LLM provider
        self.llm_provider = os.getenv("CHAT_ANALYZER_LLM_PROVIDER", self.llm_provider)

        # Model
        if not self.model:
            self.model = os.getenv("CHAT_ANALYZER_MODEL") or self._default_model()

        # Output directory - prioritize ANALYSIS_RESULTS_DIR from .env
        if self.output_dir is None:
            output_dir = (
                os.getenv("ANALYSIS_RESULTS_DIR") or os.getenv("CHAT_ANALYZER_OUTPUT_DIR") or "./analysis_results"
            )
            self.output_dir = Path(output_dir)

        # Ensure output_dir is Path
        if not isinstance(self.output_dir, Path):
            self.output_dir = Path(self.output_dir)

    def _default_model(self) -> str:
        """Get default model for provider."""
        defaults = {"openai": "gpt-4o-mini", "claude": "claude-sonnet-4-5-20250929", "gemini": "gemini-2.0-flash-exp"}
        return defaults.get(self.llm_provider, "gpt-4o-mini")
