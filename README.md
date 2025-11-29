# 🚀 Telegram Automation Toolkit

> **Python + Rust** — мощные инструменты для автоматизации Telegram чатов

[![Python](https://img.shields.io/badge/Python-3.11+-3776AB?style=for-the-badge&logo=python&logoColor=white)](https://python.org)
[![Rust](https://img.shields.io/badge/Rust-1.70+-000000?style=for-the-badge&logo=rust&logoColor=white)](https://rust-lang.org)
[![Telegram](https://img.shields.io/badge/Telegram-MTProto-26A5E4?style=for-the-badge&logo=telegram&logoColor=white)](https://core.telegram.org/mtproto)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](LICENSE)

---

## 🎯 Какие задачи решает?

Этот тулкит — ваш швейцарский нож для работы с Telegram. Забудьте про ручную рутину!

### 📖 Чтение и экспорт чатов

| Функция | Описание |
|---------|----------|
| 📥 **Экспорт в Markdown** | Сохраняйте историю чатов с реакциями, эмодзи и метаданными |
| 📊 **Анализ вовлечённости** | Узнайте, какие сообщения собирают больше лайков и ответов |
| 🖼️ **Скачивание медиа** | Автоматическое сохранение фото/видео из популярных постов |
| 🔍 **Поиск пользователей** | Найдите любого участника чата по имени |
| 📋 **Список чатов** | Получите полный список ваших диалогов с метаданными |

### 🤖 AI-автоматизация

| Функция | Описание |
|---------|----------|
| 💬 **Умный автоответчик** | Автоответы на gpt-4o/Claude/Gemini (настраивается через `.env`) |
| 🧠 **Контекстные реакции** | Бот ставит подходящие эмодзи в зависимости от содержания |
| 📝 **Linear интеграция** | Превращайте сообщения в задачи одной командой `!linear` |
| ❤️ **Массовые реакции** | Автоматически лайкайте сообщения нужных людей |
| 🤝 **AI Project Consultant** | RAG-консультант по вашей базе знаний `knowledge_base/` |
| 📋 **Промпты в Markdown** | Все системные промпты хранятся в `prompts/*.md` |

### 🧹 Автоочистка и гигиена

| Функция | Описание |
|---------|----------|
| 🗑️ **Удаление неинтересного** | Автоудаление ваших сообщений без реакций |
| 🔗 **Чистка Zoom-ссылок** | Удаляйте устаревшие ссылки на созвоны |
| 🚫 **Фильтрация спама** | Настраиваемые правила очистки |

### 🔧 DevOps и мониторинг

| Функция | Описание |
|---------|----------|
| 🌐 **Мониторинг сайтов** | Проверка доступности с уведомлениями в Telegram |
| ⚡ **HTTP бенчмарк** | Аналог `wrk` — тестируйте производительность API |
| 📈 **Алерты** | Мгновенные уведомления о проблемах |
| 🩺 **N8N мониторинг/бэкапы** | Health-check + автоперезапуск, бэкап/restore workflows |
| ☸️ **K8s Dashboard** | Простой CLI для управления Kubernetes (pods, logs, nodes) |

---

## 🦀 Почему Rust?

### Проблемы Python-библиотек

```
😤 Pyrogram — устарел, баги не фиксятся
😤 Telethon — медленный, жрёт память
😤 Нет поддержки новых API (gifts, stories, reactions)
```

### Решение: grammers 🦀

**grammers** — современный MTProto клиент на чистом Rust:

| Преимущество | Описание |
|--------------|----------|
| 🚀 **Скорость** | В 10-50x быстрее Python |
| 🪶 **Лёгкость** | Минимальное потребление RAM |
| ✨ **Актуальность** | Поддержка всех новых API Telegram |
| 🔒 **Type-safe** | Ошибки ловятся на этапе компиляции |
| 🎁 **Gifts API** | Работа с подарками (то, чего нет в Pyrogram!) |
| 📖 **Stories** | Полная поддержка историй |
| ⭐ **Reactions** | Все типы реакций включая premium |

### Сравнение производительности

```
┌─────────────────┬──────────┬──────────┐
│ Операция        │ Python   │ Rust     │
├─────────────────┼──────────┼──────────┤
│ Загрузка 1000   │ 12.3s    │ 0.8s     │
│ сообщений       │          │          │
├─────────────────┼──────────┼──────────┤
│ Экспорт чата    │ 8.5s     │ 0.3s     │
├─────────────────┼──────────┼──────────┤
│ RAM при работе  │ 150 MB   │ 12 MB    │
└─────────────────┴──────────┴──────────┘
```

---

## 📦 Установка

### Python (для быстрого старта)

```bash
# Рекомендуется: UV package manager
brew install uv  # или pip install uv
uv sync

# Классический способ
pip install -r requirements.txt
```

### Rust (для максимальной производительности)

```bash
cargo build --release

# Бинарники появятся в target/release/
```

---

## 🔐 Настройка

### 1️⃣ Получите API credentials

1. Откройте https://my.telegram.org/
2. Войдите в свой аккаунт
3. Перейдите в "API Development Tools"
4. Создайте приложение
5. Сохраните `API_ID` и `API_HASH`

### 2️⃣ Создайте `.env` файл

```bash
cp .env.example .env
```

```env
# Обязательные
TELEGRAM_API_ID=12345678
TELEGRAM_API_HASH=abcdef1234567890
TELEGRAM_PHONE=+79001234567
TELEGRAM_SESSION_NAME=telegram_session
TELEGRAM_SESSION_FILE=telegram_session

# AI провайдеры (выберите нужные)
OPENAI_API_KEY=sk-...           # gpt-4o / gpt-4o-mini
GOOGLE_API_KEY=AI...            # Gemini 2.x / 1.5
ANTHROPIC_API_KEY=sk-ant-...    # Claude Sonnet / Haiku

# Интеграции
LINEAR_API_KEY=lin_api_...      # Linear для задач

# Ops: мониторинг/бэкапы/боты
N8N_URL=https://n8n.example.com
N8N_API_KEY=n8n_api_...
N8N_RESTART_COMMAND=systemctl restart n8n
TELEGRAM_BOT_TOKEN=bot_token_for_alerts
TELEGRAM_CHAT_ID=123456789
TASK_ASSISTANT_BOT_TOKEN=task_assistant_token
AI_CONSULTANT_BOT_TOKEN=ai_consultant_token
KNOWLEDGE_BASE_PATH=./knowledge_base

# Нишевые боты (MySQL)
BFL_SALES_BOT_TOKEN=your_bfl_sales_bot_token
CREDIT_EXPERT_BOT_TOKEN=your_credit_expert_bot_token
MYSQL_HOST=localhost
MYSQL_PORT=3306
MYSQL_DATABASE=pythorust_tg
MYSQL_USER=pythorust_tg
MYSQL_PASSWORD=your_mysql_password
```

Полный список переменных (включая `DEFAULT_CHAT`, `MEDIA_REACTION_LIMIT`, `CHAT_ANALYZER_*` и т.д.) и подсказки по получению ключей: `ENV_SETUP.md` (короткое резюме — `CONFIGURATION_SUMMARY.md`).

### 3️⃣ Инициализация сессии

```bash
# ⚠️ Выполнить ОДИН раз!
cargo run -- init-session

# Введите код из Telegram
# Создастся файл telegram_session.session
```

> 💡 **Важно:** После инициализации вас выкинет из Telegram на других устройствах. Это нормально!

---

## 🚀 Использование

> CLI теперь полностью на Rust (`cargo run -- ...`). Python-версии базовых команд (read/tg/autoanswer/linear и др.) удалены.

### 📋 Работа с чатами

```bash
# Список всех чатов
cargo run -- list-chats --limit 20

# Экспорт чата в Markdown
cargo run -- read my_chat --limit 3000 --delete-unengaged

# Быстрый экспорт (200 сообщений)
cargo run -- tg my_chat --limit 200

# Экспорт личной переписки
cargo run -- export username --limit 300 --output chat.md
```

### 🤖 AI-автоматизация

```bash
# Запуск умного автоответчика
OPENAI_API_KEY=sk-... cargo run -- auto-answer --model gpt-4o-mini

# Linear бот (создание задач из чата)
cargo run -- linear --title "Bug" --description "Steps to reproduce"

# В чате пишите:
# !linear Название задачи | Описание
```

### 🛒 Специализированные боты (MySQL + GPT)

- **BFL Sales Bot** (`bfl_sales_bot.py`) — AI-продавец массажных кресел Relaxio (воронка продаж, возражения, подбор моделей R5/R7/R9).
- **Credit Expert Bot** (`credit_expert_bot.py`) — тёплый консультант по долгам, цель — получить телефон и назначить звонок.

Подготовка MySQL (минимальный DDL):
```sql
CREATE TABLE bot_users (
  id BIGINT PRIMARY KEY,
  username VARCHAR(255), first_name VARCHAR(255), last_name VARCHAR(255),
  language_code VARCHAR(8), is_premium BOOL DEFAULT FALSE, is_bot BOOL DEFAULT FALSE,
  last_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE bot_sessions (
  id BIGINT AUTO_INCREMENT PRIMARY KEY,
  user_id BIGINT NOT NULL, bot_name VARCHAR(64) NOT NULL, state VARCHAR(32),
  is_active BOOL DEFAULT TRUE,
  session_start TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  session_end TIMESTAMP NULL,
  KEY user_bot_idx (user_id, bot_name, is_active)
);
CREATE TABLE bot_messages (
  id BIGINT AUTO_INCREMENT PRIMARY KEY,
  telegram_message_id BIGINT NOT NULL, user_id BIGINT NOT NULL, bot_name VARCHAR(64) NOT NULL,
  direction ENUM('incoming','outgoing') NOT NULL, message_text TEXT, reply_to_message_id BIGINT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  KEY user_bot_created_idx (user_id, bot_name, created_at)
);
-- A/B эксперименты промптов (создаётся автоматически при старте бота)
CREATE TABLE bot_experiments (
  id BIGINT AUTO_INCREMENT PRIMARY KEY,
  bot_name VARCHAR(64) NOT NULL,
  experiment_name VARCHAR(128) NOT NULL,
  session_id BIGINT NULL,
  user_id BIGINT NOT NULL,
  variant VARCHAR(64) NOT NULL,
  conversion BOOL DEFAULT FALSE,
  conversion_reason VARCHAR(255) NULL,
  conversion_value INT NULL,
  assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  closed_at TIMESTAMP NULL,
  KEY bot_exp_idx (bot_name, experiment_name, variant),
  KEY session_idx (session_id),
  KEY user_idx (user_id)
);
```
Заполните `.env`: `BFL_SALES_BOT_TOKEN` или `CREDIT_EXPERT_BOT_TOKEN` + `MYSQL_HOST`/`MYSQL_PORT`/`MYSQL_DATABASE`/`MYSQL_USER`/`MYSQL_PASSWORD`.

Запуск:
```bash
uv run python bfl_sales_bot.py
uv run python credit_expert_bot.py
```

- **A/B тест промптов для BFL Sales Bot**: варианты `control_consultative`, `fast_close_cta`, `story_social_proof` назначаются на сессию и пишутся в `bot_experiments` (имя эксперимента задаётся `BFL_PROMPT_EXPERIMENT`, дефолт `bfl_prompt_ab`). Отчёт по конверсии:
```bash
python ab_test_report.py --bot-name BFL_sales_bot --experiment bfl_prompt_ab --days 14
```
Причины конверсий детектируются по телефону/интенту и выводятся в отчёте.

### 📊 Аналитика ботов (конверсии / воронка / retention)

```bash
uv run python bot_analytics.py --bots BFL_sales_bot Credit_Expert_Bot --days 30
```

- Собирает метрики из MySQL (`bot_sessions`, `bot_messages`)
- Считает воронку: Start → Engaged (≥1 осмысленный ответ) → Multi-turn (≥2 ответа) → Phone shared (конверсия)
- Retention: D1/D7 по новым пользователям в окне `--days`
- Репорт сохраняется в `analysis_results/bot_analytics_<timestamp>.md` (можно указать `--output`)

### 🧠 AI-анализ чатов (Chat Analyzer)

```bash
uv run python -m chat_analysis.analyzer @channel_name --provider openai --limit 800 --days 30 --output-format both
```

- Использует сессию `TELEGRAM_SESSION_NAME/FILE` и ключи `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `GOOGLE_API_KEY`
- Сохраняет JSON и Markdown в `analysis_results/`
- Кастомный промпт: `--prompt prompts/chat_categorizer.md`

### 🧠 Graph RAG (LightRAG из MySQL, Rust CLI)

```bash
cargo run --bin lightrag -- --index --limit 3000
cargo run --bin lightrag -- --index --query "кто ищет дизайнеров?" --mode hybrid --results 5
```

- Забирает сообщения и реакции из MySQL (`telegram_messages`, `telegram_chats`) и строит граф знаний LightRAG
- Требуются `OPENAI_API_KEY` и MySQL переменные из `.env` (`MYSQL_HOST`, `MYSQL_PORT`, `MYSQL_DATABASE`, `MYSQL_USER`, `MYSQL_PASSWORD`)
- Поддерживает режимы `naive | local | global | hybrid` для запросов (`--mode`)

### 🧹 Очистка

```bash
# Удаление ваших сообщений без реакций
cargo run --bin delete_unanswered --all --limit 500 --hours 1

# Удаление Zoom-ссылок
cargo run -- delete-zoom username --limit 3000

# Массовые реакции (по id/ссылкам или последним сообщениям)
cargo run -- react --chat chat_alias --ids 123 124 125 --emoji "🔥"
cargo run -- react --chat chat_alias --file ids.txt --recent 20 --user-id 123456 --emoji "🔥" --delay-ms 600
# Контекстные лайки по пользователю
cargo run -- like --chat chat_alias --user target_user --emoji "🔥" --limit 200
```

### 🛠 Ops-инструменты (N8N и сервисные боты)

```bash
# Мониторинг N8N + автоперезапуск + алерты в Telegram
uv run python n8n_monitor.py

# Бэкапы N8N (ротация/restore)
uv run python n8n_backup.py backup
uv run python n8n_backup.py list
uv run python n8n_backup.py cleanup
uv run python n8n_backup.py restore --file /srv/backups/n8n/<archive>.tar.gz

# Task Assistant Bot: перезапуск N8N, бэкапы, быстрые статусы
uv run python task_assistant_bot.py

# DevOps AI Assistant: статус/логи/перезапуск + AI-подсказки
uv run python devops_ai_bot.py  # конфиг: devops_bot.yml
```

### 🤝 AI Project Consultant (RAG)

```bash
# Консольный режим
uv run python ai_project_consultant.py --mode interactive

# Telegram-бот (нужен AI_CONSULTANT_BOT_TOKEN)
uv run python ai_project_consultant.py --mode telegram

# База знаний: markdown-файлы в knowledge_base/
```

### 🔌 MCP сервер для IDE/агентов

```bash
uv run python mcp_telegram_server.py
```

- FastMCP сервер с тулзами `list_configured_chats`, `fetch_recent_messages`, `send_message`
- Использует локальный файл сессии (`cargo run -- init-session`) и алиасы чатов из `config.yml`
- Лимиты управляются через `.env`: `MCP_TELEGRAM_LIMIT` (дефолт 50), `MCP_TELEGRAM_MAX_LIMIT` (200), `MCP_TELEGRAM_LOCK_RETRY` (10)

---

### 🛰️ N8N мониторинг и бэкапы

1. Подготовьте `.env`: `N8N_URL`, `N8N_API_KEY`, `N8N_RESTART_COMMAND`, `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`, `BACKUP_DIR`, `RETENTION_DAYS`, `MAX_BACKUPS`.
2. Мониторинг + автоперезапуск: `uv run python n8n_monitor.py` (шлёт алерты в Telegram, перезапускает через `N8N_RESTART_COMMAND` после `MAX_RETRIES` неудач).
3. Ручной бэкап: `uv run python n8n_backup.py backup`; список/очистка — `uv run python n8n_backup.py list` и `uv run python n8n_backup.py cleanup`.
4. Автокрон: добавьте строку `0 2 * * * ${PROJECT_ROOT}/n8n_backup_cron.sh` (скрипт активирует `.venv`, делает backup + cleanup и пишет лог в `/var/log/n8n_backup.log`).

---

## 🦀 Rust версия

### Unified CLI

```bash
# Список чатов
cargo run -- list-chats

# Чтение с удалением неинтересного
cargo run -- read my_chat --delete-unengaged

# Экспорт по username
cargo run -- export username

# AI автоответчик
OPENAI_API_KEY=sk-... cargo run -- auto-answer

# Linear интеграция
LINEAR_API_KEY=... cargo run -- linear --title "Bug" --description "Steps"

# 📰 AI-дайджест чата (резюме за 24 часа)
cargo run -- digest my_chat --hours 24 --model gpt-5.1-mini

# 🚫 Модерация мата (анализ без удаления)
cargo run -- profanity-stats my_chat --limit 1000

# Модерация с удалением (требуются права админа)
cargo run -- moderate my_chat --delete --warn

# 📊 CRM-парсинг (извлечение контактов и сделок)
cargo run -- crm my_chat --limit 100 --export-csv contacts.csv

# 🎯 Охота на пользователей по ключевым словам
cargo run -- hunt --chats chat1,chat2 --keywords "работа,вакансия" --required "python" --exclude "спам" --days 30 --export-csv results.csv
```

### Утилиты

```bash
# HTTP бенчмарк (аналог wrk)
cargo run --bin http_bench -- https://api.example.com -c 100 -d 10
# -c: количество соединений
# -d: длительность в секундах

# Мониторинг сайтов
cargo run --bin site_monitor -- check https://example.com
cargo run --bin site_monitor -- watch --interval 60

# K8s Dashboard
cargo run --bin k8s_dash -- pods default        # Pods в namespace
cargo run --bin k8s_dash -- logs my-pod -n app  # Логи pod
cargo run --bin k8s_dash -- nodes               # Статус nodes
cargo run --bin k8s_dash -- namespaces          # Список namespaces
```

---

## 📂 Структура проекта

```
📦 pythorust_tg/
│
├── 🦀 src/                  # Rust CLI (единственный источник CLI)
│   ├── bin/                 # Отдельные бинарники (read_chat, tg, lightrag, http_bench, ...)
│   ├── commands/            # Логика CLI-команд
│   ├── integrations/        # OpenAI, Gemini, Claude, Ollama
│   └── lightrag/            # 🧠 RAG для знаний
│
├── 🐍 Python сервисы/боты
│   ├── ai_project_consultant.py # 🤝 ИИ-консультант с RAG
│   ├── task_assistant_bot.py    # 🛠️ Бот для N8N/бэкапов/статусов
│   ├── n8n_monitor.py           # 🔍 Health-check + автоперезапуск N8N
│   ├── n8n_backup.py            # 💾 Бэкапы/restore N8N + cron-скрипт
│   ├── telegram_service.py      # MCP сервер/Telegram утилиты
│   └── telegram_session.py      # 🔐 Управление сессиями
│
├── 🔍 chat_analysis/            # AI-анализ и категоризация чатов (Python)
│   ├── analyzer.py
│   ├── fetcher.py
│   └── prompts/chat_categorizer.md
│
├── 📝 prompts/                  # Системные промпты (Markdown)
├── 📝 config.yml                # Настройки чатов
├── 📚 knowledge_base/           # Markdown-база знаний для RAG-консультанта
└── 🔒 .env                      # Секреты (не в git!)
```

---

## ⚡ Матрица возможностей

| Функция | Python | Rust | Статус |
|---------|:------:|:----:|:------:|
| 📖 Чтение чатов | ✅ | ✅ | Готово |
| 📤 Экспорт в Markdown | ✅ | ✅ | Готово |
| 🤖 AI автоответчик | ✅ | ✅ | Готово |
| 📝 Linear интеграция | ✅ | ✅ | Готово |
| 🖼️ Скачивание медиа | ✅ | ✅ | Готово |
| 🗑️ Удаление сообщений | ✅ | ✅ | Готово |
| ❤️ Массовые реакции | ✅ | ✅ | Готово |
| 🌐 Мониторинг сайтов | ❌ | ✅ | Готово |
| ⚡ HTTP бенчмарк | ❌ | ✅ | Готово |
| 🧠 LightRAG | ❌ | ✅ | Готово |
| 🧠 AI-анализ/категоризация | ✅ | ❌ | **NEW** |
| 📰 **Дайджест чата (AI)** | ❌ | ✅ | **NEW** |
| 🚫 **Модерация мата** | ❌ | ✅ | **NEW** |
| 📊 **CRM-парсинг** | ❌ | ✅ | **NEW** |
| 🎯 **Охота на пользователей** | ❌ | ✅ | **NEW** |
| 🩺 **N8N мониторинг/бэкапы** | ✅ | ❌ | **NEW** |
| ☁️ **AWS интеграция** | ✅ | ❌ | **NEW** |
| ☸️ **K8s Dashboard** | ❌ | ✅ | **NEW** |
| 🎁 Gifts API | ❌ | 🔜 | Планируется |
| 📖 Stories API | ❌ | 🔜 | Планируется |

---

## 🔒 Безопасность

```
✅ Единая SQLite сессия — никаких утечек токенов
✅ Блокировка параллельного запуска — защита от конфликтов
✅ Все секреты в .env — ничего не хардкодится
✅ Полный .gitignore — приватные файлы не попадут в репо
✅ Нет персональных данных в коде — safe to publish
```

---

## 💰 Монетизация: Как зарабатывать на этих инструментах

### 🤖 SaaS-сервисы на базе инструментария

| Бизнес-модель | Описание | Потенциал |
|---------------|----------|-----------|
| 💬 **AI-консьерж для бизнеса** | White-label автоответчик для компаний. Клиент платит за подписку, бот отвечает клиентам 24/7 | $50-500/мес за клиента |
| 📊 **Аналитика чатов** | Дашборд с метриками вовлечённости, активности, sentiment-анализом для комьюнити-менеджеров | $30-100/мес |
| 📝 **Telegram → Задачи** | Интеграция с Linear/Jira/Notion — превращайте сообщения в тикеты одной командой | $10-50/мес |
| 🗂️ **Архиватор чатов** | Автоматический экспорт и бэкап истории с поиском и индексацией | $5-20/мес |

### 🛠️ Услуги и консалтинг

| Услуга | Описание | Ставка |
|--------|----------|--------|
| ⚙️ **Настройка автоматизации** | Развёртывание и кастомизация под клиента | $500-2000 разово |
| 🔧 **Кастомная разработка** | Новые фичи, интеграции, боты под заказ | $50-150/час |
| 📈 **Аудит Telegram-каналов** | Анализ вовлечённости, рекомендации по росту | $200-500 за отчёт |
| 🎓 **Обучение** | Курсы/воркшопы по автоматизации Telegram | $100-500 за участника |

### 🚀 Готовые продукты

```
┌─────────────────────────────────────────────────────────────────┐
│  🎯 Ниши с высоким спросом                                      │
├─────────────────────────────────────────────────────────────────┤
│  • Инфобизнес — автоответы на вопросы учеников                  │
│  • E-commerce — поддержка клиентов в Telegram                   │
│  • HR/рекрутинг — первичный скрининг кандидатов                 │
│  • Недвижимость — бот-консультант по объектам                   │
│  • Медицина — запись на приём, напоминания                      │
│  • Event-менеджмент — регистрация, уведомления                  │
└─────────────────────────────────────────────────────────────────┘
```

### 📊 Модели монетизации

#### Подписка (SaaS)
```
Стартер:   $29/мес  — 1 бот, 1000 сообщений/мес
Бизнес:    $99/мес  — 5 ботов, 10000 сообщений/мес
Enterprise: $299/мес — безлимит, приоритетная поддержка
```

#### Pay-per-use
```
$0.01 за обработанное сообщение
$0.10 за AI-ответ (GPT-4)
$1.00 за экспорт чата (до 10000 сообщений)
```

#### Freemium + Upsell
```
🆓 Бесплатно: базовый экспорт, 100 сообщений/день
💎 Premium: AI-ответы, аналитика, интеграции
```

### 💡 Идеи для быстрого старта

1. **Бот записи на консультации**
   - Клиент выбирает слот → бот бронирует → напоминание за час
   - Интеграция с Calendly/Cal.com
   - Монетизация: % от консультации или подписка

2. **Автодайджест для каналов**
   - Ежедневная/недельная сводка самых важных сообщений
   - AI-резюме обсуждений
   - Монетизация: $10-30/мес за канал

3. **Модератор-бот**
   - Автоудаление спама, Zoom-ссылок, рекламы
   - Предупреждения нарушителям
   - Монетизация: $5-15/мес за чат

4. **Lead-генератор**
   - Сбор контактов из чатов в CRM
   - Квалификация лидов по активности
   - Монетизация: $0.10-1.00 за лид

### 🔥 Конкурентные преимущества

```
✅ Rust-производительность — обработка тысяч чатов одновременно
✅ Низкие затраты на инфраструктуру — 12 МБ RAM vs 150 МБ у конкурентов
✅ Поддержка новых API — gifts, stories, premium-реакции
✅ Self-hosted — данные клиента остаются у клиента
✅ Open-source ядро — доверие и кастомизация
```

### 📈 Примерный ROI

| Инвестиции | Потенциальный доход | Срок окупаемости |
|------------|---------------------|------------------|
| VPS $20/мес + время | 10 клиентов × $50 = $500/мес | 1-2 месяца |
| Разработка $2000 | 50 клиентов × $30 = $1500/мес | 2-3 месяца |
| Маркетинг $500/мес | 100+ клиентов = $3000+/мес | 1 месяц |

### Практические способы заработка

- 🤝 **Retainer-поддержка каналов** — берёте обслуживание (автоответы, модерация, дайджесты) на абонентскую плату $100-500/мес за канал
- 🏷️ **White-label для агентств** — ставите готовый стек агентству под их брендом, берёте единовременный сетап + процент со всех их клиентов
- 🧩 **Продажа готовых шаблонов** — Docker-compose/uv-сборки и промпты под конкретные ниши (рекрутинг, e-commerce, инфопродукты) за $50-200
- 🎛️ **Managed AI-операции** — берёте на себя лимиты/ключи, выставляете счёт за обработанные сообщения + наценка 50-150%
- 🔗 **Интеграции за фикс** — быстрая связка Telegram ↔ CRM/Notion/Linear за 1-2 дня работы ($300-1500), поддержка отдельно
- 🎓 **Обучение + сообщество** — микрокурс по автоматизации Telegram + закрытый чат с готовыми скриптами, подписка $15-49/мес

---

## 🛠️ Разработка

### Запуск тестов

```bash
# Python
uv run pytest -v

# Rust (63 теста)
cargo test --lib

# Мутационное тестирование (Rust)
cargo mutants --no-shuffle --jobs 4
```

### Линтинг

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

---

## 📚 Документация

| Документ | Описание |
|----------|----------|
| [CLAUDE.md](CLAUDE.md) | Полная техническая документация |
| [rust_telegram/README.md](rust_telegram/README.md) | Документация Rust версии |
| [ENV_SETUP.md](ENV_SETUP.md) | Настройка и справочник по `.env` |
| [CONFIGURATION_SUMMARY.md](CONFIGURATION_SUMMARY.md) | Быстрая шпаргалка по переменным |
| [DASHA_TOOLS.md](DASHA_TOOLS.md) | Инструкции по N8N мониторингу, бэкапам и ботам |
| [codev/](codev/) | Спецификации и планы развития |

---

## 🤝 Contributing

Это open-source проект! PRs приветствуются 🎉

1. Fork репозитория
2. Создайте feature branch
3. Commit ваши изменения
4. Push в branch
5. Откройте Pull Request

---

<div align="center">

**⭐ Star this repo if you find it useful!**

Made with ❤️ and 🦀

</div>
