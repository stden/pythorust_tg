# CODEV Plan: Credit Expert Bot

> Protocol: SPIDER-SOLO
> Related spec: `codev/specs/0005-credit-expert-bot.md`
> Context: Implementation of a new bot for the "Bankruptcy/Debt Relief" niche based on the provided sales script.

## Goals
- Create a Telegram bot that qualifies leads for debt relief services.
- Implement a strict dialog scenario: Greeting -> Qualification -> Close for a call.
- Handle objections (Price, "I'll think about it", Competitors) according to the script.
- Store all users and conversation history in MySQL for analytics and CRM.
- Use OpenAI for generating natural responses within the given system prompt.

## Phases

**Phase 1: Core Implementation and Database (Completed)**
- Objective: Create a working bot with DB and AI integration.
- Tasks:
  - Implement `src/bin/bots/credit_expert_bot.rs` based on `teloxide`.
  - Set up MySQL schema via `migrations/002_create_bot_tables.sql` (tables `bot_users`, `bot_messages`, `bot_sessions`, `bot_experiments`).
  - Integrate `OpenAIClient` with system prompt "Credit Expert Credit Expert".
  - Implement message saving logic and session management.
- Deliverable: Working Rust bot binary.

**Phase 2: Testing and QA (Completed)**
- Objective: Ensure bot logic matches the sales script through BDD tests.
- Tasks:
  - Create feature file `features/credit_expert_bot.feature` with scenarios (Greeting, Qualification, Objections).
  - Implement test steps in `features/steps/credit_expert_bot_steps.py` with DB and AI mocks.
  - Run tests and verify logic correctness.
- Deliverable: Green `behave` tests.

**Phase 3: Deployment and Monitoring (Planned)**
- Objective: Launch the bot in production environment.
- Tasks:
  - Add environment variables (`CREDIT_EXPERT_BOT_TOKEN`) to `.env`.
  - Set up systemd unit or Docker Compose service.
  - Connect monitoring via the Rust N8N monitor or service-level health checks.
- Deliverable: Running bot service.

## Artifacts
- Code: `src/bin/bots/credit_expert_bot.rs`
- Tests: `features/credit_expert_bot.feature`, `features/steps/credit_expert_bot_steps.py`
- Plan: `codev/plans/0005-credit-expert-bot.md`
