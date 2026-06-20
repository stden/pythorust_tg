# Telegram Chat Reader & Auto-responder

**⚠️ Rust first:** all new backend work and supported services must be in Rust. Python stays only for existing legacy/ops scripts until they are migrated.

## Project overview
Tools to:
- Read and export Telegram messages to Markdown files
- Track reactions and engagement
- Send AI-powered replies (OpenAI integration)
- Manage multiple chats and sessions
- Monitor and back up N8N plus service bots

## Capabilities

### Chat reading (Rust CLI `read`)
- Export chat history to Markdown (up to ~3000 messages)
- Track reactions and engagement
- Download media from high-engagement posts
- Auto-delete low-engagement messages
- Supports private chats and channels

### Auto-responder (Rust CLI `auto-answer`)
- AI replies via OpenAI API
- Polls/monitors messages (placeholder implementation)
- Configurable system instructions
- Session management via grammers

### Simple export (Rust CLI `tg`)
- Lightweight export flow
- Configurable message limit (default: 200)
- Download media for popular posts
- Show reactions and emoji

### Additional utilities
- 🤖 **AI Project Consultant** (`src/bin/bots/ai_project_consultant.rs`) — interactive mode and Telegram bot, searches answers in `knowledge_base/`
- 🛠️ **Task Assistant Bot** (`src/bin/bots/task_assistant_bot.rs`) — N8N control, backups, quick commands
- 🔍 **N8N Monitor** (`src/bin/ops/n8n_monitor.rs`, `n8n_monitor.service`) — health-check + auto-restart
- 💾 **N8N Backup** (`src/bin/ops/n8n_backup.rs`, `n8n_backup_cron.sh`) — backups and rotation
- 🛒 **Sales Bot** (`src/bin/bots/sales_bot.rs`) — Rust bot with MySQL logging and A/B prompt testing
- 🤝 **Credit Expert Bot** (`src/bin/bots/credit_expert_bot.rs`) — warm debt-consultant bot (MySQL dialog storage)

## Dependencies

Rust: standard toolchain (`cargo`).

Python legacy/ops:
```
telethon
openai
aiohttp
requests
pytest
python-dotenv
behave
```
Install with `uv sync`.

## Setup

### 1) Get Telegram API credentials
1. Go to https://my.telegram.org/
2. Sign in to your Telegram account
3. Open "API Development Tools"
4. Create an app
5. Save `API_ID` and `API_HASH`

### 2) Configure `.env`
```bash
cp .env.example .env
```
Set at minimum:
- `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TELEGRAM_PHONE`
- `OPENAI_API_KEY` and model (`OPENAI_MODEL` or CLI `--model`)
- `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` — for N8N monitor alerts
- `TASK_ASSISTANT_BOT_TOKEN` or `AI_CONSULTANT_BOT_TOKEN` — when running bots

Most runtime configuration is read from `.env`. Telegram credentials are required; AI/N8N/MySQL variables are only required for the features that use them.

### 3) Create a session (one-time)
```bash
cargo run -- init-session
```
The CLI uses `.env` values, asks for the Telegram code, and creates `telegram_session.session` (currently a fixed filename).

## Usage

### Chat history (Rust)
```bash
cargo run -- read chat_alpha --limit 3000 --delete-unengaged
```
Chats are configured in `config.yml`. If you omit the chat argument, the CLI uses its built-in defaults (`chat_alpha` for `read`, `chat_delta` for `tg`).

### Simple export (Rust)
```bash
cargo run -- tg chat_alpha --limit 200
```

### Auto-responder (Rust)
```bash
OPENAI_API_KEY=sk-... cargo run -- auto-answer --model gpt-4o-mini
```

### AI chat analysis (Python legacy)
```bash
python -m chat_analysis.analyzer @channel_name --provider openai --limit 800 --days 30 --output-format both
```
Session is taken from `TELEGRAM_SESSION_NAME/FILE`, LLM keys from `.env`. Results go to `analysis_results/` (JSON + Markdown). Custom prompt: `--prompt prompts/chat_categorizer.md`.

### N8N monitoring and backups
```bash
cargo run --bin n8n_monitor -- run
cargo run --bin n8n_backup -- backup
cargo run --bin n8n_backup -- list
cargo run --bin n8n_backup -- cleanup
cargo run --bin n8n_backup -- restore --file /srv/backups/n8n/<archive>.tar.gz
```

### Task Assistant Bot
```bash
cargo run --bin task_assistant_bot
```

### AI Project Consultant
```bash
cargo run --bin ai_project_consultant -- --mode interactive
cargo run --bin ai_project_consultant -- --mode telegram  # requires AI_CONSULTANT_BOT_TOKEN
```

### Specialized bots (MySQL)
- **Sales Bot** (`cargo run --bin sales_bot`) — Rust bot (sales funnel, objection handling).
- **Credit Expert Bot** (`cargo run --bin credit_expert_bot`) — warm debt consultant.

MySQL tables required: `bot_users`, `bot_sessions`, `bot_messages` (DDL in tests).

## Chat configuration

Chats live in `config.yml`:
```yaml
chats:
  example_channel:
    type: channel
    id: your_channel_id
  example_user:
    type: username
    username: example_name
```

## Key features
- Reaction tracking (counts, emoji extraction, engagement filters)
- Media handling (download media from popular posts)
- Message filtering (auto-delete zero-reaction posts, skip replies, drop certain patterns like Zoom links)
- AI integration (OpenAI GPT models, configurable system instructions, realtime replies)

## Environment-aware behavior
- CI/CD limits export to ~1000 messages
- Media downloads can be skipped in automated environments

## Security
- Session files contain auth tokens — keep them private
- API keys stay in `.env`
- OpenAI keys are read from environment in production

## Output format
Exports are saved as Markdown files:
```
[timestamp] [sender_name]: [message_text] [reactions] [media_path]
```
Example:
```
01.10.2025 12:30:45 SenderA: Hello, world! 🔥❤️👍
SenderB: Great post! 🎉
```

## Development
- Tests: `pytest`
- Windows venv helper: `create_venv.cmd`

## Notes
- Default limit: 3000 messages (1000 in GitHub Actions)
- Media download threshold: 100,000 reactions
- Session files are reused between runs
- Supports private chats and channels

## Contribution
Personal automation project — use as a reference for your own Telegram workflows.

## License
Personal project — follow Telegram ToS.

---

## Codev methodology

The project follows Codev (context-driven development).

### Active protocol
- Protocol: SPIDER-SOLO (single-developer variant)
- Location: `codev/protocols/spider-solo/protocol.md`

### Directory structure
- Specs: `codev/specs/`
- Plans: `codev/plans/`
- Reviews: `codev/reviews/`
- Resources: `codev/resources/`

### Workflow agents
Available in `.claude/agents/`:
- `spider-protocol-updater` — analyze SPIDER implementations and suggest improvements
- `architecture-documenter` — help with architecture documentation
- `codev-updater` — keep Codev installation up to date
- `tdd-tester` — TDD Red phase: writes failing tests
- `tdd-coder` — TDD Green phase: implements minimal code
- `tdd-refactorer` — TDD Refactor phase: cleans up code

See `codev/protocols/spider-solo/protocol.md` for full protocol docs.
