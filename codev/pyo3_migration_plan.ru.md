# План миграции Python → Rust (PyO3)

## Обзор

Этот документ описывает функции и модули Python, которые выгодно перенести на Rust
с использованием PyO3 для повышения производительности.

> Статус на 2026-06: документ исторический. Часть Python-файлов ниже уже
> удалена или перенесена в Rust; актуальный инвентарь миграции см.
> `codev/plans/python-to-rust-migration.md`.

## Критерии выбора

- CPU-интенсивные операции (парсинг, регулярные выражения, обработка текста)
- Горячие пути (функции, вызываемые для каждого сообщения)
- Операции агрегации данных (циклы, множества, словари)
- Параллелизуемые задачи

**Не переносим:**
- Асинхронный I/O (HTTP, Telegram API, OpenAI)
- Тонкие обёртки над внешними API
- Конфигурационный код

---

## Фаза 1: Быстрые победы (LOW complexity)

### 1.1 `chat_export_utils.py` — Парсинг реакций

| Функция | Описание | Причина миграции | Ускорение |
|---------|----------|------------------|-----------|
| `_parse_reactions()` | Извлечение emoji и счётчиков | Горячий путь, вызов на каждое сообщение | 5-10x |
| `sanitize_filename()` | Очистка имени файла regex | Множественные `re.sub()` | 5-10x |
| `collect_reactions_summary()` | Сводка реакций по чату | Итерация + join | 5x |
| `build_message_text()` | Форматирование текста сообщения | Строковые операции | 5x |

**Интерфейс Rust:**
```rust
#[pyfunction]
fn parse_reactions(reactions: &PyAny) -> PyResult<HashMap<String, i32>>;

#[pyfunction]
fn sanitize_filename(name: &str, max_length: usize) -> String;

#[pyfunction]
fn collect_reactions_summary(messages: Vec<PyObject>) -> String;
```

### 1.2 `ab_testing.py` — Детекция паттернов

| Функция | Описание | Причина миграции | Ускорение |
|---------|----------|------------------|-----------|
| `contains_phone()` | Поиск телефона regex | Компиляция regex на каждый вызов | 15-30x |
| `detect_conversion()` | Поиск ключевых слов | Множественные `in` проверки | 10-20x |
| `is_meaningful_user_message()` | Фильтрация команд | Строковые проверки | 5x |

**Интерфейс Rust:**
```rust
#[pyfunction]
fn contains_phone(text: &str) -> bool;

#[pyfunction]
fn detect_conversion(text: &str) -> Option<String>;

#[pyfunction]
fn is_meaningful_user_message(text: &str) -> bool;
```

### 1.3 `export_linear_to_mysql.py` — Парсинг дат

| Функция | Описание | Причина миграции | Ускорение |
|---------|----------|------------------|-----------|
| `parse_iso_datetime()` | ISO 8601 → datetime | Вызов на каждую задачу | 20-50x |

**Интерфейс Rust:**
```rust
#[pyfunction]
fn parse_iso_datetime(s: &str) -> PyResult<chrono::DateTime<Utc>>;
```

---

## Фаза 2: Высокое влияние (MEDIUM complexity)

### 2.1 `bot_analytics.py` — Аналитика сессий

| Функция | Описание | Причина миграции | Ускорение |
|---------|----------|------------------|-----------|
| `attach_messages_to_sessions()` | Привязка сообщений к сессиям | Вложенные циклы, 10k+ записей | 10-20x |
| `compute_retention()` | Расчёт D1/D7 retention | Операции с множествами | 5-10x |
| `build_metrics()` | Агрегация метрик | defaultdict, comprehensions | 5-10x |

**Структуры данных:**
```rust
#[pyclass]
struct SessionMetrics {
    session_id: i64,
    user_id: i64,
    message_count: usize,
    first_message: DateTime<Utc>,
    last_message: DateTime<Utc>,
}

#[pyclass]
struct RetentionMetrics {
    d1_retention: f64,
    d7_retention: f64,
    total_users: usize,
}
```

### 2.2 `chat_analysis/fetcher.py` — Форматирование сообщений

| Функция | Описание | Причина миграции | Ускорение |
|---------|----------|------------------|-----------|
| `format_messages_for_llm()` | Форматирование 1000+ сообщений | Параллелизуемый цикл | 8-15x |
| `get_metadata()` | Агрегация статистики | Итеративное извлечение | 5-10x |
| `_format_message()` | Форматирование одного сообщения | Строковые операции | 5x |

**Интерфейс Rust:**
```rust
#[pyfunction]
fn format_messages_for_llm(
    messages: Vec<MessageData>,
    include_reactions: bool,
) -> String;

#[pyfunction]
fn get_chat_metadata(messages: Vec<MessageData>) -> ChatMetadata;
```

