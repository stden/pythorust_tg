# language: en

Feature: Task Assistant Bot
  As an administrator
  I want to manage services via a bot
  So that I can quickly check N8N and backups

  Background:
    Given Task Assistant Bot is initialized

  Scenario: /start shows all action buttons
    When user 111 sends /start
    Then the bot shows 6 buttons
    And the buttons include action "check_n8n"
    And the buttons include action "restart_n8n"
    And the buttons include action "create_backup"
    And the buttons include action "list_backups"
    And the buttons include action "ai_consultant"
    And the buttons include action "server_status"

  Scenario: Access is restricted by ALLOWED_USERS
    Given allowed users are "123"
    When user 999 sends /start
    Then the bot replies "❌ Доступ запрещён"

  Scenario: N8N health check returns status
    When the bot checks N8N health
    Then the health status is "✅ Работает"
    And the HTTP status code is 200

  Scenario: Restarting N8N returns result and health
    When I restart the N8N service
    Then the restart result is successful
    And the result includes health with status code 200

  Scenario: Creating a backup via the bot returns output
    When the bot creates an N8N backup
    Then the backup output contains "backup complete"

  Scenario: Server status returns metrics
    When the bot requests server status
    Then CPU, memory, and disk values are returned
