# -*- coding: utf-8 -*-
"""Behave steps for AI Project Consultant."""

import asyncio
import os
import sys
import tempfile
from pathlib import Path
from typing import List
from unittest.mock import AsyncMock, patch

from behave import given, when, then

# Set safe environment defaults before importing the module
os.environ.setdefault("TELEGRAM_API_ID", "1")
os.environ.setdefault("TELEGRAM_API_HASH", "hash")
os.environ.setdefault("AI_CONSULTANT_TEMPERATURE", "0.3")
os.environ.setdefault("AI_CONSULTANT_MODEL", "gpt-consultant")
os.environ.setdefault("KNOWLEDGE_BASE_PATH", str(Path.cwd()))

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from ai_project_consultant import AIProjectConsultant  # noqa: E402


def _get_loop():
    """Return an event loop, creating a new one if needed."""
    try:
        return asyncio.get_event_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        return loop


@given("a temporary knowledge base is prepared")
def step_prepare_tmp_kb(context):
    """Create a temporary directory for knowledge base files."""
    tmp_dir = tempfile.mkdtemp(prefix="kb_behave_")
    context.kb_path = Path(tmp_dir)
    context.temp_dirs = getattr(context, "temp_dirs", [])
    context.temp_dirs.append(tmp_dir)


@given('a knowledge base file "{filename}" with content')
@when('a knowledge base file "{filename}" with content')
@then('a knowledge base file "{filename}" with content')
def step_create_kb_file(context, filename):
    """Create a Markdown file in the knowledge base."""
    kb_file = context.kb_path / filename
    kb_file.parent.mkdir(parents=True, exist_ok=True)
    kb_file.write_text(context.text, encoding="utf-8")


@when("I create a consultant with this knowledge base")
def step_create_consultant(context):
    """Initialize the consultant."""
    context.consultant = AIProjectConsultant(knowledge_base_path=context.kb_path)


@when("I index the knowledge base")
def step_index_kb(context):
    """Run knowledge base indexing."""
    loop = _get_loop()
    context.index_result = loop.run_until_complete(context.consultant.index_knowledge_base())


@when('I run a consultation for query "{query}" with RAG')
def step_consult_with_rag(context, query):
    """Run a consultation with a stubbed chat_completion."""
    sent_messages: List[dict] = []

    async def fake_chat_completion(messages, **_kwargs):
        sent_messages.extend(messages)
        return "Recommendation: store backups and periodically verify availability."

    # Guarantee RAG context without relying on external search
    context.rag_docs = ["MinIO context about backups"]
    context.consultant.search_knowledge_base = AsyncMock(return_value=context.rag_docs)

    loop = _get_loop()
    with patch("ai_project_consultant.chat_completion", side_effect=fake_chat_completion):
        context.response_text = loop.run_until_complete(context.consultant.consult(query, use_rag=True))
    context.sent_messages = sent_messages


@when("I clear the conversation history")
def step_clear_history(context):
    """Clear consultant message history."""
    loop = _get_loop()
    loop.run_until_complete(context.consultant.clear_history())


@then("at least {count:d} documents are found")
def step_found_docs(context, count):
    """Assert the number of documents after indexing."""
    assert len(context.index_result) >= count, f"Expected >= {count}, got {len(context.index_result)}"


@then("the knowledge base context is included in the messages")
def step_messages_have_context(context):
    """Assert that knowledge base content is included in the prompt."""
    rag_docs = getattr(context, "rag_docs", [])
    assert rag_docs, "No RAG docs were configured for the test"
    assert any(
        msg["role"] == "system" and any(doc in msg["content"] for doc in rag_docs) for msg in context.sent_messages
    ), "Knowledge base context was not found in system messages"


@then('the response contains "{text}"')
def step_response_contains(context, text):
    """Assert response text contains a substring."""
    assert text.lower() in context.response_text.lower(), f"'{text}' not found in response: {context.response_text}"


@then("the conversation history is empty")
def step_history_empty(context):
    """Assert that history was cleared."""
    assert context.consultant.conversation_history == []
