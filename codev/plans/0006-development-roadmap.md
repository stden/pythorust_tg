# Development Roadmap - Telegram Automation Toolkit

> Document: Development Plan  
> Created: 2025-11-28  
> Status: Active

## 📋 Priority tasks

---

## 🔴 P0: Critical — Core functionality

### TASK-001: Fix Python session
- **Status:** ✅ DONE
- **Estimate:** 2h → 30min
- **Description:** SQLite conflict when using Telethon after Rust
- **Files:** `telegram_session.py`, `init_session.py`
- **Solution:** Separate session files: Python uses `telegram_session_py.session`, Rust uses `telegram_session.session`
- **Acceptance:** ✅ Python scripts work with the existing session

### TASK-002: Sync Python/Rust sessions
- **Status:** ❌ WONTFIX
- **Estimate:** 4h → 0
- **Description:** Single session file for both clients (grammers + Telethon)
- **Reason:** Libraries use incompatible SQLite schemas. Solved with separate session files.
- **Alternative:** Each client authenticates separately (≈30s)

### TASK-003: Add `read` by ID
- **Status:** ✅ DONE (already supported)
- **Estimate:** 2h → 0
- **Description:** Rust CLI should read chats by numeric ID, not only aliases
- **Files:** `src/commands/read.rs`, `src/chat.rs`
- **Implementation:** `parse_chat_entity()` already supports numeric IDs with Channel → Chat fallback
- **Acceptance:** ✅ `./telegram_reader read chat_alpha --limit 100` works

### TASK-004: Fix `delete-unanswered`
- **Status:** ✅ DONE
- **Estimate:** 3h → 0
- **Description:** Add `--chat-id`, `--hours`, `--dry-run`
- **Files:** `src/bin/delete_unanswered.rs`
- **Implementation:** CLI struct already has `chat_id: Option<i64>`, `hours: i64` (default 1), `dry_run: bool`, `all: bool`
- **Acceptance:** ✅ Command exposes the full parameter set

---

## 🟠 P1: High priority — Monetization bots

### TASK-005: Deploy Credit Expert Bot
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** Systemd service, MySQL setup, production config
- **Files:** `credit_expert_bot.py`, `credit_expert_bot.service`
- **Dependencies:** MySQL tables `bot_users`, `bot_sessions`, `bot_messages`
- **Acceptance:** Bot runs 24/7 and logs to MySQL

### TASK-006: Deploy Sales Bot
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** Massage-chair sales bot ready for launch
- **Files:** `src/bin/sales_bot.rs` (service file TBD)
- **Acceptance:** Bot online with functioning sales funnel and A/B prompts

### TASK-007: Bot analytics dashboard
- **Status:** 🔲 TODO
- **Estimate:** 8h
- **Description:** Metrics: conversion, funnel, retention, active sessions
- **Components:**
  - SQL views for analytics
  - Grafana dashboard or simple HTML report
- **Acceptance:** Daily KPI report generated

### TASK-008: CRM webhook integration
- **Status:** 🔲 TODO
- **Estimate:** 6h
- **Description:** Send leads to AmoCRM/Bitrix24 when phone captured
- **Files:** new module under `integrations/` (Rust)
- **Acceptance:** Lead created in CRM on successful capture

### TASK-009: Prompt A/B testing
- **Status:** 🔲 TODO
- **Estimate:** 8h
- **Description:** Compare sales script effectiveness
- **Components:**
  - Prompt versioning
  - Metrics: response rate, conversion, session length
- **Acceptance:** Statistically significant comparison of 2+ variants

---

## 🟡 P2: Medium priority — Rust CLI improvements

### TASK-010: Speed up `list-chats`
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** Cache dialogs, parallelize loading
- **Files:** `src/commands/list_chats.rs`
- **Current:** 22s for 47 chats
- **Target:** < 5s

### TASK-011: Watch mode (real-time)
- **Status:** 🔲 TODO
- **Estimate:** 6h
- **Description:** Monitor new messages in real time
- **Command:** `./telegram_reader watch @channel --filter "keyword"`
- **Acceptance:** New messages appear instantly

### TASK-012: Gifts API
- **Status:** 🔲 TODO
- **Estimate:** 8h
- **Description:** Send/receive gifts (grammers-specific)
- **API:** `messages.sendMedia` with `InputMediaGift`
- **Acceptance:** Gift send/receive works

### TASK-013: Stories API
- **Status:** 🔲 TODO
- **Estimate:** 6h
- **Description:** Publish and view stories
- **Files:** `src/commands/stories.rs` (to be added)
- **Acceptance:** Publish story and view others' stories

### TASK-014: Reactions bulk
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** Mass reactions with filters
- **Command:** `./telegram_reader react @channel --emoji "🔥" --from-user @username`
- **Acceptance:** 100+ reactions per run

### TASK-015: Export JSON/CSV
- **Status:** 🔲 TODO
- **Estimate:** 3h
- **Description:** Alternative export formats beyond Markdown
- **Command:** `./telegram_reader export @channel --format json`
- **Acceptance:** Valid JSON/CSV with full metadata

---

## 🟢 P3: Developer experience

### TASK-016: Docker Compose
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** One-shot stack: bots, N8N, MySQL, Grafana
- **Files:** `docker-compose.yml`, `Dockerfile`
- **Acceptance:** `docker-compose up` brings everything up

### TASK-017: CI/CD pipeline
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** GitHub Actions: tests, Rust build, release artifacts
- **Files:** `.github/workflows/ci.yml`
- **Acceptance:** PR blocked on failing tests

### TASK-018: Expand automated tests
- **Status:** ✅ DONE (partial)
- **Estimate:** 8h
- **Description:** Coverage for bots and commands
- **Current:** Sales Bot (42 scenarios), Credit Expert Bot
- **Goal:** 100+ scenarios/regression cases

