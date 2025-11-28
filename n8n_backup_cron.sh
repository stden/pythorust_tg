#!/bin/bash
# N8N Automatic Backup Script
# ВАЖНО: Замените /path/to/project на реальный путь
# Add to crontab: 0 2 * * * /path/to/project/n8n_backup_cron.sh

set -e

# ВАЖНО: Установите правильный путь к проекту
PROJECT_DIR="/path/to/project"
cd "$PROJECT_DIR"

# Activate virtual environment
source .venv/bin/activate

# Run backup
python n8n_backup.py backup

# Cleanup old backups
python n8n_backup.py cleanup

echo "$(date): N8N backup completed" >> /var/log/n8n_backup.log
