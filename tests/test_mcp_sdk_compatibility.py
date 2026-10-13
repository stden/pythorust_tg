"""Verify SDK registration independently of missing legacy Telegram helpers."""

import importlib
import sys
from types import ModuleType


async def test_mcp_server_registers_tools_without_exposing_context(monkeypatch):
    export_helpers = ModuleType("chat_export_utils")
    export_helpers.collect_reaction_breakdown = lambda message: {}
    export_helpers.load_chats_from_config = lambda *args, **kwargs: {}
    export_helpers.resolve_sender_name = lambda *args, **kwargs: "Unknown sender"
    session_helpers = ModuleType("telegram_session")
    session_helpers.LOCK_FILE = "unused-test-session.lock"
    session_helpers.get_client = lambda: None
    session_helpers.known_senders = {}
    monkeypatch.setitem(sys.modules, "chat_export_utils", export_helpers)
    monkeypatch.setitem(sys.modules, "telegram_session", session_helpers)
    monkeypatch.delitem(sys.modules, "mcp_telegram_server", raising=False)

    module = importlib.import_module("mcp_telegram_server")
    try:
        tools = {tool.name: tool for tool in await module.server.list_tools()}
        assert set(tools) == {"list_configured_chats", "fetch_recent_messages", "send_message"}
        assert "ctx" not in tools["fetch_recent_messages"].input_schema["properties"]
        resources = await module.server.list_resources()
        assert [str(resource.uri) for resource in resources] == ["resource://telegram/chats"]
    finally:
        sys.modules.pop("mcp_telegram_server", None)