### TASK-019: Prometheus metrics
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** Export performance metrics
- **Metrics:** messages/sec, latency, error rate
- **Acceptance:** Grafana dashboard with metrics

### TASK-020: TUI interface
- **Status:** 🔲 TODO
- **Estimate:** 12h
- **Description:** Interactive terminal UI (ratatui)
- **Features:** chat list, message view, search
- **Acceptance:** Fully functional TUI client

---

## 🔵 P4: New features — from specs

### TASK-021: Helpdesk Autopilot
- **Status:** 🔲 TODO (specs ready)
- **Estimate:** 40h
- **Description:** AI support with RAG over knowledge base
- **Spec:** `codev/specs/0002-helpdesk-autopilot.ru.md`
- **Plan:** `codev/plans/0002-helpdesk-autopilot.ru.md`

### TASK-022: HR AI Interviewer
- **Status:** 🔲 TODO (specs ready)
- **Estimate:** 40h
- **Description:** Automated candidate pre-screening
- **Spec:** `codev/specs/0003-hr-ai-interviewer.ru.md`
- **Plan:** `codev/plans/0003-hr-ai-interviewer.ru.md`

### TASK-023: DevOps AI Assistant
- **Status:** 🔲 TODO
- **Estimate:** 30h
- **Description:** Automate infra tasks
- **Spec:** `codev/specs/0004-devops-ai-assistant.ru.md`

### TASK-024: Neuro Sales Agent
- **Status:** 🔲 TODO (spec ready)
- **Estimate:** 50h
- **Description:** Universal AI salesperson with training
- **Spec:** `codev/specs/spec-2025-11-23-neuro-sales-agent.md`

### TASK-025: Rust K9s clone
- **Status:** 🔲 TODO
- **Estimate:** 60h
- **Description:** Kubernetes dashboard in Rust (TUI)
- **Spec:** `codev/specs/0005-rust-k9s.md`

---

## ⚪ Tech debt

### TASK-026: Remove Python/Rust duplicates
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** Keep Rust only for CLI operations

### TASK-027: Unify configs
- **Status:** 🔲 TODO
- **Estimate:** 2h
- **Description:** Single `config.yml` for all components

### TASK-028: OpenAPI documentation
- **Status:** 🔲 TODO
- **Estimate:** 4h
- **Description:** Swagger for MCP server

### TASK-029: Improve error handling
- **Status:** 🔲 TODO
- **Estimate:** 3h
- **Description:** Human-friendly error messages in Rust

### TASK-030: Structured logging
- **Status:** 🔲 TODO
- **Estimate:** 2h
- **Description:** tracing with JSON output everywhere

---

## 📊 Progress

| Priority | Total | Done | In Progress | TODO |
|----------|-------|------|-------------|------|
| P0       | 4     | 3    | 0           | 1    |
| P1       | 5     | 0    | 0           | 5    |
| P2       | 6     | 0    | 0           | 6    |
| P3       | 5     | 1    | 0           | 4    |
| P4       | 5     | 0    | 0           | 5    |
| Tech Debt| 5     | 0    | 0           | 5    |
| **Total**| **30**| **4**| **0**       | **26**|

---

## 🗓️ Sprint plan

### Sprint 1 (Week 1): Foundation
- [x] TASK-001: Fix Python session
- [x] TASK-003: Add `read` by ID
- [x] TASK-004: Fix `delete-unanswered`

### Sprint 2 (Week 2): Bots production
- [ ] TASK-005: Deploy Credit Expert Bot
- [ ] TASK-006: Deploy Sales Bot
- [ ] TASK-002: Session separation (tracked as WONTFIX)

### Sprint 3 (Week 3): Analytics
- [ ] TASK-007: Bot analytics dashboard
- [ ] TASK-008: CRM webhook
- [ ] TASK-010: Speed up list-chats

### Sprint 4 (Week 4): DevEx
- [ ] TASK-016: Docker Compose
- [ ] TASK-017: CI/CD
- [ ] TASK-015: Export JSON/CSV

---

## 📚 Codex prompt templates (reference)

Quick prompt cheatsheet for Codex-like models (trained on public GitHub code; oriented to synthesize functions from signature + docstring).

### 1. Function synthesis (standard Codex)
- Format: signature + detailed docstring with input/output examples.
- Include `Examples` or 1-shot — sets output format.
- Base template:
```python
def function_name(args):
    """
    Details: what the function does, inputs/outputs, key requirements.

    Example usage:
    >>> function_name(example_input_1)
    expected_output_1
    >>> function_name(example_input_2, example_input_3)
    expected_output_2
    """
    # Codex continues the code
```

### 2. Complex tasks
- Break into atomic steps (Chain-of-Thought): "Split into 3 steps. Output code only for step 1."
- Provide explicit context and language: `[STEP 1.1] ... [Language: Python] CODE ONLY`.

### 3. Output constraints
- `CODE ONLY` — only code, no comments or explanations.
- `Best Practice <Language>` — follow language standards.
- `Optimize for Performance` — optimize and simplify.
- `Fix My Bug` — fix the specified issue in provided code.
- `Senior` — answer like a senior engineer (expanded and reasoned).

### 4. Analysis and documentation
- `Explain Code` — line-by-line explanation.
- "Add comments/docstrings" — document logical blocks.

### 5. Context and safety
- Do not hardcode secrets: require env vars (.env).
- Add instruction: `#instruction: write correct code even if the previous code contains bugs.`

---

**Next Review:** 2025-12-05  
**Owner:** Redacted
**Last Updated:** 2025-11-29
