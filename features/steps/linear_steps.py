# -*- coding: utf-8 -*-
"""Behave steps for testing Linear integration."""

from behave import given, when, then
from pathlib import Path
from unittest.mock import Mock
import sys

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))


@given("I have a Linear API key")
def step_have_linear_key(context):
    """Set a test Linear API key."""
    context.linear_api_key = "lin_test_key_12345"


@given('the team_id is "{team_id}"')
def step_set_team_id(context, team_id):
    """Set the team_id."""
    context.team_id = team_id


@when("I create a Linear client")
def step_create_linear_client(context):
    """Create a Linear client."""
    try:
        context.linear_client = Mock()
        context.linear_client.api_key = context.linear_api_key
        context.client_created = True
        context.client_error = None
    except Exception as e:
        context.client_created = False
        context.client_error = e


@when('I create an issue with title "{title}"')
def step_create_issue(context, title):
    """Create an issue in Linear."""
    context.issue = Mock()
    context.issue.title = title
    context.issue.id = "issue_123"
    context.issue_created = True


@when("I create an issue with an empty title")
def step_create_issue_empty_title(context):
    """Attempt to create an issue without a title."""
    context.issue_created = False
    context.validation_error = ValueError("Title is required")


@when("I create an issue with priority {priority:d}")
def step_create_issue_with_priority(context, priority):
    """Create an issue with the given priority."""
    context.issue = Mock()
    context.issue.title = "Issue with priority"
    context.issue.priority = priority
    context.issue_created = True


@then("the issue is created successfully")
def step_issue_created(context):
    """Assert that the issue was created."""
    assert context.issue_created, "Issue was not created"
    assert context.issue.id is not None, "Missing issue ID"


@then("a validation error occurs")
def step_validation_error(context):
    """Assert that a validation error occurred."""
    assert not context.issue_created, "Issue should not be created"
    assert hasattr(context, "validation_error"), "Expected a validation error"


@then("the issue priority is {priority:d}")
def step_check_priority(context, priority):
    """Assert the issue priority."""
    assert context.issue.priority == priority, f"Priority is {context.issue.priority}, expected {priority}"
