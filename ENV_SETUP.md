# ENV Setup

Шпаргалка по заполнению `.env` для Rust CLI и Python-скриптов.

```bash
cp .env.example .env
```

## Базовые (Telegram)
- `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TELEGRAM_PHONE`
- `TELEGRAM_SESSION_NAME`, `TELEGRAM_SESSION_FILE` — имя файла сессии
- `MY_ID` — ваш Telegram user ID (для фильтрации неинтересных сообщений)

> **Важно:** Rust (`telegram_session.session`) и Python (`+79117117850.session`) используют разные файлы сессий!

## AI-провайдеры
- `OPENAI_API_KEY` (обязательно для автоответчика/анализа), `OPENAI_MODEL` (по умолчанию: `gpt-4o-mini`)
- Дополнительно: `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY` — для альтернативных моделей

## Алерты и боты
- `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` — алерты из `n8n_monitor.py`
- `TASK_ASSISTANT_BOT_TOKEN`, `ALLOWED_USERS` — доступ к `task_assistant_bot.py`
- `AI_CONSULTANT_BOT_TOKEN`, `AI_CONSULTANT_MODEL`, `AI_CONSULTANT_TEMPERATURE`, `KNOWLEDGE_BASE_PATH`

## N8N мониторинг и бэкапы
- `N8N_URL` (например, `https://n8n.example.com`)
- `N8N_API_KEY` (опционально, если включён API key)
- `N8N_RESTART_COMMAND` — команда рестарта (например, `systemctl restart n8n`)
- `CHECK_INTERVAL`, `MAX_RETRIES`, `TIMEOUT` — интервалы для `n8n_monitor.py`
- `BACKUP_DIR`, `RETENTION_DAYS`, `MAX_BACKUPS` — куда сохранять архивы и правила ротации

## Базы данных и нишевые боты
- `BFL_SALES_BOT_TOKEN`, `CREDIT_EXPERT_BOT_TOKEN`
- `MYSQL_HOST`, `MYSQL_PORT`, `MYSQL_DATABASE`, `MYSQL_USER`, `MYSQL_PASSWORD`

## Linear и вспомогательные интеграции
- `LINEAR_API_KEY`, `LINEAR_TEAM_KEY`, `LINEAR_PROJECT_ID`, `LINEAR_COMMAND_PREFIX`, `LINEAR_ALLOWED_SENDERS`
- `BITRIX24_WEBHOOK_URL`, `BITRIX24_USER_ID` (если используете Bitrix24)

## Экспорт чатов и фильтры
- `DEFAULT_CHAT` — алиас из `config.yml` для `read.py`/`tg.py`
- `MEDIA_REACTION_LIMIT`, `ZOOM_CLEANUP_CHATS`, `ZOOM_URL_PATTERN`, `LOW_ENGAGEMENT_SENDER_IDS`
- `LIKE_CHAT_ID`, `LIKE_TARGET_USER_ID` — автоматические реакции

## MCP сервер (FastMCP + Telethon)
- `MCP_TELEGRAM_LIMIT` — сколько сообщений возвращает `fetch_recent_messages` (по умолчанию 50)
- `MCP_TELEGRAM_MAX_LIMIT` — верхний порог лимита (по умолчанию 200)
- `MCP_TELEGRAM_LOCK_RETRY` — задержка повторных попыток захвата сессионного lock-файла (секунды)

## Пути и логи
- `PROJECT_ROOT`, `PROMPTS_DIR`, `ANALYSIS_RESULTS_DIR`, `CHATS_EXPORT_DIR`
- `LOG_LEVEL`, `LOG_FILE`

Все дополнительные переменные и комментарии смотрите в `.env.example`.