### 2.3 `chat_analysis/models.py` — Генерация отчётов

| Функция | Описание | Причина миграции | Ускорение |
|---------|----------|------------------|-----------|
| `_format_topics()` | Форматирование тем | Итерация + join | 5-8x |
| `_format_participants()` | Форматирование участников | Сортировка + форматирование | 5x |
| `_format_insights()` | Форматирование инсайтов | Строковые операции | 5x |
| `to_markdown()` | Полный отчёт | Объединение секций | 5-8x |

---

## Фаза 3: Интеграция

### ⚠️ Статус: Отменена

После анализа бенчмарков установлено, что функции Фазы 3 **не являются хорошими кандидатами** для миграции на Rust:

1. **`load_messages_from_chat()`** - работает с объектами Telegram (User, Chat, Channel) через Telethon. Это Python-специфичные типы, которые нельзя эффективно передать в Rust.

2. **`get_chat_type()`** - использует `isinstance()` проверки для Python-объектов. FFI overhead (~500-600 ns) превышает время выполнения простых проверок типов.

### Рекомендация

Оставить эти функции на Python. Rust эффективен только для:
- Regex-интенсивных операций (15-30x ускорение в чистом Rust, 2-5x через FFI)
- Batch обработки (FFI overhead амортизируется)
- CPU-интенсивных вычислений (парсинг, сериализация)

---

## Архитектура модуля

```
telegram_reader (PyO3 модуль)
├── ChatTarget           # ✅ Реализовано
├── Message              # ✅ Реализовано
├── SendResult           # ✅ Реализовано
├── list_configured_chats()  # ✅ Реализовано
├── resolve_chat()       # ✅ Реализовано
├── check_session()      # ✅ Реализовано
│
├── text_processing      # ✅ Фаза 1 (завершена)
│   ├── parse_reactions()      # ✅
│   ├── sanitize_filename()    # ✅
│   ├── contains_phone()       # ✅
│   ├── detect_conversion()    # ✅
│   ├── is_meaningful_message() # ✅
│   ├── parse_iso_datetime()   # ✅
│   ├── build_message_text()   # ✅
│   └── collect_reactions_summary() # ✅
│
├── analytics            # ✅ Фаза 2.1 (завершена)
│   ├── SessionMetrics         # ✅
│   ├── RetentionMetrics       # ✅
│   └── attach_message()       # ✅ (метод SessionMetrics)
│
└── formatting           # ✅ Фаза 2.2 (завершена)
    ├── MessageData            # ✅
    ├── ChatMetadata           # ✅
    ├── format_messages_for_llm() # ✅
    └── get_chat_metadata()    # ✅
```

---

## Оценка трудозатрат

| Фаза | Функций | Сложность | Статус |
|------|---------|-----------|--------|
| Фаза 1 | 8 | LOW | ✅ Завершена |
| Фаза 2.1 | 3 | MEDIUM | ✅ Завершена |
| Фаза 2.2 | 4 | MEDIUM | ✅ Завершена |
| Фаза 3 | 3 | LOW-MEDIUM | ❌ Отменена (FFI overhead) |
| **Итого** | **15** | — | **100% завершено** |

---

## Ожидаемые результаты

| Операция | Текущее время | После миграции | Ускорение |
|----------|---------------|----------------|-----------|
| Экспорт 5000 сообщений | ~10 сек | ~1-2 сек | 5-10x |
| Аналитика 50k сообщений | ~30 сек | ~2-3 сек | 10-15x |
| A/B анализ 10k сообщений | ~5 сек | ~0.2 сек | 20-30x |
| Генерация отчёта | ~2 сек | ~0.3 сек | 5-8x |

---

## Прогресс

### Завершено

1. [x] Реализовать `text_processing` модуль в `src/python.rs`
2. [x] Добавить тесты для новых функций (46 тестов)
3. [x] SessionMetrics и RetentionMetrics классы
4. [x] format_messages_for_llm и get_chat_metadata
5. [x] Python test generator (tests/generate_pyo3_tests.py)
6. [x] Обновить Python-код для использования Rust-реализаций
7. [x] Измерить реальное ускорение с бенчмарками

### Миграция завершена

Фаза 3 отменена после анализа бенчмарков - FFI overhead превышает выигрыш для простых операций с Python-объектами.

---

## Результаты бенчмарков

### Criterion (чистый Rust)

