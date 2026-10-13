# language: en

Feature: AI Project Consultant
  As a developer
  I want to get consultations using a knowledge base
  So that I can speed up project work

  Background:
    Given a temporary knowledge base is prepared

  Scenario: Knowledge base indexing returns documents
    And a knowledge base file "arch.md" with content:
      """
      FastAPI service architecture
      """
    When I create a consultant with this knowledge base
    And I index the knowledge base
    Then at least 1 documents are found

  Scenario: Consultation uses RAG and history
    And a knowledge base file "storage.md" with content:
      """
      We use MinIO for backups.
      """
    When I create a consultant with this knowledge base
    And I run a consultation for query "How should we store backups?" with RAG
    Then the knowledge base context is included in the messages
    And the response contains "backups"

  Scenario: Clearing conversation history
    And a knowledge base file "history.md" with content:
      """
      Just a placeholder.
      """
    When I create a consultant with this knowledge base
    And I run a consultation for query "Reset history?" with RAG
    And I clear the conversation history
    Then the conversation history is empty
