# language: en

Feature: N8N backups and restore
  As an operations engineer
  I want to manage N8N backups
  So that I can ensure configuration safety

  Background:
    Given the backup directory is a temporary folder

  Scenario: Creating a backup saves data
    Given the retention policy is 30 days and max 5 backups
    And the API returns 2 workflows and 1 credential
    When an N8N backup is created
    Then a backup archive is created
    And the archive contains info for 2 workflows and 1 credential

  Scenario: Cleanup removes old and extra backups
    Given the retention policy is 3 days and max 2 backups
    And backups exist with dates:
      | name                  | days_ago |
      | n8n_backup_old.tar.gz | 10      |
      | n8n_backup_mid.tar.gz | 2       |
      | n8n_backup_new.tar.gz | 1       |
    When backup cleanup runs
    Then 2 recent archives remain
    And the archive "n8n_backup_old.tar.gz" is removed

  Scenario: Restoring a missing archive returns an error
    When restore is executed from "missing_backup.tar.gz"
    Then restore is unsuccessful
