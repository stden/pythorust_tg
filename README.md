# Telegram Automation Toolkit (Rust-first)

Rust-first toolkit for Telegram automation. High-speed chat exports, AI auto-responder, reactions/digests/moderation/CRM parsing, LightRAG indexing, and ops helpers (N8N monitor/backup, HTTP bench, site monitor, k8s dashboard). Python scripts stay only for legacy bots/ops while they are migrated to Rust.

## What you get
- Chat export and insights: Markdown exports with reactions/media, active chat listing, AI-powered analysis and digests, optional cleanup of low-engagement posts.
- AI automation: auto-responder, digest summarization, profanity stats/moderation, CRM/contact parsing, user hunting by keywords, viral question sender.
- Reactions and hygiene: targeted likes, reaction blasts from ids/links/files, delete Zoom invites, delete unanswered or low-reaction posts.
- DevOps and ops: N8N monitor + backups, HTTP benchmark, site availability monitor, k8s dashboard, Prometheus metrics for CLI commands.
- RAG and data: LightRAG indexer over Telegram history/MySQL, chat indexing and search helpers.
- Bots: Rust BFL Sales Bot with MySQL logging and A/B prompts; legacy Python bots (Credit Expert, AI Project Consultant, Task Assistant) until they are migrated.

## Why Rust
- grammers-based MTProto client: faster and leaner than Python clients.
- Type-safe async pipeline with clear error handling.
- Session lock prevents concurrent runs against the same account.
- Tracing + Prometheus metrics for observability.

## Install

### Rust (recommended)
```bash
cargo build --release
# binaries land in target/release
```

### Python (legacy/ops helpers)
```bash
brew install uv   # or pip install uv
uv sync           # creates .venv with requirements
# fallback:
pip install -r requirements.txt
```

## Configure
1) Get Telegram API credentials at https://my.telegram.org/ (API_ID, API_HASH, phone).
2) Copy `.env.example` to `.env` and fill the basics:
```env
TELEGRAM_API_ID=123456
TELEGRAM_API_HASH=abcdef1234567890
TELEGRAM_PHONE=+70000000000
TELEGRAM_SESSION_NAME=telegram_session
TELEGRAM_SESSION_FILE=telegram_session

OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini
LINEAR_API_KEY=lin_api_...

# Ops / N8N / bots
N8N_URL=https://n8n.example.com
N8N_API_KEY=...
TELEGRAM_BOT_TOKEN=...
TELEGRAM_CHAT_ID=...
TASK_ASSISTANT_BOT_TOKEN=...
AI_CONSULTANT_BOT_TOKEN=...

# MySQL for bots/analytics
MYSQL_HOST=localhost
MYSQL_PORT=3306
MYSQL_DATABASE=pythorust_tg
MYSQL_USER=pythorust_tg
MYSQL_PASSWORD=...
```
3) Initialize the session once (uses `.env` values and creates `telegram_session.session` (currently a fixed filename)):
```bash
cargo run -- init-session
```
4) Map your chats in `config.yml` (aliases → channel/user id or username):
```yaml
chats:
  my_channel:
    type: channel
    id: 1234567890
  my_user:
    type: username
    username: example_name
```

## Usage (Rust CLI `telegram_reader`)

### Chat export and listing
```bash
cargo run -- list-chats --limit 20 --filter channels
cargo run -- active-chats --limit 20
cargo run -- dialogs --limit 50 --format table --output dialogs.yaml
cargo run -- read chat_alpha --limit 3000 --delete-unengaged
cargo run -- tg chat_alpha --limit 200
cargo run -- export username --limit 300 --output chat.md
cargo run -- delete-zoom username --limit 3000
```

### AI automation and analysis
```bash
OPENAI_API_KEY=sk-... cargo run -- auto-answer --model gpt-4o-mini
cargo run -- digest chat_alpha --hours 24 --limit 500 --model gpt-4o-mini
cargo run -- analyze @channel --provider openai --limit 800 --days 30 --output-format both --prompt prompts/chat_categorizer.md
cargo run -- crm chat_alpha --limit 100 --export-csv contacts.csv --model gpt-4o-mini
cargo run -- hunt --chats chat1,chat2 --keywords "jobs,vacancy" --required "python" --exclude "spam" --days 30 --export-csv results.csv --top 50
```

