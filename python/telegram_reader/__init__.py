"""Telegram Reader - Rust-powered Telegram chat toolkit with Python bindings.

This module provides high-performance Telegram operations implemented in Rust
with Python bindings via PyO3. It's designed for use with MCP servers and
other Python applications that need fast, reliable Telegram integration.

Usage:
    from telegram_reader import (
        # Chat management
        list_configured_chats,
        resolve_chat,
        check_session,
        get_lock_file_path,
        ChatTarget,
        Message,
        SendResult,
        # Text processing (Phase 1)
        sanitize_filename,
        contains_phone,
        detect_conversion,
        is_meaningful_message,
        parse_iso_datetime,
        parse_reactions,
        build_message_text,
        collect_reactions_summary,
    )

    # List all configured chats
    chats = list_configured_chats()

    # Resolve a chat by name
    target = resolve_chat("my_channel")

    # Check if session exists
    if check_session():
        print("Telegram session is ready")

    # Text processing examples
    filename = sanitize_filename("My Chat @#$", fallback="unknown")
    has_phone = contains_phone("+0 000 000-00-00")
    conversion = detect_conversion("Хочу купить!")
"""

from __future__ import annotations

# Re-export from the Rust extension module
try:
    from telegram_reader.telegram_reader import (
        # Classes
        ChatTarget,
        Message,
        SendResult,
        # Analytics classes (Phase 2.1)
        SessionMetrics,
        RetentionMetrics,
        # Chat functions
        check_session,
        get_lock_file_path,
        list_configured_chats,
        resolve_chat,
        # Text processing (Phase 1)
        build_message_text,
        collect_reactions_summary,
        contains_phone,
        detect_conversion,
        is_meaningful_message,
        parse_iso_datetime,
        parse_reactions,
        sanitize_filename,
    )

    __all__ = [
        # Classes
        "ChatTarget",
        "Message",
        "SendResult",
        # Analytics classes (Phase 2.1)
        "SessionMetrics",
        "RetentionMetrics",
        # Chat functions
        "list_configured_chats",
        "resolve_chat",
        "check_session",
        "get_lock_file_path",
        # Text processing (Phase 1)
        "sanitize_filename",
        "contains_phone",
        "detect_conversion",
        "is_meaningful_message",
        "parse_iso_datetime",
        "parse_reactions",
        "build_message_text",
        "collect_reactions_summary",
    ]
except ImportError:
    # Rust extension not built yet - provide stub for development
    import warnings

    warnings.warn(
        "telegram_reader Rust extension not found. Build with 'maturin develop' or 'pip install .'",
        ImportWarning,
        stacklevel=2,
    )

    __all__ = []
