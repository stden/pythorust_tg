# Python → Rust Migration Plan

**Дата**: 2025-11-25
**Цель**: Переписать все Python скрипты на Rust для:
- Повышения производительности
- Улучшения type safety
- Уменьшения зависимостей
- Единой кодовой базы

---

## 📊 Инвентаризация Python скриптов

### Всего Python файлов: 32

#### Категория 1: Telegram API (HIGH PRIORITY)
| Файл | Размер | Rust эквивалент | Статус |
|------|--------|-----------------|--------|
| read.py | ~500 строк | ✅ rust_telegram/src/commands/read.rs | Done |
| tg.py | ~200 строк | ✅ rust_telegram/src/commands/tg.rs | Done |
| list_chats.py | ~100 строк | ✅ rust_telegram/src/commands/list_chats.rs | Done |
| init_session.py | ~80 строк | ✅ rust_telegram/src/commands/init_session.rs | Done |
| send_viral_question.py | ~80 строк | ✅ rust_telegram/src/bin/send_viral_questions.rs | Done |
| export_any_chat.py | ~150 строк | ✅ rust_telegram/src/bin/export_any_chat.rs | Done |
| export_chat.py | ~200 строк | ⏳ rust_telegram/src/commands/export.rs | Needs update |
| download_chat.py | ~150 строк | ⏳ TODO | Pending |
| download_user_chat.py | ~150 строк | ⏳ TODO | Pending |
| find_user.py | ~50 строк | ⏳ TODO | Pending |
| get_active_chats.py | ~100 строк | ⏳ TODO | Pending |
| like_messages.py | ~100 строк | ⏳ TODO | Pending |
| delete_zoom_messages.py | ~150 строк | ✅ rust_telegram/src/commands/delete_zoom.rs | Done |
| delete_unanswered.py | ~100 строк | ⏳ TODO | Pending |

#### Категория 2: AI & Бизнес-логика (MEDIUM PRIORITY)
| Файл | Размер | Rust эквивалент | Статус |
|------|--------|-----------------|--------|
| autoanswer.py | ~300 строк | ⏳ rust_telegram/src/commands/autoanswer.rs | Needs completion |
| autoanswer_refactored.py | ~350 строк | ⏳ TODO | Pending |
| chat_analyzer.py | ~400 строк | ⏳ TODO | Pending |
| collect_chat_ideas.py | ~200 строк | ⏳ TODO | Pending |
| check_all_chats_tasks.py | ~250 строк | ⏳ TODO | Pending |
| ai_project_consultant.py | ~500 строк | ⏳ TODO | Pending |
| ai_service.py | ~300 строк | ⏳ TODO | Pending |

#### Категория 3: Linear Integration (MEDIUM PRIORITY)
| Файл | Размер | Rust эквивалент | Статус |
|------|--------|-----------------|--------|
| linear_client.py | ~200 строк | ⏳ TODO | Pending |
| linear_bot.py | ~350 строк | ⏳ TODO | Pending |
| create_linear_tasks.py | ~150 строк | ⏳ TODO | Pending |
| sync_linear_tasks.py | ~200 строк | ⏳ TODO | Pending |

#### Категория 4: Telegram Bots (LOW PRIORITY)
| Файл | Размер | Rust эквивалент | Статус |
|------|--------|-----------------|--------|
| telegram_bot_base.py | ~400 строк | ⏳ TODO | Pending |
| task_assistant_bot.py | ~500 строк | ⏳ TODO | Pending |
| test_doroga_bot.py | ~200 строк | ⏳ TODO | Pending |
| mcp_telegram_server.py | ~600 строк | ⏳ TODO | Pending |

#### Категория 5: Утилиты (SHARED)
| Файл | Размер | Rust эквивалент | Статус |
|------|--------|-----------------|--------|
| telegram_session.py | ~200 строк | ✅ rust_telegram/src/session.rs | Done |
| telegram_service.py | ~300 строк | ⏳ TODO | Pending |
| chat_export_utils.py | ~500 строк | ⏳ TODO | Pending |
| n8n_backup.py | ~150 строк | ⏳ TODO | Pending |
| n8n_monitor.py | ~100 строк | ⏳ TODO | Pending |

---

## 🎯 Приоритизация миграции

### Phase 1: Core Telegram Commands (1-2 недели)
**Цель**: Заменить все базовые Telegram операции

1. ✅ **read.rs** (Done)
2. ✅ **list_chats.rs** (Done)
3. ✅ **send_message.rs** (Done)
4. ⏳ **export_any_chat.rs** (New)
5. ⏳ **download_chat.rs** (New)
6. ⏳ **find_user.rs** (New)
7. ⏳ **get_active_chats.rs** (New)
8. ⏳ **like_messages.rs** (New)
9. ⏳ **delete_unanswered.rs** (New)

**Expected result**: Все Telegram CLI команды в Rust, Python deprecated.

---

### Phase 2: AI Integration (2-3 недели)
**Цель**: AI-powered функциональность на Rust

1. ⏳ **autoanswer.rs** — полная реализация
   - OpenAI/Anthropic API клиент
   - Telegram integration
   - Async processing

2. ⏳ **chat_analyzer.rs** — анализ чатов с AI
   - Monetization opportunities detector
   - Viral questions generator
   - Sentiment analysis

3. ⏳ **ai_service.rs** — общий AI сервис
   - Claude/GPT/Gemini abstractions
   - Prompt management
   - Token counting

**Dependencies**:
- `reqwest` для HTTP
- `serde_json` для JSON
- `tokio` для async

