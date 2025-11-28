# Telegram Automation Toolkit (Rust)

**ВАЖНО: Весь код должен быть написан на Rust. Python только для legacy-скриптов, которые нужно переписать.**

## Правила разработки

1. **Только Rust** - все новые функции писать на Rust
2. **Переписать Python** - постепенно мигрировать весь Python на Rust
3. **Максимальные оптимизации** - использовать `--release` и LTO
4. **Zero-cost abstractions** - никаких лишних аллокаций
5. **Async/await** - использовать tokio для асинхронности

## Rust Best Practices

### Инструменты качества кода

```bash
# Форматирование (обязательно перед коммитом)
cargo fmt

# Линтер (400+ проверок)
cargo clippy -- -W clippy::pedantic

# Автоисправление
cargo clippy --fix

# Проверка безопасности зависимостей
cargo audit
```

### Оптимизация производительности

```toml
# Cargo.toml - профили сборки
[profile.release]
opt-level = 3
lto = "fat"           # Full LTO для максимальной оптимизации
codegen-units = 1     # Одна единица компиляции = лучшая оптимизация
panic = "abort"       # Без раскрутки стека
strip = true          # Убрать отладочную информацию

[profile.dev]
opt-level = 0
incremental = true    # Быстрая инкрементальная сборка

[profile.dev.package."*"]
opt-level = 2         # Оптимизировать зависимости даже в dev
```

### Паттерны идиоматичного Rust

```rust
// Используй итераторы вместо циклов
let sum: i32 = vec.iter().filter(|x| **x > 0).sum();

// Result и Option вместо паники
fn process() -> Result<Data, Error> {
    let data = fetch_data()?;
    Ok(transform(data))
}

// Избегай clone() где возможно - используй ссылки
fn process(data: &Data) -> &str { ... }

// const для compile-time констант
const MAX_RETRIES: u32 = 3;

// Используй #[derive] для стандартных трейтов
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config { ... }
```

### Безопасность

- **Избегай `unsafe`** - только в крайних случаях с документацией
- **Валидируй входные данные** на границах системы
- **Используй `secrecy`** для чувствительных данных
- **Никогда не коммить секреты** в git

## Бинарник CLI

```bash
# Сборка
cargo build --release

# Бинарник
./target/release/telegram_reader --help
```

## Доступные команды

| Команда | Описание | Пример |
|---------|----------|--------|
| `read` | Экспорт чата в markdown | `telegram_reader read "Хара"` |
| `tg` | Простой экспорт | `telegram_reader tg chat_name` |
| `list-chats` | Список чатов | `telegram_reader list-chats` |
| `active-chats` | Активные чаты | `telegram_reader active-chats` |
| `like` | Лайки сообщений | `telegram_reader like -c "Chat" -u "User"` |
| `delete-zoom` | Удаление Zoom-ссылок | `telegram_reader delete-zoom @channel` |
| `auto-answer` | AI автоответчик | `telegram_reader auto-answer` |
| `init-session` | Инициализация сессии | `telegram_reader init-session` |
| `linear` | Создать задачу Linear | `telegram_reader linear -t "Title"` |
| `digest` | AI дайджест чата | `telegram_reader digest "Chat" -H 24` |
| `moderate` | Модерация мата | `telegram_reader moderate "Chat"` |
| `crm` | Парсинг CRM данных | `telegram_reader crm "Chat"` |
| `hunt` | Поиск пользователей | `telegram_reader hunt -c "Chat" -k "keyword"` |
| `n8n-monitor` | Мониторинг N8N | `telegram_reader n8n-monitor` |
| `n8n-backup` | Бэкап N8N | `telegram_reader n8n-backup backup` |

## Структура проекта

```
/srv/pythorust_tg/
├── Cargo.toml              # Rust зависимости
├── src/
│   ├── main.rs             # CLI точка входа
│   ├── lib.rs              # Библиотека
│   ├── config.rs           # Загрузка config.yml
│   ├── session.rs          # Telegram сессия (Grammers)
│   ├── error.rs            # Типы ошибок
│   ├── commands/           # CLI команды
│   │   ├── mod.rs
│   │   ├── read.rs         # Чтение чатов
│   │   ├── tg.rs           # Экспорт
│   │   ├── like.rs         # Лайки
│   │   ├── digest.rs       # AI дайджест
│   │   ├── moderate.rs     # Модерация
│   │   ├── crm.rs          # CRM парсинг
│   │   ├── hunt.rs         # Поиск пользователей
│   │   ├── n8n.rs          # N8N мониторинг/бэкап
│   │   └── ...
│   ├── integrations/       # AI провайдеры
│   │   ├── openai.rs
│   │   ├── gemini.rs
│   │   ├── claude.rs
│   │   └── ollama.rs
│   └── analysis/           # Анализ данных
│       ├── vector_db.rs    # Qdrant
│       └── graph_db.rs     # Neo4j
├── prompts/                # Системные промпты
└── config.yml              # Конфигурация чатов
```

