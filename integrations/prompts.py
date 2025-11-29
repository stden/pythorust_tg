"""
Загрузчик системных промптов из файлов.

Промпты хранятся в каталоге `prompts/` в корне проекта.
"""

from enum import Enum
from pathlib import Path
from typing import Optional


class Prompt(Enum):
    """Доступные промпты (Markdown формат)."""

    SALES_AGENT = "sales_agent.md"
    CALCULATOR = "calculator.md"
    FRIENDLY_AI = "friendly_ai.md"
    MODERATOR = "moderator.md"
    DIGEST = "digest.md"
    CRM_PARSER = "crm_parser.md"


def get_prompts_dir() -> Path:
    """Получить путь к каталогу промптов."""
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
    Загрузить промпт из файла.

    Args:
        prompt: Enum промпта или имя файла
        context: Дополнительный контекст для добавления в конец промпта

    Returns:
        Текст промпта

    Raises:
        FileNotFoundError: Если файл не найден
    """
    filename = prompt.value if isinstance(prompt, Prompt) else prompt
    path = get_prompts_dir() / filename

    content = path.read_text(encoding="utf-8")

    if context:
        content = f"{content}\n\nКонтекст: {context}"

    return content


def list_prompts() -> list[Prompt]:
    """Список всех доступных промптов."""
    return list(Prompt)


# Быстрый доступ к промптам
SALES_AGENT = Prompt.SALES_AGENT
CALCULATOR = Prompt.CALCULATOR
FRIENDLY_AI = Prompt.FRIENDLY_AI
MODERATOR = Prompt.MODERATOR
DIGEST = Prompt.DIGEST
CRM_PARSER = Prompt.CRM_PARSER


if __name__ == "__main__":
    # Тест загрузки
    print("Доступные промпты:")
    for p in list_prompts():
        print(f"  - {p.name}: {p.value}")

    print("\n--- Sales Agent ---")
    print(load_prompt(Prompt.SALES_AGENT))
