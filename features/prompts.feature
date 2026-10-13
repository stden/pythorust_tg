# language: en

Feature: Prompt system
  As a developer
  I want to load prompts from files
  So that I can use them in AI integrations

  Scenario: Load sales agent prompt
    Given a prompt file "sales_agent.md" exists
    When I load the prompt "sales_agent"
    Then the prompt contains the text "SPIN"

  Scenario: Load digest prompt
    Given a prompt file "digest.md" exists
    When I load the prompt "digest"
    Then the prompt contains the text "Digest Creator"

  Scenario: Load moderator prompt
    Given a prompt file "moderator.md" exists
    When I load the prompt "moderator"
    Then the prompt contains the text "Chat Moderator"

  Scenario: List all prompts
    When I request the prompt list
    Then the list contains at least 5 prompts

  Scenario: Load a missing prompt
    When I load the prompt "missing_prompt"
    Then a prompt load error occurs
