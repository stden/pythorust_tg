# -*- coding: utf-8 -*-
"""Behave steps for testing the prompt system."""

from behave import given, when, then
from pathlib import Path
import sys

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from integrations.prompts import load_prompt, list_prompts, Prompt


@given('a prompt file "{filename}" exists')
def step_prompt_file_exists(context, filename):
    """Assert the prompt file exists."""
    prompts_dir = Path(__file__).parent.parent.parent / "prompts"
    filepath = prompts_dir / filename
    assert filepath.exists(), f"File {filename} not found in {prompts_dir}"
    context.prompt_file = filepath


@when('I load the prompt "{prompt_name}"')
def step_load_prompt(context, prompt_name):
    """Load a prompt by name."""
    try:
        # Convert to enum name if applicable
        prompt_enum = getattr(Prompt, prompt_name.upper(), None)
        if prompt_enum:
            context.prompt_content = load_prompt(prompt_enum)
        else:
            context.prompt_content = load_prompt(prompt_name)
        context.load_error = None
    except Exception as e:
        context.prompt_content = None
        context.load_error = e


@when("I request the prompt list")
def step_list_prompts(context):
    """Fetch the list of available prompts."""
    context.prompts_list = list_prompts()


@then('the prompt contains the text "{text}"')
def step_prompt_contains(context, text):
    """Assert the prompt contains the given text."""
    assert context.prompt_content is not None, "Prompt not loaded"
    assert text.lower() in context.prompt_content.lower(), f"Text '{text}' not found in prompt"


@then("the list contains at least {count:d} prompts")
def step_prompts_count(context, count):
    """Assert the number of prompts."""
    assert len(context.prompts_list) >= count, f"Found {len(context.prompts_list)} prompts, expected >= {count}"


@then("a prompt load error occurs")
def step_load_error(context):
    """Assert that a load error occurred."""
    assert context.load_error is not None, "Expected a prompt load error"
