# language: en

Feature: Telegram operations
  As a user
  I want to read and export chats
  So that I can analyze conversations

  Background:
    Given the configuration is loaded from "config.yml"

  Scenario: Get message limit
    When I request the message limit
    Then the limit equals 3000 messages

  Scenario: Limit in GitHub Actions
    Given the environment variable "GITHUB_ACTIONS" is set to "true"
    When I request the message limit
    Then the limit equals 1000 messages

  Scenario: Get chat settings
    Given a chat "example_channel" exists
    When I request settings for chat "example_channel"
    Then the chat type is "channel"

  Scenario: Missing chat
    When I request settings for chat "missing_chat"
    Then an empty result is returned
