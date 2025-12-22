# Configuration summary

Quick, copy-paste oriented guide for the most common `.env` setups. For the full list, use `.env.example` (source of truth) and `ENV_SETUP.md`.

## Minimal `.env` for Rust CLI chat export
```env
TELEGRAM_API_ID=123456
TELEGRAM_API_HASH=abcdef1234567890
TELEGRAM_PHONE=+70000000000
USER_ID=123456789
```

Init once:
```bash
cargo run -- init-session
```

## Minimal `.env` for AI commands (auto-answer/digest/analyze)
```env
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini
```

## Minimal `.env` for N8N monitor (Rust CLI)
```env
N8N_URL=https://n8n.example.com
N8N_RESTART_COMMAND="systemctl restart n8n"
CHECK_INTERVAL=60
MAX_RETRIES=3
TIMEOUT=30

# Optional: alerts
TELEGRAM_BOT_TOKEN=123:bot-token
TELEGRAM_CHAT_ID=123456789
```

## Minimal `.env` for N8N backup (Rust CLI or Python)
```env
N8N_URL=https://n8n.example.com
BACKUP_DIR=/srv/backups/n8n
RETENTION_DAYS=30
MAX_BACKUPS=50
```

## Minimal `.env` for Task Assistant bot (legacy Python)
```env
TELEGRAM_API_ID=123456
TELEGRAM_API_HASH=abcdef1234567890
TASK_ASSISTANT_BOT_TOKEN=123:bot-token
ALLOWED_USERS=123456789,987654321

OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini
```

## Minimal `.env` for MySQL bots/analytics
```env
MYSQL_HOST=localhost
MYSQL_PORT=3306
MYSQL_DATABASE=pythorust_tg
MYSQL_USER=pythorust_tg
MYSQL_PASSWORD=...
```
