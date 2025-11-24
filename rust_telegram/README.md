# Телеграм-ридер (Rust)

Rust-версия ридера и автоответчика для Telegram.

## Сборка

```bash
cd rust_telegram
cargo build --release
```

## Использование

### Единый CLI

```bash
# Чтение чата с удалением неинтересных сообщений
cargo run -- read chat_beta --delete-unengaged

# Простой экспорт чата
cargo run -- tg chat_delta

# Список активных чатов
cargo run -- list-chats

# Экспорт чата по username
cargo run -- export <username>

# Удаление Zoom-ссылок
cargo run -- delete-zoom <username>

# AI авто-ответчик
OPENAI_API_KEY=... cargo run -- auto-answer

# Инициализация сессии (только один раз!)
cargo run -- init-session

# Создание задачи в Linear
LINEAR_API_KEY=... LINEAR_TEAM_KEY=APP cargo run -- linear --title "Исправить баг" --description "Шаги воспроизведения"
# Дополнительно: --project <PROJECT_ID> --priority 0..4 --assignee <USER_ID> --labels label1,label2
# Для локальных тестов можно переопределить эндпоинт: LINEAR_API_URL=http://localhost:8080/graphql

# 📰 AI-дайджест чата (резюме за период)
cargo run -- digest my_chat --hours 24 --model gpt-4o-mini

# 🚫 Статистика мата в чате
cargo run -- profanity-stats my_chat --limit 1000

# 📊 CRM-парсинг (извлечение контактов)
cargo run -- crm my_chat --limit 100 --export-csv contacts.csv

# 🎯 Охота на пользователей по ключевым словам
cargo run -- hunt --chats chat1,chat2 --keywords "работа,вакансия"

# 📈 Статистика чата
cargo run --bin chat_stats my_chat

# 🔍 Поиск сообщений
cargo run --bin search_messages --query "важное" --chat my_chat

# 📤 Отправка сообщения
cargo run --bin send_message --chat my_chat --text "Привет!"

# 👤 Поиск пользователя
cargo run --bin find_user @username

# 📇 Экспорт контактов
cargo run --bin export_contacts --format csv

# ❤️ Лайки сообщений
cargo run --bin like_messages --chat my_chat --emoji "👍"

# 📋 Дайджест сообщений
cargo run --bin message_digest my_chat --days 7
```

### Отдельные бинарники

```bash
# Эквивалент read.py
cargo run --bin read_chat chat_beta

# Эквивалент tg.py
cargo run --bin tg chat_delta

# Эквивалент list_chats.py
cargo run --bin list_chats

# Эквивалент get_active_chats.py
cargo run --bin get_active_chats

# Эквивалент export_chat.py
cargo run --bin export_chat <username>

# Эквивалент delete_zoom_messages.py
cargo run --bin delete_zoom_messages <username>

# Эквивалент autoanswer.py
OPENAI_API_KEY=... cargo run --bin autoanswer

# Эквивалент init_session.py
cargo run --bin init_session
```

## Структура проекта

```
rust_telegram/
├── Cargo.toml              # Зависимости и конфигурация
├── src/
│   ├── lib.rs              # Основная библиотека
│   ├── main.rs             # Единый CLI
│   ├── config.rs           # Конфигурация (API ключи, чаты)
│   ├── error.rs            # Типы ошибок
│   ├── session.rs          # Управление сессиями
│   ├── chat.rs             # Операции с чатами
│   ├── reactions.rs        # Обработка реакций
│   ├── export.rs           # Экспорт в файлы
│   ├── commands/           # Реализации команд
│   │   ├── mod.rs
│   │   ├── read.rs
│   │   ├── tg.rs
│   │   ├── list_chats.rs
│   │   ├── active_chats.rs
│   │   ├── export.rs
│   │   ├── delete_zoom.rs
│   │   ├── autoanswer.rs
│   │   └── init_session.rs
│   └── bin/                # Отдельные бинарники
│       ├── read_chat.rs
│       ├── tg.rs
│       ├── list_chats.rs
│       ├── get_active_chats.rs
│       ├── export_chat.rs
│       ├── delete_zoom_messages.rs
│       ├── autoanswer.rs
│       └── init_session.rs
```

## Соответствие Python → Rust

| Python | CLI на Rust | Отдельный бинарь |
|--------|----------|-------------|
| `python read.py chat_beta` | `cargo run -- read chat_beta -d` | `cargo run --bin read_chat chat_beta` |
| `python tg.py chat_delta` | `cargo run -- tg chat_delta` | `cargo run --bin tg chat_delta` |
| `python list_chats.py` | `cargo run -- list-chats` | `cargo run --bin list_chats` |
| `python get_active_chats.py` | `cargo run -- active-chats` | `cargo run --bin get_active_chats` |
| `python export_chat.py <username>` | `cargo run -- export <username>` | `cargo run --bin export_chat <username>` |
| `python delete_zoom_messages.py` | `cargo run -- delete-zoom` | `cargo run --bin delete_zoom_messages` |
| `python autoanswer.py` | `cargo run -- auto-answer` | `cargo run --bin autoanswer` |
| `python init_session.py` | `cargo run -- init-session` | `cargo run --bin init_session` |
| - | `cargo run -- digest` | `cargo run --bin message_digest` |
| - | `cargo run -- linear` | `cargo run --bin linear_bot` |
| - | `cargo run -- crm` | `cargo run --bin crm` |
| - | `cargo run -- hunt` | `cargo run --bin hunt` |

## Новые утилиты

| Бинарник | Описание |
|----------|----------|
| `chat_stats` | Статистика чата (сообщений, участников, активность) |
| `search_messages` | Поиск сообщений по ключевым словам |
| `send_message` | Отправка сообщений в чат |
| `find_user` | Поиск пользователя по username |
| `export_contacts` | Экспорт контактов в CSV/JSON |
| `like_messages` | Простановка реакций на сообщения |
| `message_digest` | AI-резюме сообщений за период |
| `index_messages` | Индексация сообщений в Qdrant |
| `site_monitor` | Мониторинг сайтов |
| `http_bench` | HTTP бенчмаркинг |
| `delete_unanswered` | Удаление сообщений без ответов |

## Зависимости

- **grammers** - Telegram MTProto клиент на чистом Rust
- **tokio** - Асинхронный runtime
- **clap** - Парсинг CLI аргументов
- **async-openai** - OpenAI API клиент
- **chrono** - Работа с датами
- **serde** - Сериализация
- **qdrant-client** - Векторная БД для поиска
- **neo4rs** - Neo4j граф-БД
- **reqwest** - HTTP клиент
- **regex** - Регулярные выражения

## CI/CD

GitHub Actions автоматически запускает:
- `cargo fmt --check` - проверка форматирования
- `cargo clippy` - линтинг
- `cargo test` - тесты
- `cargo tarpaulin` - покрытие кода
- `cargo audit` - аудит безопасности

## Безопасность

⚠️ **Важно:**
- Session файл содержит токены авторизации - храните в безопасности
- API ключи должны храниться в переменных окружения
- Не коммитьте session файлы в git
