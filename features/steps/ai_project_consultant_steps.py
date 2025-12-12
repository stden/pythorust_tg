# -*- coding: utf-8 -*-
"""Шаги Behave для AI Project Consultant."""

import asyncio
import os
import sys
import tempfile
from pathlib import Path
from typing import List
from unittest.mock import AsyncMock, patch

from behave import given, when, then

# Задаём безопасные значения окружения до импорта модуля
os.environ.setdefault("TELEGRAM_API_ID", "1")
os.environ.setdefault("TELEGRAM_API_HASH", "hash")
os.environ.setdefault("AI_CONSULTANT_TEMPERATURE", "0.3")
os.environ.setdefault("AI_CONSULTANT_MODEL", "gpt-consultant")
os.environ.setdefault("KNOWLEDGE_BASE_PATH", str(Path.cwd()))

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from ai_project_consultant import AIProjectConsultant  # noqa: E402


def _get_loop():
    """Возвращает event loop, создавая новый при необходимости."""
    try:
        return asyncio.get_event_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        return loop


@given("подготовлена временная база знаний")
def step_prepare_tmp_kb(context):
    """Создаём временную директорию под знания."""
    tmp_dir = tempfile.mkdtemp(prefix="kb_behave_")
    context.kb_path = Path(tmp_dir)
    context.temp_dirs = getattr(context, "temp_dirs", [])
    context.temp_dirs.append(tmp_dir)


@given('файл базы знаний "{filename}" с содержимым')
@when('файл базы знаний "{filename}" с содержимым')
@then('файл базы знаний "{filename}" с содержимым')
def step_create_kb_file(context, filename):
    """Создаём markdown файл в базе знаний."""
    kb_file = context.kb_path / filename
    kb_file.parent.mkdir(parents=True, exist_ok=True)
    kb_file.write_text(context.text, encoding="utf-8")


@when("я создаю консультанта с этой базой")
def step_create_consultant(context):
    """Инициализируем AI консультанта."""
    context.consultant = AIProjectConsultant(knowledge_base_path=context.kb_path)


@when("я индексирую базу знаний")
def step_index_kb(context):
    """Запускаем индексацию базы знаний."""
    loop = _get_loop()
    context.index_result = loop.run_until_complete(context.consultant.index_knowledge_base())


@when('я выполняю консультацию по запросу "{query}" с RAG')
def step_consult_with_rag(context, query):
    """Выполняем консультацию с подменой chat_completion."""
    sent_messages: List[dict] = []

    async def fake_chat_completion(messages, **_kwargs):
        sent_messages.extend(messages)
        return "Рекомендация: храните бэкапы и проверяйте доступность."

    # Гарантируем наличие контекста RAG без зависимости от простого поиска
    context.consultant.search_knowledge_base = AsyncMock(return_value=["Контекст про бэкапы в MinIO"])

    loop = _get_loop()
    with patch("ai_project_consultant.chat_completion", side_effect=fake_chat_completion):
        context.response_text = loop.run_until_complete(context.consultant.consult(query, use_rag=True))
    context.sent_messages = sent_messages


@when("я очищаю историю диалога")
def step_clear_history(context):
    """Сбрасываем историю сообщений консультанта."""
    loop = _get_loop()
    loop.run_until_complete(context.consultant.clear_history())


@then("найдено не менее {count:d} документов")
def step_found_docs(context, count):
    """Проверяем количество документов после индексации."""
    assert len(context.index_result) >= count, f"Ожидалось >= {count}, получено {len(context.index_result)}"


@then("в сообщения передан контекст из базы знаний")
def step_messages_have_context(context):
    """Проверяем, что контекст из базы знаний попадает в сообщения."""
    assert any(
        msg["role"] == "system" and "Релевантная информация" in msg["content"] for msg in context.sent_messages
    ), "Контекст из базы знаний не найден в сообщениях"


@then('ответ содержит "{text}"')
def step_response_contains(context, text):
    """Проверяем текст ответа."""
    assert text.lower() in context.response_text.lower(), f"'{text}' не найден в ответе: {context.response_text}"


@then("история диалога пустая")
def step_history_empty(context):
    """Убеждаемся, что история очищена."""
    assert context.consultant.conversation_history == []
