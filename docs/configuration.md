# Configuration (`.env`)

Configuration is loaded from:
- `.env` (Rust uses `dotenvy`, Python uses `python-dotenv`)
- `config.yml` (chat aliases, limits, some defaults)
- `devops_bot.yml` (DevOps AI bot services/commands; override path with `DEVOPS_BOT_CONFIG`)

`.env.example` is the **source of truth** for the full variable list and defaults.
This page gives copy-paste quick-starts (top) and a by-feature reference (bottom).

## Quick start
1) Copy the template:
```bash
cp .env.example .env
```
2) Fill Telegram API credentials (required for any Telegram feature):
`TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TELEGRAM_PHONE`
3) Create a session once (writes `telegram_session.session` in the project dir):
```bash
cargo run -- init-session
```
4) Add your chats to `config.yml` (aliases → ids/usernames).

---

## Copy-paste minimal setups

### Rust CLI chat export
```env
TELEGRAM_API_ID=your_api_id
TELEGRAM_API_HASH=your_api_hash
TELEGRAM_PHONE=your_phone_number
USER_ID=your_telegram_user_id
```

### AI commands (auto-answer / digest / analyze)
```env
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini
```

### N8N monitor (Rust CLI)
```env
N8N_URL=https://n8n.example.com
N8N_RESTART_COMMAND="systemctl restart n8n"
CHECK_INTERVAL=60
MAX_RETRIES=3
TIMEOUT=30

# Optional: alerts
TELEGRAM_BOT_TOKEN=123:bot-token
TELEGRAM_CHAT_ID=your_alert_chat_id
```

### N8N backup (Rust CLI)
```env
N8N_URL=https://n8n.example.com
BACKUP_DIR=/srv/backups/n8n
RETENTION_DAYS=30
MAX_BACKUPS=50
```

### Task Assistant bot (Rust)
```env
TELEGRAM_API_ID=your_api_id
TELEGRAM_API_HASH=your_api_hash
TASK_ASSISTANT_BOT_TOKEN=123:bot-token
ALLOWED_USERS=comma_separated_user_ids

OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini
```

### MySQL bots / analytics
```env
MYSQL_HOST=localhost
MYSQL_PORT=3306
MYSQL_DATABASE=pythorust_tg
MYSQL_USER=pythorust_tg
MYSQL_PASSWORD=...
```

---

## Reference (by feature)

### Rust CLI (core)
- Required: `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TELEGRAM_PHONE`
- Recommended: `USER_ID` (your Telegram user id; used by some commands/filters)
- The Rust CLI uses a fixed session filename: `telegram_session.session`.
- `TELEGRAM_SESSION_NAME` / `TELEGRAM_SESSION_FILE` name the Telegram session;
  leave as `telegram_session` unless you need a different one (also used by the
  remaining Python tools, e.g. `mcp_telegram_server.py`).

### AI features (auto-answer, digest, analyze, crm, hunt)
Set one or more provider credentials:
- OpenAI: `OPENAI_API_KEY` (+ optional `OPENAI_MODEL`)
- Anthropic: `ANTHROPIC_API_KEY`
- Google Gemini: `GOOGLE_API_KEY`

Optional analyzer overrides:
- `CHAT_ANALYZER_LLM_PROVIDER`, `CHAT_ANALYZER_MODEL`, `CHAT_ANALYZER_OUTPUT_DIR`

### N8N monitor / backup
Rust CLI (`telegram_reader n8n-monitor` / `telegram_reader n8n-backup ...`):
- `N8N_URL`, `N8N_API_KEY` (if required), `N8N_RESTART_COMMAND` (monitor auto-restart)
- Monitor tuning (optional; Rust has defaults): `CHECK_INTERVAL`, `MAX_RETRIES`, `TIMEOUT`
- Backup tuning (optional; Rust has defaults): `BACKUP_DIR`, `RETENTION_DAYS`, `MAX_BACKUPS`
- Optional alerts: `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`

See [operations.md](operations.md) for runbooks and the `scripts/ops/` templates.

### Task Assistant / DevOps bots (Rust)
- `TASK_ASSISTANT_BOT_TOKEN` (Task Assistant bot)
- `DEVOPS_BOT_TOKEN` or `TELEGRAM_BOT_TOKEN` (DevOps AI bot token fallback)
- `ALLOWED_USERS` / `DEVOPS_ALLOWED_USERS` (optional allowlist; comma-separated ids)
- `OPENAI_API_KEY`, `OPENAI_MODEL` (for AI answers)

### MySQL-backed bots / analytics
- `MYSQL_HOST`, `MYSQL_PORT`, `MYSQL_DATABASE`, `MYSQL_USER`, `MYSQL_PASSWORD`
- `SALES_BOT_TOKEN` / `CREDIT_EXPERT_BOT_TOKEN` (depending on bot)

## Security notes
- `*.session` files contain Telegram auth tokens — keep them private and back them up securely.
- Never commit `.env` to git.
