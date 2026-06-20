# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

Telegram Automation Toolkit — Rust CLI (`telegram_reader`) with AI integrations for chat export, analysis, moderation, CRM, reactions, and bot automation. Python is legacy-only; all new work must be Rust.

## Build and test commands

```bash
# Dev build
cargo build

# Optimized build
cargo build --release

# Max optimization (fat LTO)
cargo build --profile release-optimized

# Run CLI
./target/release/telegram_reader --help

# Run a standalone binary
cargo run --bin sales_bot
cargo run --bin test_bot_dialogue -- --bot @example_bot --file dialogue.md

# Tests
cargo test                              # all tests
cargo test test_name                    # single test by name
cargo test --lib                        # library tests only
cargo test --bin sales_bot          # tests for a specific binary

# Pre-commit checks
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test

# CI/CD pipeline runs
cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release

# Benchmarks
cargo bench

# Python legacy tests
uv run pytest tests/ -v
```

## Development rules

1. **Commit messages**: English, prefix `[+]` (new) / `[-]` (remove) / `[!]` (fix/change), briefly reflecting the change
2. Rust only for new functionality; migrate Python over time
3. Async/await with tokio throughout
4. TDD workflow: Red (failing test) → Green (minimal pass) → Blue (refactor). Test naming: `test_<feature>_<scenario>`
5. No `.unwrap()` in production code — use `Result<T, E>` with `?` operator
6. Prefer iterators, references, and zero-cost abstractions

## Architecture

### CLI structure (Clap derive)

`src/main.rs` defines the CLI via `#[derive(Parser)]` with subcommands. Each subcommand dispatches to a function in `src/commands/`. The `Cli` struct accepts an optional `--metrics-addr` for Prometheus, then a subcommand enum.

### Session management

`src/session.rs` provides file-based exclusive locking (`SessionLock`) via `fs2` to prevent parallel Telegram connections. `get_client()` creates a `grammers_client::Client` using `SqliteSession` storage. The session file is `telegram_session.session`; lock file is `telegram_reader.lock`. **Only one process can use the Telegram session at a time.**

### Library modules (`src/lib.rs`)

The crate is both a library and a CLI binary (plus 52 specialized binaries in `src/bin/<category>/`). Key modules:
- `commands/` — CLI command implementations (read, tg, digest, moderate, crm, hunt, like, react, analyze, linear, n8n, etc.)
- `integrations/` — AI provider clients: OpenAI (`async-openai`), Gemini, Claude, Ollama, Yandex TTS
- `analysis/` — Vector DB (Qdrant) and graph DB (Neo4j) helpers
- `lightrag/` — LightRAG support
- `config.rs` — YAML config loading from `config.yml`, known senders, session constants
- `error.rs` — `thiserror`-based error enum with variants for session, Telegram, AI, MySQL, etc.
- `prompts.rs` — System prompt loading from `prompts/` directory (Markdown files)
- `metrics.rs` — Prometheus metrics server
- `reactions.rs` — Reaction tracking and engagement
- `export.rs` — Export formatting
- `python.rs` — PyO3 bindings for MCP server integration

### Standalone binaries (`src/bin/`)

52 specialized binaries for focused tasks: bots (`sales_bot`, `credit_expert_bot`, `community_game_bot`, `devops_ai_bot`), utilities (`delete_unanswered`, `bulk_reactions`, `http_bench`), data tools (`index_messages`, `export_chats_mysql`, `k8s_dash`), and more. These import from the `telegram_reader` library crate.

### Chat configuration

Chats are defined in `config.yml` with type (channel/username) and ID. Commands accept chat names that resolve through this config.

### AI integration pattern

Commands that use AI (analyze, digest, auto-answer, moderate) load system prompts from `prompts/*.md` via `prompts.rs`, then call the appropriate integration client. The default model comes from `OPENAI_MODEL` env var.

### Bot architecture

Bots (`sales_bot`, `credit_expert_bot`) use `teloxide` for the Telegram Bot API and `mysql_async` for dialog/session persistence. The sales bot supports A/B prompt testing via `SALES_PROMPT_EXPERIMENT`.

## Environment

Required env vars: `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TELEGRAM_PHONE`. AI features need `OPENAI_API_KEY`. Bots need their `*_BOT_TOKEN` and MySQL creds. N8N features need `N8N_URL` and `N8N_API_KEY`. Session init: `cargo run -- init-session`.

## Python (legacy) — do not use for new code

Remaining Python surfaces: `chat_analysis`, `mcp_telegram_server.py` (currently broken due missing legacy helper modules), `ab_testing.py`, and `voice_utils.py`. Install deps: `uv sync`. Already migrated to Rust (`src/bin/<category>/`): `n8n_monitor`, `n8n_backup`, `sales_bot`, `credit_expert_bot`, `task_assistant_bot`, `devops_ai_bot`, `community_game_bot`, `ai_project_consultant`, `check_all_chats_tasks`, `collect_chat_ideas`, `load_messages_to_db`, and `sync_linear_tasks`.

## Codev methodology

The project uses SPIDER-SOLO protocol (`codev/protocols/spider-solo/protocol.md`). Specs in `codev/specs/`, plans in `codev/plans/`, reviews in `codev/reviews/`. TDD agents available in `.claude/agents/` (tdd-tester, tdd-coder, tdd-refactorer).
