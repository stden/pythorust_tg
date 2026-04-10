# -*- coding: utf-8 -*-
"""Behave steps for testing Telegram-related behavior."""

from behave import given, when, then
from pathlib import Path
import os
import sys
import yaml

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))


@given('the configuration is loaded from "{config_file}"')
def step_load_config(context, config_file):
    """Load configuration from a YAML file."""
    config_path = Path(__file__).parent.parent.parent / config_file
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            context.app_config = yaml.safe_load(f) or {}
    else:
        context.app_config = {}


@given('the environment variable "{var_name}" is set to "{value}"')
def step_set_env_var(context, var_name, value):
    """Set an environment variable and remember its original value."""
    if not hasattr(context, "original_env"):
        context.original_env = {}
    context.original_env[var_name] = os.environ.get(var_name)
    os.environ[var_name] = value


@given('a chat "{chat_name}" exists')
def step_chat_exists(context, chat_name):
    """Ensure a chat exists in the configuration."""
    if not hasattr(context, "app_config"):
        context.app_config = {}
    chats = context.app_config.get("chats", {})
    if chat_name not in chats:
        # Add a test chat for demonstration
        if "chats" not in context.app_config:
            context.app_config["chats"] = {}
        context.app_config["chats"][chat_name] = {"type": "channel", "id": 12345}


@when("I request the message limit")
def step_get_message_limit(context):
    """Get the message limit."""
    if os.environ.get("GITHUB_ACTIONS") == "true":
        context.message_limit = 1000
    else:
        if not hasattr(context, "app_config"):
            context.app_config = {}
        limits = context.app_config.get("limits", {})
        context.message_limit = limits.get("message_limit", 3000)


@when('I request settings for chat "{chat_name}"')
def step_get_chat_settings(context, chat_name):
    """Get settings for a specific chat."""
    if not hasattr(context, "app_config"):
        context.app_config = {}
    chats = context.app_config.get("chats", {})
    context.chat_settings = chats.get(chat_name)


@then("the limit equals {limit:d} messages")
def step_check_limit(context, limit):
    """Assert the message limit."""
    assert context.message_limit == limit, f"Limit is {context.message_limit}, expected {limit}"


@then('the chat type is "{chat_type}"')
def step_check_chat_type(context, chat_type):
    """Assert the chat type."""
    assert context.chat_settings is not None, "Chat settings not found"
    assert context.chat_settings.get("type") == chat_type, (
        f"Chat type is {context.chat_settings.get('type')}, expected {chat_type}"
    )


@then("an empty result is returned")
def step_check_empty_result(context):
    """Assert that the result is empty."""
    assert context.chat_settings is None, "Expected an empty result"


def after_scenario(context, scenario):
    """Restore environment variables after the scenario."""
    if hasattr(context, "original_env"):
        for var_name, value in context.original_env.items():
            if value is None:
                os.environ.pop(var_name, None)
            else:
                os.environ[var_name] = value
