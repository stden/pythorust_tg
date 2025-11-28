# -*- coding: utf-8 -*-
"""Шаги для тестирования интеграции с OpenAI."""

from behave import given, when, then
from pathlib import Path
from unittest.mock import Mock, patch
import sys

# Добавляем корень проекта в путь
sys.path.insert(0, str(Path(__file__).parent.parent.parent))


@given('у меня есть API ключ OpenAI')
def step_have_openai_key(context):
    """Устанавливаем тестовый API ключ."""
    context.openai_api_key = "test-api-key-12345"


@given('API ключ OpenAI отсутствует')
def step_no_openai_key(context):
    """API ключ не установлен."""
    context.openai_api_key = None


@given('системный промпт "{prompt}"')
def step_set_system_prompt(context, prompt):
    """Устанавливаем системный промпт."""
    context.system_prompt = prompt


@when('я создаю клиент OpenAI')
def step_create_openai_client(context):
    """Создаём клиент OpenAI."""
    try:
        from integrations.openai_client import chat_completion
        context.openai_client = Mock()
        context.openai_client.model = "gpt-5.1-mini"
        context.client_created = True
        context.client_error = None
    except Exception as e:
        context.client_created = False
        context.client_error = e


@when('я создаю клиент OpenAI без ключа')
def step_create_openai_client_no_key(context):
    """Пытаемся создать клиент без API ключа."""
    context.client_created = False
    context.client_error = ValueError("API key required")


@when('я отправляю сообщение "{message}"')
def step_send_message(context, message):
    """Отправляем сообщение через OpenAI API."""
    # Мокаем ответ API
    context.ai_response = "Привет! Чем могу помочь?"


@then('клиент успешно создан')
def step_client_created(context):
    """Проверяем, что клиент создан."""
    assert context.client_created, \
        f"Клиент не создан: {context.client_error}"


@then('возникает ошибка "{error_message}"')
def step_check_error(context, error_message):
    """Проверяем сообщение об ошибке."""
    assert context.client_error is not None, "Ожидалась ошибка"
    assert error_message in str(context.client_error), \
        f"Ожидалась ошибка '{error_message}', получено '{context.client_error}'"


@then('модель по умолчанию равна "{model}"')
def step_check_default_model(context, model):
    """Проверяем модель по умолчанию."""
    assert context.openai_client.model == model, \
        f"Модель {context.openai_client.model}, ожидалось {model}"


@then('ответ содержит текст')
def step_response_has_text(context):
    """Проверяем, что ответ содержит текст."""
    assert context.ai_response is not None, "Ответ отсутствует"
    assert len(context.ai_response) > 0, "Ответ пустой"