| Функция | Время | Примечание |
|---------|-------|------------|
| `sanitize_filename` | 830 ns | С regex |
| `contains_phone` (положительный) | 35 ns | Regex скомпилирован статически |
| `contains_phone` (отрицательный) | 84 ns | — |
| `detect_conversion` | 456 ns | Множественные проверки |
| `is_meaningful_message` | 4 ns | Простые строковые операции |
| `build_message_text` (с медиа) | 59 ns | — |
| `build_message_text` (без медиа) | 13 ns | — |
| **Batch 1000 сообщений** | **79 µs** | 3 функции на сообщение |

### Python vs Rust (через PyO3 FFI)

| Функция | Python | Rust (FFI) | Ускорение |
|---------|--------|------------|-----------|
| `contains_phone` (короткий) | 1.13 µs | 531 ns | **2.1x** |
| `contains_phone` (длинный) | 2.78 µs | 557 ns | **5.0x** |
| `sanitize_filename` | 2.12 µs | 2.54 µs | 0.8x (FFI overhead) |
| `is_meaningful_message` | 172 ns | 600 ns | 0.3x (FFI overhead) |
| **Batch 10k сообщений** | **18.05 ms** | **15.84 ms** | **1.1x** |

### Выводы

1. **Regex-интенсивные функции** (`contains_phone`) получают значительное ускорение (2-5x) благодаря lazy_static компиляции.

2. **Простые строковые операции** имеют FFI overhead (~500-600 ns), который превышает время выполнения. Для таких функций чистый Python быстрее.

3. **Batch обработка** показывает умеренное ускорение (1.1x), так как FFI overhead амортизируется.

4. **Рекомендация**: Использовать Rust только для regex-интенсивных функций (`contains_phone`) и batch операций. Простые функции оставить на Python.

---

## API Reference

### Text Processing

```python
from telegram_reader import (
    sanitize_filename,
    contains_phone,
    detect_conversion,
    is_meaningful_message,
    parse_iso_datetime,
    build_message_text,
    collect_reactions_summary,
)

# Очистка имени файла
filename = sanitize_filename("My Chat @#$", fallback="unknown", max_length=50)
# -> "My_Chat"

# Проверка телефона
has_phone = contains_phone("+0 000 000-00-00")
# -> True

# Детекция конверсии
conversion = detect_conversion("Хочу купить!")
# -> "purchase_intent"

# Проверка осмысленности сообщения
is_meaningful = is_meaningful_message("Hello")
# -> True

# Парсинг ISO даты
parsed = parse_iso_datetime("2024-01-15T10:30:00Z")
# -> {"year": "2024", "month": "01", ..., "iso": "2024-01-15T10:30:00+00:00"}

# Форматирование текста сообщения
text = build_message_text("Hello", has_media=True)
# -> "Hello [Media]"

# Сводка реакций
summary = collect_reactions_summary(["🔥", "❤️", "👍"])
# -> "🔥❤️👍"
```

### Analytics Classes

```python
from telegram_reader import SessionMetrics, RetentionMetrics

# Метрики сессии
session = SessionMetrics(
    session_id=123,
    user_id=456,
    bot_name="my_bot",
    session_start="2024-01-15T10:00:00"
)
session.attach_message("Hello!", "incoming")
print(session.is_engaged())  # True после 1 осмысленного сообщения
print(session.is_multi_turn())  # True после 2 осмысленных сообщений

# Метрики retention
retention = RetentionMetrics(total_users=100, d1_base=80)
retention.d1_returned = 40
print(retention.d1_rate())  # 50.0
```

### Formatting Functions

```python
from telegram_reader import MessageData, ChatMetadata, format_messages_for_llm, get_chat_metadata

# Форматирование сообщений для LLM
messages = [
    MessageData(sender="EntityA", text="Hello", date="10:00", reactions="🔥"),
    MessageData(sender="EntityB", text="Hi!", date="10:01", reactions=None),
]
formatted = format_messages_for_llm(messages, include_reactions=True)

# Получение метаданных чата
metadata = get_chat_metadata(messages)
print(metadata.message_count)  # 2
print(metadata.unique_senders)  # 2
```

---

## Обновлённые Python файлы

Следующие файлы были обновлены для использования Rust-реализаций:

| Файл | Функции | Fallback |
|------|---------|----------|
| `bot_analytics.py` | `is_meaningful_message`, `contains_phone` | Python при отсутствии Rust |
| `ab_testing.py` | `detect_conversion` | Python при отсутствии Rust |
| `chat_export_utils.py` | `sanitize_filename`, `build_message_text` | Python при отсутствии Rust |
| `export_linear_to_mysql.py` | `parse_iso_datetime` | Python при отсутствии Rust |

Все файлы автоматически переключаются на Python-реализацию если модуль `telegram_reader` не установлен.
