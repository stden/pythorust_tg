# Python → Rust Migration Plan

**Date:** 2025-11-25  
**Goal:** consolidate the stack on Rust for performance, type safety, and a single codebase. Python should remain only for legacy analysis/MCP/helpers until fully replaced.

---

## Inventory by area

### Core Telegram CLI (mostly done in Rust)
- Rust: `src/main.rs` + commands (`read`, `tg`, `list_chats`, `dialogs`, `export`, `delete_zoom`, `react`, `like`, `linear`, `digest`, `analyze`, `crm`, `hunt`, `n8n-monitor/backup`) and supporting binaries under `src/bin/chat/`, `src/bin/export/`, `src/bin/moderation/`, and `src/bin/data/`.
- Python legacy: no root chat-export scripts remain.
- Actions: keep parity tests for exports/reactions and keep the Rust session lock documented.

### AI & automation
- Rust: `autoanswer` (placeholder polling), `analyze`, `digest`, `crm`, `hunt`, `message_digest`, `chat_analyzer`, `collect_chat_ideas`, `check_all_chats_tasks`, and `bulk_reactions`.
- Python legacy: `chat_analysis/`, plus retained helper modules `ab_testing.py` and `voice_utils.py`.
- Actions: port the remaining chat analyzer pipeline to Rust; improve auto-responder streaming; retire Python analyzers after parity tests.

### Linear
- Rust: `src/commands/linear.rs`, `src/linear.rs`, `src/bin/bots/linear_bot.rs`, `src/bin/linear/create_linear_tasks.rs`, and `src/bin/linear/sync_linear_tasks.rs`.
- Python legacy: no Linear root helpers remain.
- Actions: confirm feature parity (labels/projects) and add regression tests around the Rust path.

### Bots
- Rust: `src/bin/bots/sales_bot.rs`, `src/bin/bots/credit_expert_bot.rs`, `src/bin/bots/task_assistant_bot.rs`, `src/bin/bots/ai_project_consultant.rs`, `src/bin/bots/devops_ai_bot.rs`, `src/bin/bots/community_game_bot.rs`, and `src/bin/bots/linear_bot.rs`.
- Python legacy: no root bot scripts remain.
- Actions: finish operational hardening (services/config/deploy) and keep the shared MySQL schema in `migrations/002_create_bot_tables.sql`.

### Ops / Utilities
- Rust: `src/commands/n8n.rs`, `src/bin/ops/site_monitor.rs`, `src/bin/ops/http_bench.rs`, `src/bin/ops/k8s_dash.rs`, `src/bin/ops/devops_bot_probe.rs`, `src/bin/ops/n8n_backup.rs`, and `src/bin/ops/n8n_monitor.rs`.
- Python legacy: `mcp_telegram_server.py` remains and is currently broken because removed helper modules (`telegram_session`, `chat_export_utils`) are still imported.
- Actions: replace `mcp_telegram_server.py` with the specced Rust MCP server; keep shell/systemd templates in `scripts/ops/`.

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
- Keep Credit Expert and AI Project Consultant on the Rust bot path; add parity tests and operational hardening where gaps remain.

### Phase 3: Ops and integrations
- Keep Rust N8N monitor/backup as the production path; keep cron/systemd templates under `scripts/ops/`.
- Replace `mcp_telegram_server.py` with Rust MCP server or extend existing CLI for IDE agents.
- Confirm Linear Rust path covers all Python features; remove legacy scripts.

### Phase 4: Cleanup
- Remove/archive remaining Python surfaces once Rust parity exists and tests cover the replacement.
- Update README/AGENTS to Rust-only guidance (done).
- Add CI to block new Python additions (except migration shims/tests).

---

## Success criteria
- All production flows (export/read/reactions/digest/linear/N8N/bots) run on Rust binaries.
- Python surfaces marked deprecated or removed after parity validation.
- Tests cover migrated features (unit/integration where applicable).
- Single source of truth for prompts/config (Rust side).

## Risks
- Session format mismatch (Telethon vs grammers) → keep separate session files and locks.
- API drift between Python and Rust implementations → maintain regression suite and fixtures.
- Operational scripts (backup/monitor) may rely on OS-specific behaviors → test on target hosts before removing Python.

## Tracking
- Code changes in `src/` and `src/bin/<category>/`
- Legacy Python surfaces tracked for removal in Codev specs/plans or issues
- Updates documented in README/AGENTS
