# Configuration summary

Quick, copy-paste oriented guide for the most common `.env` setups. For the full list, use `.env.example` (source of truth) and `ENV_SETUP.md`.

## Minimal `.env` for Rust CLI chat export
```env
TELEGRAM_API_ID=your_api_id
TELEGRAM_API_HASH=your_api_hash
TELEGRAM_PHONE=your_phone_number
USER_ID=your_telegram_user_id
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
TELEGRAM_CHAT_ID=your_alert_chat_id
```

## Minimal `.env` for N8N backup (Rust CLI or Python)
```env
N8N_URL=https://n8n.example.com
BACKUP_DIR=/srv/backups/n8n
RETENTION_DAYS=30
MAX_BACKUPS=50
```

## Minimal `.env` for Task Assistant bot (Rust)
```env
TELEGRAM_API_ID=your_api_id
TELEGRAM_API_HASH=your_api_hash
TASK_ASSISTANT_BOT_TOKEN=123:bot-token
ALLOWED_USERS=comma_separated_user_ids

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
