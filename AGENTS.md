# Telegram Chat Reader & Auto-responder

Бэкенд: весь новый функционал и поддерживаемые сервисы пишем на Rust. Python оставляем только для существующих legacy-скриптов/ops до миграции.

## 📋 Обзор проекта

Этот проект предоставляет инструменты для:
- Чтения и экспорта сообщений Telegram в markdown файлы
- Отслеживания реакций и метрик вовлечённости
- Автоматических ответов с помощью AI (интеграция OpenAI)
- Управления несколькими чатами и сессиями
- Мониторинга и бэкапов N8N + сервисных ботов

## 🚀 Возможности

### Чтение чатов (Rust CLI `read`)
- Экспорт истории чата в markdown (до 3000 сообщений)
- Отслеживание реакций и вовлечённости
- Скачивание медиа из популярных сообщений (>100k реакций)
- Автоудаление сообщений с низкой вовлечённостью
- Поддержка личных чатов и каналов

### Автоответчик (Rust CLI `auto-answer`)
- AI-ответы через OpenAI API
- Мониторинг сообщений в реальном времени
- Настраиваемые системные инструкции для AI
- Управление сессиями через Telethon

### Простой экспорт (Rust CLI `tg`)
- Упрощённый экспорт чата
- Настраиваемый лимит сообщений (по умолчанию: 200)
- Скачивание медиа для сообщений с 1000+ реакций
- Отображение реакций и эмодзи

### Дополнительные утилиты
- 🤖 **AI-консультант с RAG** (`ai_project_consultant.py`) — интерактивный режим и Telegram-бот, ищет ответы в `knowledge_base/`
- 🛠️ **Task Assistant Bot** (`task_assistant_bot.py`) — управление N8N, бэкапами и быстрые команды
- 🔍 **N8N Monitor** (`n8n_monitor.py`, `n8n_monitor.service`) — health-check и автоперезапуск
- 💾 **N8N Backup** (`n8n_backup.py`, `n8n_backup_cron.sh`) — бэкапы и ротация
- 🛒 **BFL Sales Bot** (`bfl_sales_bot.py`) — AI-продавец массажных кресел (сохранение сессий в MySQL)
- 🤝 **Credit Expert Bot** (`credit_expert_bot.py`) — тёплый консультант по долгам (MySQL хранилище диалогов)

Подробности: `DASHA_TOOLS.md`.

## 📦 Зависимости

```
telethon
openai
aiohttp
requests
pytest
python-dotenv
behave
```

Установка:
```bash
pip install -r requirements.txt
```

## 🔧 Настройка

### 1. Получение API данных Telegram

1. Перейдите на https://my.telegram.org/
2. Войдите в свой аккаунт Telegram
3. Откройте "API Development Tools"
4. Создайте новое приложение
5. Сохраните `API_ID` и `API_HASH`

### 2. Настройка окружения (.env)

Скопируйте шаблон и заполните переменные:
```bash
cp .env.example .env
nano .env  # TELEGRAM_API_ID, TELEGRAM_API_HASH, TELEGRAM_PHONE, токены ботов, ключи AI
```

Минимум:
- `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`, `TELEGRAM_PHONE`
- `OPENAI_API_KEY` и модель (`OPENAI_MODEL` или `AI_CONSULTANT_MODEL`)
- `TELEGRAM_BOT_TOKEN` и `TELEGRAM_CHAT_ID` — для алертов из `n8n_monitor.py`
- `TASK_ASSISTANT_BOT_TOKEN` или `AI_CONSULTANT_BOT_TOKEN` — если запускаете ботов

Все ключевые конфигурации берутся из `.env` (Telegram/AI/N8N обязательны, остальное опционально). Полный список переменных: `ENV_SETUP.md`.

### 3. Создайте сессию (один раз)

```bash
cargo run -- init-session
```

CLI использует значения из `.env`, запросит код из Telegram и создаст файл сессии (`{TELEGRAM_SESSION_NAME}.session`).

## 💻 Использование

### Чтение истории чата (Rust)

```bash
cargo run -- read chat_alpha --limit 3000 --delete-unengaged
```

Чаты настраиваются в `config.yml`. Можно задать `DEFAULT_CHAT` в `.env`, тогда аргумент необязателен.

### Простой экспорт (Rust)

```bash
cargo run -- tg chat_alpha --limit 200
```

### Запуск автоответчика (Rust)

```bash
OPENAI_API_KEY=sk-... cargo run -- auto-answer --model gpt-4o-mini
```

### AI-анализ чатов (chat_analysis)

```bash
python -m chat_analysis.analyzer @channel_name --provider openai --limit 800 --days 30 --output-format both
```

