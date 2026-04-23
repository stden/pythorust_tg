# Python → Rust Migration Plan

**Date:** 2025-11-25  
**Goal:** consolidate the stack on Rust for performance, type safety, and a single codebase. Python should remain only for legacy bots/ops until fully replaced.

---

## Inventory by area

### Core Telegram CLI (mostly done in Rust)
- Rust: `src/main.rs` + commands (`read`, `tg`, `list_chats`, `dialogs`, `export`, `delete_zoom`, `react`, `like`, `linear`, `digest`, `analyze`, `crm`, `hunt`, `n8n-monitor/backup`) and supporting binaries (`export_any_chat.rs`, `download_chat.rs`, `download_user_chat.rs`, `delete_unanswered.rs`, `find_user.rs`, `like_messages.rs`, `send_message.rs`, `send_viral_question.rs`).
- Python legacy: `read_example_chat.py`, `export_user_simple.py`, `read_vibecod3rs.py`, assorted helpers.
- Actions: validate parity on exports/reactions; deprecate Python helpers after verification; ensure session lock shared.

### AI & automation
- Rust: `autoanswer` (placeholder polling), `analyze`, `digest`, `crm`, `hunt`, `message_digest`, `chat_analyzer` bin.
- Python legacy: `chat_analysis/`, `ai_project_consultant.py`, `ai_service.py`, `collect_chat_ideas.py`, `check_all_chats_tasks.py`, `bulk_reactions.py`.
- Actions: port chat analyzer pipeline to Rust; improve auto-responder streaming; retire Python analyzers after parity tests.

### Linear
- Rust: `src/commands/linear.rs`, `src/bin/linear_bot.rs`.
- Python legacy: `linear_client.py`, `create_linear_tasks.py`, `sync_linear_tasks.py`.
- Actions: confirm feature parity (labels/projects), add tests, archive Python clients once Rust path is stable.

### Bots
- Rust: `src/bin/bfl_sales_bot.rs` (MySQL logging + A/B prompts).
- Python legacy: `credit_expert_bot.py`, `task_assistant_bot.py`, `ai_project_consultant.py`, `devops_ai_bot.py`, base helpers in `telegram_bot_base.py`.
- Actions: migrate Credit Expert bot to Rust (teloxide), share logging/AB infra with BFL bot, deprecate legacy base once Rust bots cover flows.

### Ops / Utilities
- Rust: `src/commands/n8n.rs`, `src/bin/site_monitor.rs`, `src/bin/http_bench.rs`, `src/bin/k8s_dash.rs`, `src/bin/devops_bot_probe.rs`.
- Python legacy: `n8n_backup.py`, `n8n_monitor.py`, `n8n_backup_cron.sh`, `telegram_service.py`, `telegram_session.py` (Python session helper), `mcp_telegram_server.py`.
- Actions: validate Rust N8N monitor/backup parity; retire Python versions after staged roll-out; align session handling.

---

## Migration phases

### Phase 1: Stabilize Rust CLI (core)
- Finish/clean auto-responder (`autoanswer.rs`) and chat analyzer wiring.
- Harden session lock + env validation; document `init-session`.
- Add regression tests for export/reactions/linear/digest.
- Milestone: run `cargo test` + CLI smoke tests replace Python flows for read/tg/export/delete-zoom/react/like/linear.

### Phase 2: AI services and bots
- Port Python chat analyzer features (topics/sentiment/activity) into Rust `analyze`/`chat_analyzer`.
- Implement streaming auto-responder loop with proper update handling.
- Migrate Credit Expert bot to Rust with shared MySQL schema; reuse A/B infra from BFL bot.
- Deprecate `ai_project_consultant.py` or reimplement core in Rust.

### Phase 3: Ops and integrations
- Validate Rust N8N monitor/backup against Python scripts; cut over cron/task assistant to Rust binaries.
- Replace `mcp_telegram_server.py` with Rust MCP server or extend existing CLI for IDE agents.
- Confirm Linear Rust path covers all Python features; remove legacy scripts.

### Phase 4: Cleanup
- Remove/archive Python scripts once parity confirmed and tests exist.
- Update README/AGENTS to Rust-only guidance (done).
- Add CI to block new Python additions (except migration shims/tests).

---

## Success criteria
- All production flows (export/read/reactions/digest/linear/N8N) run on Rust binaries.
- Python scripts marked deprecated or removed after parity validation.
- Tests cover migrated features (unit/integration where applicable).
- Single source of truth for prompts/config (Rust side).

## Risks
- Session format mismatch (Telethon vs grammers) → keep separate session files and locks.
- API drift between Python and Rust implementations → maintain regression suite and fixtures.
- Operational scripts (backup/monitor) may rely on OS-specific behaviors → test on target hosts before removing Python.

## Tracking
- Code changes in `src/` and `src/bin/`
- Legacy scripts tracked for removal in `legacy-python/` checklist or issues
- Updates documented in README/AGENTS
