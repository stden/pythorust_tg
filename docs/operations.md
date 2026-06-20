# Ops runbook

This doc covers monitoring/backup and helper bots used for ops. Everything here runs on Rust — main CLI subcommands (`cargo run -- ...`) or standalone binaries (`cargo run --bin ...`).

## N8N monitor

### Rust CLI (recommended)
```bash
cargo run -- n8n-monitor
```

Env:
- `N8N_URL`, `N8N_RESTART_COMMAND`
- Optional: `N8N_API_KEY`
- Optional alerts: `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`

### systemd template
`scripts/ops/n8n_monitor.service` is a template: update `WorkingDirectory`, `EnvironmentFile`, and `ExecStart`, then install:
```bash
sudo cp scripts/ops/n8n_monitor.service /etc/systemd/system/n8n_monitor.service
sudo systemctl daemon-reload
sudo systemctl enable --now n8n_monitor.service
```

## N8N backup

### Rust CLI
```bash
cargo run -- n8n-backup backup
cargo run -- n8n-backup list
cargo run -- n8n-backup cleanup
cargo run -- n8n-backup restore --file /srv/backups/n8n/n8n_backup_YYYYMMDD_HHMMSS.tar.gz
```

Cron template:
- `scripts/ops/n8n_backup_cron.sh` is a template; set `PROJECT_DIR` inside and add to crontab.

## Task Assistant bot (Rust)
```bash
cargo run --bin task_assistant_bot
```

Env:
- `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`
- `TASK_ASSISTANT_BOT_TOKEN`
- Optional allowlist: `ALLOWED_USERS` (comma-separated user ids)
- AI answers: `OPENAI_API_KEY`, `OPENAI_MODEL`

## DevOps AI bot (Rust)
```bash
cargo run --bin devops_ai_bot
```

Config:
- `devops_bot.yml` (services + commands)
- `DEVOPS_BOT_CONFIG` to override config path

Env:
- `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`
- `DEVOPS_BOT_TOKEN` (or `TASK_ASSISTANT_BOT_TOKEN` / `TELEGRAM_BOT_TOKEN` fallback)
- Optional allowlist: `DEVOPS_ALLOWED_USERS`