### Reactions and moderation
```bash
cargo run -- react --chat chat_alias --ids 123,124,125 --emoji "🔥" --delay-ms 600 --dry-run
cargo run -- react --chat chat_alias --file ids.txt --recent 20 --user-id 123456 --emoji "🔥"
cargo run -- like --chat chat_alias --user target_user --emoji "❤️" --limit 200
cargo run -- moderate chat_alpha --delete --warn
cargo run -- profanity-stats chat_alpha --limit 1000
cargo run -- n8n-monitor
cargo run -- n8n-backup backup
```

### Additional binaries
```bash
# Delete your unanswered messages
cargo run --bin delete_unanswered -- --all --limit 500 --hours 1

# LightRAG index/query
cargo run --bin lightrag -- --index --limit 3000
cargo run --bin lightrag -- --query "who is looking for designers?" --mode hybrid --results 5

# Message indexing and search helpers
cargo run --bin index_messages -- --chat chat_alpha --limit 2000
cargo run --bin search_messages -- --chat chat_alpha --query "linear bug" --limit 200

# HTTP bench / site monitor / k8s dash
cargo run --bin http_bench -- https://api.example.com -c 100 -d 10
cargo run --bin site_monitor -- watch --interval 60 https://example.com
cargo run --bin k8s_dash -- pods default
```

### Bots and ops
```bash
# Rust BFL Sales Bot with MySQL logging and A/B prompts
cargo run --bin bfl_sales_bot

# Legacy Python bots (pending Rust migration)
uv run python credit_expert_bot.py
uv run python ai_project_consultant.py --mode interactive
uv run python task_assistant_bot.py
uv run python n8n_monitor.py
uv run python n8n_backup.py backup
```
MySQL tables expected for bots: `bot_users`, `bot_sessions`, `bot_messages`, and `bot_experiments` (see `tests/test_credit_*` for schema hints).

## Project layout
```
src/                    # Rust library + CLI commands
src/bin/                # Standalone binaries (bfl_sales_bot, delete_unanswered, lightrag, http_bench, k8s_dash, etc.)
prompts/                # System prompts used by analyzers/bots
chat_analysis/          # Legacy Python analyzer
analysis_results/       # Stored outputs (JSON/Markdown)
codev/                  # Specs, plans, protocols, reviews
.claude/agents/         # Agent configs (mirrors codev/agents)
```

## Capability matrix
| Feature | Rust CLI | Python (legacy) | Status |
|---------|----------|-----------------|--------|
| Chat export to Markdown | ✅ | Legacy scripts | Stable |
| AI auto-responder | ✅ | Legacy variant | In progress (Rust improvements planned) |
| Linear integration | ✅ | Legacy helper | Stable |
| Reactions/likes | ✅ | — | Stable |
| Delete Zoom / cleanup | ✅ | Legacy scripts | Stable |
| AI digest / analysis | ✅ | Legacy analyzer | Stable |
| CRM parsing / user hunt | ✅ | — | New |
| N8N monitor/backup | ✅ CLI | ✅ Python | Stable |
| LightRAG indexing | ✅ | — | New |
| K8s dashboard / HTTP bench / site monitor | ✅ | — | Stable |
| Gifts / Stories API | 🔜 | — | Planned |

## Security
- Session files contain Telegram auth tokens — keep them private.
- Secrets live in `.env`; never commit them.
- Session lock prevents concurrent runs against the same account.

## Development and testing
```bash
# Rust
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all

# Python
uv run pytest -v
```

## Documentation
- `AGENTS.md` — project overview and quickstart for operators.
- `CLAUDE.md` — Rust-first engineering guidelines and CLI reference.
- `CODEX.md` — testing playbook for bots and data flows.
- `ENV_SETUP.md` — full `.env` reference (Telegram/AI/N8N/MySQL/etc.).
- `CONFIGURATION_SUMMARY.md` — minimal env variable sets per feature.
- `DASHA_TOOLS.md` — ops runbook (N8N monitor/backup + templates).
- `ROADMAP.md` — current priorities.
- `codev/` — protocols, specs, plans, and reviews (SPIDER-SOLO).
