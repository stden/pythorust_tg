# Environment setup (`.env`)

This repo loads configuration from:
- `.env` (Rust uses `dotenvy`, Python uses `python-dotenv`)
- `config.yml` (chat aliases, limits, some defaults)

## Quick start
1) Copy the template:
```bash
cp .env.example .env
```
2) Fill Telegram API credentials (required for any Telegram features):
- `TELEGRAM_API_ID`
- `TELEGRAM_API_HASH`
- `TELEGRAM_PHONE`
3) Create a session once (creates `telegram_session.session` in the project dir):
```bash
cargo run -- init-session
```
4) Add your chats to `config.yml` (aliases → ids/usernames).

## What to set (by feature)

### Rust CLI (core)
Required:
- `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TELEGRAM_PHONE`
Recommended:
- `USER_ID` (your Telegram user id; used by some commands/filters)

Notes:
- Rust CLI currently uses a fixed session filename: `telegram_session.session`.
- `TELEGRAM_SESSION_NAME` / `TELEGRAM_SESSION_FILE` are kept for legacy Python tools; leave them as `telegram_session` unless you know you need a different Telethon session name.

### AI features (auto-answer, digest, analyze, crm, hunt)
Set one (or more) provider credentials:
- OpenAI: `OPENAI_API_KEY` (+ optional `OPENAI_MODEL`)
- Anthropic: `ANTHROPIC_API_KEY`
- Google Gemini: `GOOGLE_API_KEY`

Optional analyzer overrides (used by Rust `analyze` and/or legacy scripts depending on tool):
- `CHAT_ANALYZER_LLM_PROVIDER`, `CHAT_ANALYZER_MODEL`, `CHAT_ANALYZER_OUTPUT_DIR`

### N8N monitor / backup
Rust CLI (`telegram_reader n8n-monitor` / `telegram_reader n8n-backup ...`):
- `N8N_URL`
- `N8N_API_KEY` (if your instance requires it)
- `N8N_RESTART_COMMAND` (for monitor auto-restart)
- `CHECK_INTERVAL`, `MAX_RETRIES`, `TIMEOUT` (optional; Rust has defaults)
- `BACKUP_DIR`, `RETENTION_DAYS`, `MAX_BACKUPS` (optional; Rust has defaults)

Optional Telegram alerts (recommended for monitor):
- `TELEGRAM_BOT_TOKEN`
- `TELEGRAM_CHAT_ID`

Legacy Python scripts (`n8n_monitor.py`, `n8n_backup.py`) are stricter: they expect most numeric values to be present (see `.env.example`).

### Task Assistant / DevOps bots (legacy Python)
- `TASK_ASSISTANT_BOT_TOKEN` (Task Assistant bot)
- `DEVOPS_BOT_TOKEN` or `TELEGRAM_BOT_TOKEN` (DevOps AI bot token fallback)
- `ALLOWED_USERS` / `DEVOPS_ALLOWED_USERS` (optional allowlist; comma-separated ids)
- `OPENAI_API_KEY` and `OPENAI_MODEL` (for AI answers)

### MySQL-backed bots/analytics
- `MYSQL_HOST`, `MYSQL_PORT`, `MYSQL_DATABASE`, `MYSQL_USER`, `MYSQL_PASSWORD`
- `BFL_SALES_BOT_TOKEN` / `CREDIT_EXPERT_BOT_TOKEN` (depending on bot)

## Security notes
- `*.session` files contain Telegram auth tokens — keep them private and back them up securely.
- Never commit `.env` to git.