---

### Phase 3: Linear Integration (1-2 недели)
**Цель**: GraphQL клиент для Linear на Rust

1. ⏳ **linear_client.rs**
   - GraphQL queries/mutations
   - Issue creation
   - Team management

2. ⏳ **linear_bot.rs**
   - Telegram → Linear bridge
   - Command parsing
   - Issue sync

**Dependencies**:
- `graphql-client` или `cynic`
- `reqwest`

---

### Phase 4: Telegram Bots (3-4 недели)
**Цель**: Полноценные боты на Rust

1. ⏳ **task_assistant_bot**
   - aiogram → teloxide migration
   - State management
   - Database integration

2. ⏳ **mcp_telegram_server**
   - MCP protocol implementation
   - Server/client architecture

**Dependencies**:
- `teloxide` (Telegram bot framework)
- `sqlx` для DB
- `tower` для MCP server

---

### Phase 5: Utilities (1 неделя)
**Цель**: Вспомогательные утилиты

1. ⏳ **chat_export_utils.rs**
   - Markdown formatting
   - Timestamp parsing
   - Sender resolution

2. ⏳ **n8n_backup.rs**
   - Workflow backup
   - API integration

---

## 📦 Новые Rust зависимости

### Уже используются:
```toml
[dependencies]
grammers-client = "0.8"  # Telegram MTProto
tokio = "1.0"            # Async runtime
serde = "1.0"            # Serialization
serde_json = "1.0"       # JSON
dotenv = "0.15"          # .env loading
```

### Нужно добавить:
```toml
# AI/HTTP
reqwest = { version = "0.11", features = ["json"] }
async-openai = "0.26"  # OpenAI API

# GraphQL (Linear)
graphql-client = "0.15"

# Telegram bots
teloxide = { version = "0.13", features = ["macros"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }

# Utilities
chrono = "0.4"  # DateTime
regex = "1.0"   # Regex
```

---

## 🚀 Roadmap

### Неделя 1-2: Core Telegram (Phase 1)
- [x] Создать `send_viral_questions.rs`
- [ ] Реализовать `export_any_chat.rs`
- [ ] Реализовать `download_chat.rs`
- [ ] Реализовать `find_user.rs`
- [ ] Реализовать `get_active_chats.rs`
- [ ] Реализовать `like_messages.rs`
- [ ] Реализовать `delete_unanswered.rs`

### Неделя 3-5: AI Integration (Phase 2)
- [ ] Закончить `autoanswer.rs`
- [ ] Создать `chat_analyzer.rs`
- [ ] Создать `ai_service.rs`
- [ ] Интеграция с OpenAI/Claude API

### Неделя 6-7: Linear (Phase 3)
- [ ] `linear_client.rs` с GraphQL
- [ ] `linear_bot.rs` Telegram integration

### Неделя 8-11: Bots (Phase 4)
- [ ] `task_assistant_bot` на teloxide
- [ ] `mcp_telegram_server` на tower

### Неделя 12: Finalization
- [ ] Удалить все `.py` файлы
- [ ] Обновить документацию
- [ ] Release 1.0.0

---

## ✅ Критерии успеха

1. **100% функциональность**:все Python скрипты работают как Rust бинарники

2. **Performance**:
   - 2-5x faster execution
   - 50% меньше memory usage

3. **Developer Experience**:
   - Единый `cargo run --bin <command>`
   - Type-safe всё
   - Лучшие error messages

4. **Deployment**:
   - Single binary releases (no Python deps)
   - Кросс-платформенная сборка

---

## 🔥 Quick Wins

### Что мигрировать ПЕРВЫМ для максимальной пользы:

1. **autoanswer.py → autoanswer.rs** (⏱️ 3-4 дня)
   - **Польза**: CPU-intensive, async-heavy → идеально для Rust
   - **Impact**: HIGH (используется каждый день)

2. **chat_analyzer.py → chat_analyzer.rs** (⏱️ 2-3 дня)
   - **Польза**: Обработка больших файлов (100K+ lines)
   - **Impact**: HIGH (нужен для monetization)

3. **linear_client.py → linear_client.rs** (⏱️ 2-3 дня)
   - **Польза**: GraphQL типы в compile-time
   - **Impact**: MEDIUM

---

## 💡 Best Practices

### 1. Incremental Migration
- Не переписывать всё сразу
- Начать с CLI команд (проще)
- Потом боты (сложнее)

### 2. Keep Python for prototyping
- Новые фичи — сначала Python (быстрый прототип)
- Потом Rust (production-ready)

### 3. Share code via FFI (если нужно)
- Python может вызывать Rust через PyO3
- Но лучше просто вызывать Rust бинарники

---

## 📝 Testing Strategy

### Unit tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_to_chat() {
        // Test with mock Telegram client
    }
}
```

### Integration tests
```bash
cargo test --test integration_tests
```

### E2E tests
```bash
# Run against real Telegram (test account)
cargo run --bin send_message -- @test_user "Hello"
```

---

## 🎓 Resources

- [Grammers Docs](https://github.com/Lonami/grammers)
- [Teloxide Book](https://docs.rs/teloxide)
- [Async Rust Book](https://rust-lang.github.io/async-book/)
- [GraphQL Client Guide](https://github.com/graphql-rust/graphql-client)

---

**Next Steps**: Start with Phase 1, Week 1 tasks.

---

**Created**: 2025-11-25
**Last Updated**: 2025-11-25
**Status**: 🚧 In Progress
