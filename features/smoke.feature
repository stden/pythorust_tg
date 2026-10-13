@smoke
Feature: Smoke test for Telegram bot framework
  Verify that core modules are operational

  Scenario: Python environment is functional
    Given the Python interpreter is available
    Then standard library modules should be importable

  Scenario: Project core modules are loadable
    Given the project root is in the Python path
    Then the core module should be importable without errors
