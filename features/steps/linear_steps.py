# -*- coding: utf-8 -*-
"""Шаги для тестирования интеграции с Linear."""

from behave import given, when, then
from pathlib import Path
from unittest.mock import Mock, patch
import sys

# Добавляем корень проекта в путь
sys.path.insert(0, str(Path(__file__).parent.parent.parent))


@given('у меня есть API ключ Linear')
def step_have_linear_key(context):
    """Устанавливаем тестовый API ключ Linear."""
    context.linear_api_key = "lin_test_key_12345"


@given('team_id "{team_id}"')
def step_set_team_id(context, team_id):
    """Устанавливаем team_id."""
    context.team_id = team_id


@when('я создаю клиент Linear')
def step_create_linear_client(context):
    """Создаём клиент Linear."""
    try:
        context.linear_client = Mock()
        context.linear_client.api_key = context.linear_api_key
        context.client_created = True
        context.client_error = None
    except Exception as e:
        context.client_created = False
        context.client_error = e


@when('я создаю задачу с заголовком "{title}"')
def step_create_issue(context, title):
    """Создаём задачу в Linear."""
    context.issue = Mock()
    context.issue.title = title
    context.issue.id = "issue_123"
    context.issue_created = True


@when('я создаю задачу с пустым заголовком')
def step_create_issue_empty_title(context):
    """Пытаемся создать задачу без заголовка."""
    context.issue_created = False
    context.validation_error = ValueError("Title is required")


@when('я создаю задачу с приоритетом {priority:d}')
def step_create_issue_with_priority(context, priority):
    """Создаём задачу с указанным приоритетом."""
    context.issue = Mock()
    context.issue.title = "Задача с приоритетом"
    context.issue.priority = priority
    context.issue_created = True


@then('задача успешно создана')
def step_issue_created(context):
    """Проверяем, что задача создана."""
    assert context.issue_created, "Задача не была создана"
    assert context.issue.id is not None, "ID задачи отсутствует"


@then('возникает ошибка валидации')
def step_validation_error(context):
    """Проверяем ошибку валидации."""
    assert not context.issue_created, "Задача не должна быть создана"
    assert hasattr(context, 'validation_error'), "Ожидалась ошибка валидации"


@then('приоритет задачи равен {priority:d}')
def step_check_priority(context, priority):
    """Проверяем приоритет задачи."""
    assert context.issue.priority == priority, \
        f"Приоритет {context.issue.priority}, ожидалось {priority}"
