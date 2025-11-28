# -*- coding: utf-8 -*-
"""Шаги для тестирования системы промптов."""

from behave import given, when, then
from pathlib import Path
import sys

# Добавляем корень проекта в путь
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from integrations.prompts import load_prompt, list_prompts, Prompt


@given('существует файл промпта "{filename}"')
def step_prompt_file_exists(context, filename):
    """Проверяем существование файла промпта."""
    prompts_dir = Path(__file__).parent.parent.parent / "prompts"
    filepath = prompts_dir / filename
    assert filepath.exists(), f"Файл {filename} не найден в {prompts_dir}"
    context.prompt_file = filepath


@when('я загружаю промпт "{prompt_name}"')
def step_load_prompt(context, prompt_name):
    """Загружаем промпт по имени."""
    try:
        # Преобразуем имя в enum
        prompt_enum = getattr(Prompt, prompt_name.upper(), None)
        if prompt_enum:
            context.prompt_content = load_prompt(prompt_enum)
        else:
            context.prompt_content = load_prompt(prompt_name)
        context.load_error = None
    except Exception as e:
        context.prompt_content = None
        context.load_error = e


@when('я запрашиваю список промптов')
def step_list_prompts(context):
    """Получаем список всех промптов."""
    context.prompts_list = list_prompts()


@then('промпт содержит текст "{text}"')
def step_prompt_contains(context, text):
    """Проверяем, что промпт содержит указанный текст."""
    assert context.prompt_content is not None, "Промпт не загружен"
    assert text.lower() in context.prompt_content.lower(), \
        f"Текст '{text}' не найден в промпте"


@then('список содержит не менее {count:d} промптов')
def step_prompts_count(context, count):
    """Проверяем количество промптов."""
    assert len(context.prompts_list) >= count, \
        f"Найдено {len(context.prompts_list)} промптов, ожидалось >= {count}"


@then('возникает ошибка загрузки')
def step_load_error(context):
    """Проверяем, что произошла ошибка загрузки."""
    assert context.load_error is not None, "Ожидалась ошибка загрузки"
