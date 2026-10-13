"""
System prompt loader from files.

Prompts are stored in the project root `prompts/` directory.
"""

from enum import Enum
from pathlib import Path
from typing import Optional


class Prompt(Enum):
    """Available prompts (Markdown format)."""

    SALES_AGENT = "sales_agent.md"
    CALCULATOR = "calculator.md"
    FRIENDLY_AI = "friendly_ai.md"
    MODERATOR = "moderator.md"
    DIGEST = "digest.md"
    CRM_PARSER = "crm_parser.md"


def get_prompts_dir() -> Path:
    """Get the path to the prompts directory."""
    candidates = [
        Path("prompts"),
        Path("../prompts"),
        Path(__file__).parent.parent / "prompts",
    ]

    for path in candidates:
        if path.exists():
            return path

    return Path("prompts")


def load_prompt(prompt: Prompt | str, context: Optional[str] = None) -> str:
    """
    Load a prompt from a file.

    Args:
        prompt: Prompt enum or filename
        context: Additional context to append to the prompt

    Returns:
        Prompt text

    Raises:
        FileNotFoundError: If the file is not found
    """
    filename = prompt.value if isinstance(prompt, Prompt) else prompt
    path = get_prompts_dir() / filename

    content = path.read_text(encoding="utf-8")

    if context:
        content = f"{content}\n\nКонтекст: {context}"

    return content


def list_prompts() -> list[Prompt]:
    """List all available prompts."""
    return list(Prompt)


# Quick access to prompts.
SALES_AGENT = Prompt.SALES_AGENT
CALCULATOR = Prompt.CALCULATOR
FRIENDLY_AI = Prompt.FRIENDLY_AI
MODERATOR = Prompt.MODERATOR
DIGEST = Prompt.DIGEST
CRM_PARSER = Prompt.CRM_PARSER


if __name__ == "__main__":
    # Loading test.
    print("Доступные промпты:")
    for p in list_prompts():
        print(f"  - {p.name}: {p.value}")

    print("\n--- Sales Agent ---")
    print(load_prompt(Prompt.SALES_AGENT))
