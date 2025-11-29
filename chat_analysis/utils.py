"""Utility functions for chat analysis."""

import os
from pathlib import Path
from datetime import datetime
from typing import Optional


def ensure_dir(path: Path) -> None:
    """Ensure directory exists.

    Args:
        path: Directory or file path
    """
    if path.suffix:  # It's a file path
        path.parent.mkdir(parents=True, exist_ok=True)
    else:  # It's a directory path
        path.mkdir(parents=True, exist_ok=True)


def parse_datetime(date_str: str, default: Optional[datetime] = None) -> datetime:
    """Parse datetime from ISO format string.

    Args:
        date_str: ISO format datetime string
        default: Default value if parsing fails (defaults to now())

    Returns:
        Parsed datetime or default
    """
    try:
        return datetime.fromisoformat(date_str)
    except (ValueError, TypeError, AttributeError):
        return default or datetime.now()


def load_prompt_template(prompt_name: str) -> str:
    """Load prompt template from prompts directory.

    Args:
        prompt_name: Name of prompt file (without .md extension)

    Returns:
        Prompt template content

    Raises:
        FileNotFoundError: If prompt file not found
    """
    prompts_dir = os.getenv("PROMPTS_DIR", "prompts")
    prompt_path = Path(prompts_dir) / f"{prompt_name}.md"

    if not prompt_path.exists():
        raise FileNotFoundError(f"Prompt not found: {prompt_path}")

    return prompt_path.read_text(encoding="utf-8")


class VerboseLogger:
    """Simple logger that respects verbose flag."""

    def __init__(self, verbose: bool = True):
        """Initialize logger.

        Args:
            verbose: Whether to print messages
        """
        self.verbose = verbose

    def log(self, message: str) -> None:
        """Log message if verbose is enabled.

        Args:
            message: Message to log
        """
        if self.verbose:
            print(message)

    def info(self, message: str) -> None:
        """Log info message."""
        self.log(message)

    def error(self, message: str) -> None:
        """Log error message (always shown)."""
        print(f"Error: {message}")

    def success(self, message: str) -> None:
        """Log success message."""
        self.log(f"✅ {message}")

    def warning(self, message: str) -> None:
        """Log warning message."""
        self.log(f"⚠️  {message}")