Сессия берётся из `TELEGRAM_SESSION_NAME/FILE`, ключи LLM — из `.env`. Итоги сохраняются в `analysis_results/` (JSON + Markdown). Можно указать свой промпт через `--prompt prompts/chat_categorizer.md`.

### Мониторинг и бэкапы N8N

```bash
# Проверка и автоперезапуск N8N
python n8n_monitor.py

# Бэкап/листинг/cleanup/restore
python n8n_backup.py backup
python n8n_backup.py list
python n8n_backup.py cleanup
python n8n_backup.py restore --file /srv/backups/n8n/<archive>.tar.gz
```

### Task Assistant Bot

```bash
python task_assistant_bot.py
```

### AI Project Consultant

```bash
# Консольный режим
python ai_project_consultant.py --mode interactive

# Telegram-бот (нужен AI_CONSULTANT_BOT_TOKEN)
python ai_project_consultant.py --mode telegram
```

### Специализированные боты (MySQL)

- **BFL Sales Bot** (`bfl_sales_bot.py`) — AI-продавец массажных кресел (воронка продаж, работа с возражениями).
- **Credit Expert Bot** (`credit_expert_bot.py`) — консультант по долгам с тёплыми скриптами и целевым сбором телефона.

Требуется MySQL с таблицами `bot_users`, `bot_sessions`, `bot_messages` (DDL в README). Добавьте в `.env`: `BFL_SALES_BOT_TOKEN` или `CREDIT_EXPERT_BOT_TOKEN` и `MYSQL_HOST`/`MYSQL_PORT`/`MYSQL_DATABASE`/`MYSQL_USER`/`MYSQL_PASSWORD`.

Запуск:
```bash
python bfl_sales_bot.py
python credit_expert_bot.py
```

## 📝 Конфигурация чатов

Чаты настраиваются в `config.yml`:

```yaml
chats:
  example_channel:
    type: channel
    id: 1234567890
  example_user:
    type: username
    username: example_name
```

## 🎯 Ключевые функции

### Отслеживание реакций
- Подсчёт всех реакций на сообщение
- Извлечение эмодзи-реакций
- Фильтрация по уровню вовлечённости

### Обработка медиа
- Скачивание медиа из популярных сообщений
- Сохранение в директории чата
- Возможность пропуска медиа в выводе

### Фильтрация сообщений
- Автоудаление сообщений без реакций (настраивается)
- Пропуск ответов на другие сообщения
- Удаление определённых паттернов (например, Zoom ссылок)

### AI интеграция
- Интеграция с OpenAI GPT моделями
- Настраиваемые системные инструкции
- Генерация ответов в реальном времени

## ⚙️ Поддержка окружений

Определение GitHub Actions и адаптация поведения:
- Уменьшение лимита до 1000 сообщений в CI/CD
- Пропуск скачивания медиа в автоматизированных окружениях

## 🔐 Безопасность

- Файлы сессий содержат токены авторизации - храните их приватно
- API ключи должны храниться безопасно
- OpenAI API ключи должны использовать переменные окружения в продакшене

## 📊 Формат вывода

Экспорт чатов сохраняется как markdown файлы:
```
[timestamp] [sender_name]: [message_text] [reactions] [media_path]
```

Пример:
```
01.10.2025 12:30:45 UserA: Привет всем! 🔥❤️👍
UserB: Отличный пост! 🎉
```

## 🛠 Разработка

### Тестирование
```bash
pytest
```

### Создание виртуального окружения (Windows)
```bash
create_venv.cmd
```

## 📌 Примечания

- Лимит сообщений по умолчанию: 3000 (1000 в GitHub Actions)
- Порог скачивания медиа: 100,000 реакций
- Файлы сессий переиспользуются между запусками
- Поддержка личных чатов и каналов

## 🤝 Участие в разработке

Это персональный проект автоматизации. Используйте как референс для своих задач автоматизации Telegram.

## ⚖️ Лицензия

Персональный проект - используйте на своё усмотрение. Соблюдайте ToS Telegram.

---

## Методология Codev

Проект использует методологию разработки Codev (context-driven development).

### Активный протокол
- Протокол: SPIDER-SOLO (вариант для одного разработчика)
- Расположение: codev/protocols/spider-solo/protocol.md

### Структура директорий
- Спецификации: codev/specs/
- Планы: codev/plans/
- Ревью: codev/reviews/
- Ресурсы: codev/resources/

### Агенты рабочего процесса
Доступны в `.claude/agents/`:
- `spider-protocol-updater` - Анализ реализаций SPIDER и рекомендации по улучшению
- `architecture-documenter` - Поддержка документации архитектуры
- `codev-updater` - Обновление установки Codev до последней версии

См. codev/protocols/spider-solo/protocol.md для полной документации протокола.
