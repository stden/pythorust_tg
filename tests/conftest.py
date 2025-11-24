# -*- coding: utf-8 -*-
"""Pytest configuration and fixtures."""
import os
import sys

# Set all required environment variables BEFORE any imports happen
# This is critical because many modules check env vars at import time

os.environ.setdefault("ANTHROPIC_API_KEY", "test-anthropic-key")
os.environ.setdefault("GOOGLE_API_KEY", "test-google-key")
os.environ.setdefault("OPENAI_API_KEY", "test-openai-key")
os.environ.setdefault("LINEAR_API_KEY", "test-linear-key")
os.environ.setdefault("TELEGRAM_PHONE", "+1234567890")
os.environ.setdefault("TELEGRAM_API_ID", "12345")
os.environ.setdefault("TELEGRAM_API_HASH", "testhash")
os.environ.setdefault("YANDEX_API_KEY", "test-yandex-key")
os.environ.setdefault("YANDEX_FOLDER_ID", "test-folder-id")
os.environ.setdefault("AWS_ACCESS_KEY_ID", "test-access-key")
os.environ.setdefault("AWS_SECRET_ACCESS_KEY", "test-secret-key")

# Ensure the project root is in the path
project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if project_root not in sys.path:
    sys.path.insert(0, project_root)


def pytest_configure(config):
    """Configure pytest markers."""
    config.addinivalue_line(
        "markers", "asyncio: mark test as an async test."
    )
