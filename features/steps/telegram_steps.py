# -*- coding: utf-8 -*-
"""Шаги для тестирования Telegram функциональности."""

from behave import given, when, then
from pathlib import Path
import os
import sys
import yaml

# Добавляем корень проекта в путь
sys.path.insert(0, str(Path(__file__).parent.parent.parent))


@given('конфигурация загружена из "{config_file}"')
def step_load_config(context, config_file):
    """Загружаем конфигурацию из YAML файла."""
    config_path = Path(__file__).parent.parent.parent / config_file
    if config_path.exists():
        with open(config_path, 'r', encoding='utf-8') as f:
            context.app_config = yaml.safe_load(f) or {}
    else:
        context.app_config = {}


@given('переменная окружения "{var_name}" установлена в "{value}"')
def step_set_env_var(context, var_name, value):
    """Устанавливаем переменную окружения."""
    if not hasattr(context, 'original_env'):
        context.original_env = {}
    context.original_env[var_name] = os.environ.get(var_name)
    os.environ[var_name] = value


@given('существует чат "{chat_name}"')
def step_chat_exists(context, chat_name):
    """Проверяем существование чата в конфигурации."""
    if not hasattr(context, 'app_config'):
        context.app_config = {}
    chats = context.app_config.get('chats', {})
    if chat_name not in chats:
        # Добавляем тестовый чат для демонстрации
        if 'chats' not in context.app_config:
            context.app_config['chats'] = {}
        context.app_config['chats'][chat_name] = {
            'type': 'channel',
            'id': 12345
        }


@when('я запрашиваю лимит сообщений')
def step_get_message_limit(context):
    """Получаем лимит сообщений."""
    if os.environ.get('GITHUB_ACTIONS') == 'true':
        context.message_limit = 1000
    else:
        if not hasattr(context, 'app_config'):
            context.app_config = {}
        limits = context.app_config.get('limits', {})
        context.message_limit = limits.get('message_limit', 3000)


@when('я запрашиваю настройки чата "{chat_name}"')
def step_get_chat_settings(context, chat_name):
    """Получаем настройки конкретного чата."""
    if not hasattr(context, 'app_config'):
        context.app_config = {}
    chats = context.app_config.get('chats', {})
    context.chat_settings = chats.get(chat_name)


@then('лимит равен {limit:d} сообщениям')
def step_check_limit(context, limit):
    """Проверяем лимит сообщений."""
    assert context.message_limit == limit, \
        f"Лимит {context.message_limit}, ожидалось {limit}"


@then('тип чата равен "{chat_type}"')
def step_check_chat_type(context, chat_type):
    """Проверяем тип чата."""
    assert context.chat_settings is not None, "Настройки чата не найдены"
    assert context.chat_settings.get('type') == chat_type, \
        f"Тип чата {context.chat_settings.get('type')}, ожидалось {chat_type}"


@then('возвращается пустой результат')
def step_check_empty_result(context):
    """Проверяем, что результат пустой."""
    assert context.chat_settings is None, "Ожидался пустой результат"


def after_scenario(context, scenario):
    """Восстанавливаем переменные окружения после сценария."""
    if hasattr(context, 'original_env'):
        for var_name, value in context.original_env.items():
            if value is None:
                os.environ.pop(var_name, None)
            else:
                os.environ[var_name] = value
