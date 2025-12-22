# language: en

Feature: OpenAI integration
  As a developer
  I want to use the OpenAI API
  So that I can generate AI responses

  Scenario: Create client with an API key
    Given I have an OpenAI API key
    When I create an OpenAI client
    Then the client is created successfully

  Scenario: Create client without an API key
    Given the OpenAI API key is missing
    When I create an OpenAI client without an API key
    Then an error occurs "API key required"

  Scenario: Default model
    Given I have an OpenAI API key
    When I create an OpenAI client
    Then the default model is "gpt-5.2-2025-12-11"

  Scenario: Generate a response with a custom system prompt
    Given I have an OpenAI API key
    And the system prompt is "You are a helpful assistant"
    When I send the message "Hello"
    Then the response contains text
