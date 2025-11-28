# -*- coding: utf-8 -*-
"""Настройки окружения для behave тестов."""

import os
import sys
from pathlib import Path

# Добавляем корень проекта в путь
project_root = Path(__file__).parent.parent
sys.path.insert(0, str(project_root))


def before_all(context):
    """Выполняется перед всеми тестами."""
    context.project_root = project_root


def before_feature(context, feature):
    """Выполняется перед каждой функциональностью."""
    pass


def before_scenario(context, scenario):
    """Выполняется перед каждым сценарием."""
    pass


def after_scenario(context, scenario):
    """Выполняется после каждого сценария."""
    # Восстанавливаем переменные окружения
    if hasattr(context, 'original_env') and context.original_env:
        for var_name, value in context.original_env.items():
            if value is None:
                os.environ.pop(var_name, None)
            else:
                os.environ[var_name] = value
