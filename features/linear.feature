# language: en

Feature: Linear integration
  As a developer
  I want to create issues in Linear
  So that I can track work

  Scenario: Create a Linear client
    Given I have a Linear API key
    When I create a Linear client
    Then the client is created successfully

  Scenario: Create an issue
    Given I have a Linear API key
    And the team_id is "abc123"
    When I create an issue with title "Test issue"
    Then the issue is created successfully

  Scenario: Create an issue without a title
    Given I have a Linear API key
    When I create an issue with an empty title
    Then a validation error occurs

  Scenario: Create an issue with priority
    Given I have a Linear API key
    And the team_id is "abc123"
    When I create an issue with priority 1
    Then the issue priority is 1
