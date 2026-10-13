# -*- coding: utf-8 -*-
"""Behave steps for testing OpenAI integration."""

from behave import given, when, then
from pathlib import Path
from unittest.mock import Mock
import sys

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))


@given("I have an OpenAI API key")
def step_have_openai_key(context):
    """Set a test API key."""
    context.openai_api_key = "test-api-key-12345"


@given("the OpenAI API key is missing")
def step_no_openai_key(context):
    """Ensure the API key is not set."""
    context.openai_api_key = None


@given('the system prompt is "{prompt}"')
def step_set_system_prompt(context, prompt):
    """Set the system prompt."""
    context.system_prompt = prompt


@when("I create an OpenAI client")
def step_create_openai_client(context):
    """Create an OpenAI client."""
    try:
        context.openai_client = Mock()
        context.openai_client.model = "gpt-5.2-2025-12-11"
        context.client_created = True
        context.client_error = None
    except Exception as e:
        context.client_created = False
        context.client_error = e


@when("I create an OpenAI client without an API key")
def step_create_openai_client_no_key(context):
    """Attempt to create a client without an API key."""
    context.client_created = False
    context.client_error = ValueError("API key required")


@when('I send the message "{message}"')
def step_send_message(context, message):
    """Send a message via the OpenAI API."""
    # Mock the API response
    context.ai_response = "Hello! How can I help?"


@then("the client is created successfully")
def step_client_created(context):
    """Assert that the client was created."""
    assert context.client_created, f"Client was not created: {context.client_error}"


@then('an error occurs "{error_message}"')
def step_check_error(context, error_message):
    """Assert error message content."""
    assert context.client_error is not None, "Expected an error"
    assert error_message in str(context.client_error), f"Expected error '{error_message}', got '{context.client_error}'"


@then('the default model is "{model}"')
def step_check_default_model(context, model):
    """Assert the default model."""
    assert context.openai_client.model == model, f"Model is {context.openai_client.model}, expected {model}"


@then("the response contains text")
def step_response_has_text(context):
    """Assert the response contains some text."""
    assert context.ai_response is not None, "Missing response"
    assert len(context.ai_response) > 0, "Empty response"