## Rust исходники

| Файл | Описание |
|------|----------|
| [src/main.rs](src/main.rs) | CLI с clap |
| [src/lib.rs](src/lib.rs) | Публичное API |
| [src/config.rs](src/config.rs) | Загрузка YAML конфига |
| [src/session.rs](src/session.rs) | Grammers сессия |
| [src/commands/read.rs](src/commands/read.rs) | Чтение чатов |
| [src/commands/tg.rs](src/commands/tg.rs) | Экспорт |
| [src/commands/like.rs](src/commands/like.rs) | Реакции |
| [src/commands/digest.rs](src/commands/digest.rs) | AI дайджест |
| [src/commands/moderate.rs](src/commands/moderate.rs) | Модерация |
| [src/commands/crm.rs](src/commands/crm.rs) | CRM парсинг |
| [src/commands/hunt.rs](src/commands/hunt.rs) | Поиск пользователей |
| [src/commands/n8n.rs](src/commands/n8n.rs) | N8N мониторинг/бэкап |
| [src/commands/linear.rs](src/commands/linear.rs) | Linear интеграция |
| [src/integrations/openai.rs](src/integrations/openai.rs) | OpenAI API |
| [src/integrations/gemini.rs](src/integrations/gemini.rs) | Google Gemini |
| [src/prompts.rs](src/prompts.rs) | Загрузка промптов |

## Ключевые зависимости (Cargo.toml)

```toml
grammers-client = "0.8"     # Telegram MTProto
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
async-openai = "0.28"       # OpenAI API
reqwest = "0.12"            # HTTP клиент
serde = "1.0"               # Сериализация
tracing = "0.1"             # Логирование
```

## Настройка

### 1. Переменные окружения (.env)

```bash
# Telegram
TELEGRAM_API_ID=12345
TELEGRAM_API_HASH=abc123
TELEGRAM_PHONE=+79001234567
MY_ID=7098803

# AI
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-4o-mini

# N8N
N8N_URL=https://n8n.example.com
N8N_API_KEY=...
N8N_RESTART_COMMAND="systemctl restart n8n"
BACKUP_DIR=/srv/backups/n8n
```

### 2. Инициализация сессии

```bash
# Rust использует отдельный файл сессии (Grammers)
./target/release/telegram_reader init-session
# Введите YES и код из Telegram
```

### 3. Конфигурация чатов (config.yml)

```yaml
chats:
  Хара:
    type: channel
    id: 2325284588
  my_chat:
    type: username
    username: example_chat
```

## Python (legacy) - НЕ ИСПОЛЬЗОВАТЬ ДЛЯ НОВОГО КОДА

Эти скрипты нужно переписать на Rust:

| Python | Статус миграции |
|--------|-----------------|
| `n8n_monitor.py` | ✅ Готово → `n8n-monitor` |
| `n8n_backup.py` | ✅ Готово → `n8n-backup` |
| `chat_analyzer.py` | ❌ Нужно мигрировать |
| `mcp_telegram_server.py` | ❌ Нужно мигрировать |
| `ai_project_consultant.py` | ❌ Нужно мигрировать |
| `bfl_sales_bot.py` | ❌ Нужно мигрировать |
| `credit_expert_bot.py` | ❌ Нужно мигрировать |

## Сборка и запуск

```bash
# Быстрая сборка (dev)
cargo build

# Оптимизированная сборка
cargo build --release

# Максимальная оптимизация
cargo build --profile release-optimized

# Запуск с логами
RUST_LOG=info ./target/release/telegram_reader list-chats

# Тесты
cargo test

# Бенчмарки
cargo bench
```

## CI/CD

```bash
# Проверки перед коммитом
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

---

**Приоритет: производительность и безопасность. Rust позволяет достичь скорости C++ с гарантиями безопасности памяти.**
