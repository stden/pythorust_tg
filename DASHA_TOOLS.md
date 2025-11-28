# Dasha Tools (N8N + assistants)

Инструкции по инструментам, которые помогают следить за N8N и управлять сервисными ботами.

## N8N Monitor (`n8n_monitor.py`)
- Требуется: `N8N_URL`, `N8N_RESTART_COMMAND`, `CHECK_INTERVAL`, `MAX_RETRIES`, `TIMEOUT`
- Опционально: `N8N_API_KEY` (если включён API key), алерты в Telegram (`TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`, `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`)
- Запуск: `python n8n_monitor.py`
- Поведение: опрашивает `/healthz`, после `MAX_RETRIES` подряд выполняет `N8N_RESTART_COMMAND` и шлёт алерты. Для self-signed сертификатов используется `ssl=False`.

## N8N Backup (`n8n_backup.py`)
- Требуется: `N8N_URL`, `BACKUP_DIR`, `RETENTION_DAYS`, `MAX_BACKUPS`; опционально `N8N_API_KEY`
- Команды:
  - `python n8n_backup.py backup` — сохранить workflows + метаданные credentials в архив
  - `python n8n_backup.py list` — показать доступные архивы
  - `python n8n_backup.py cleanup` — удалить старые/лишние архивы по возрасту/количеству
  - `python n8n_backup.py restore --file <archive.tar.gz>` — распаковать архив; загрузка workflows через API пока не реализована, используйте файл `workflows.json` для ручного импорта
- Cron: используйте `n8n_backup_cron.sh`, пропишите `PROJECT_DIR` на корень проекта и убедитесь, что активируется нужное окружение (`.venv/bin/python` или `uv run`).

## Task Assistant Bot (`task_assistant_bot.py`)
- Требуется: `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TASK_ASSISTANT_BOT_TOKEN`; доступ ограничивается `ALLOWED_USERS` (comma-separated IDs)
- Использует `N8N_*` и `PROJECT_ROOT` для кнопок проверки/перезапуска/бэкапа. Имеет встроенную кнопку ИИ-консультанта.
- Запуск: `python task_assistant_bot.py`, команда `/start` выдаёт кнопки: проверить/перезапустить N8N, создать/показать бэкап, AI-ответы, статус серверов.

## AI Project Consultant (`ai_project_consultant.py`)
- Режимы: `--mode interactive` (CLI) или `--mode telegram`
- Требуется: `AI_CONSULTANT_BOT_TOKEN` (для режима Telegram), ключ LLM (`OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `GOOGLE_API_KEY`)
- Контекст: Markdown-файлы в `knowledge_base/`, путь можно задать через `KNOWLEDGE_BASE_PATH`.
