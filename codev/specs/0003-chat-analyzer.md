# SPEC-0003: AI Chat Analyzer

**Status:** Draft
**Created:** 2025-11-24
**Author:** @stden
**Priority:** High
**Complexity:** Medium

---

## 📋 Обзор

Система AI-анализа Telegram чатов для автоматической категоризации, определения тематики, анализа тональности и извлечения ключевых метрик.

### Контекст

**Проблема:**
- Пользователи участвуют в десятках Telegram чатов
- Сложно быстро понять тематику нового чата
- Нет автоматической категоризации и систематизации
- Запрос от Eldor из чата @vibecod3rs: нужна автоматизация парсинга чатов по критериям

**Решение:**
AI-анализатор, который:
1. Читает последние N сообщений из чата
2. Анализирует через LLM (OpenAI/Claude/Gemini)
3. Определяет тематику, подтемы, тональность
4. Извлекает ключевые метрики и инсайты
5. Сохраняет результаты в структурированном виде

---

## 🎯 Цели и не-цели

### Цели
- [x] Автоматическая категоризация чатов по тематике
- [x] Анализ тональности и эмоционального фона
- [x] Извлечение ключевых тем и обсуждений
- [x] Метрики активности и вовлечённости
- [x] Экспорт результатов в JSON/Markdown
- [x] CLI интерфейс для быстрого использования

### Не-цели
- ❌ Real-time мониторинг (будет в следующей версии)
- ❌ Web UI (отдельная спецификация)
- ❌ Автоматическое действие на основе анализа (будет позже)
- ❌ Анализ медиа-контента (отдельная спецификация OCR)

---

## 🏗️ Архитектура

### Компоненты

```
┌─────────────────────────────────────────────┐
│           ChatAnalyzer (main)               │
├─────────────────────────────────────────────┤
│  - analyze_chat(chat_id, limit)            │
│  - get_chat_messages()                      │
│  - prepare_context()                        │
│  - call_llm_analysis()                      │
│  - parse_results()                          │
│  - save_results()                           │
└─────────────────────────────────────────────┘
           │                    │
           ▼                    ▼
┌──────────────────┐   ┌──────────────────┐
│  MessageFetcher  │   │   LLMAnalyzer    │
├──────────────────┤   ├──────────────────┤
│ - get_messages() │   │ - categorize()   │
│ - filter()       │   │ - sentiment()    │
│ - format()       │   │ - extract_topics()│
└──────────────────┘   └──────────────────┘
           │                    │
           ▼                    ▼
┌──────────────────┐   ┌──────────────────┐
│ TelegramSession  │   │  OpenAI/Claude   │
├──────────────────┤   ├──────────────────┤
│ - connect()      │   │ - completion()   │
│ - get_entity()   │   │ - embeddings()   │
└──────────────────┘   └──────────────────┘
```

### Data Flow

```
User Input (chat URL/ID)
    │
    ▼
MessageFetcher.get_messages(limit=1000)
    │
    ▼
Filter & Format Messages
    │
    ▼
Prepare Context for LLM
    │
    ▼
LLM Analysis (categorize, sentiment, topics)
    │
    ▼
Parse & Structure Results
    │
    ▼
Save to JSON + Generate Markdown Report
    │
    ▼
Return Analysis Results
```

---

## 📊 Data Models

### ChatAnalysisResult

```python
@dataclass
class ChatAnalysisResult:
    """Результат анализа чата."""

    # Базовая информация
    chat_id: int
    chat_title: str
    chat_type: str  # "private", "group", "channel"
    analysis_date: datetime
    messages_analyzed: int

    # Категоризация
    primary_category: str  # "IT", "Business", "Entertainment", etc.
    subcategories: List[str]  # ["AI/ML", "Web Development", "DevOps"]
    tags: List[str]  # ["python", "ai", "automation"]

    # Тональность
    sentiment: str  # "positive", "neutral", "negative"
    sentiment_score: float  # -1.0 to 1.0
    toxicity_level: str  # "low", "medium", "high"

    # Темы и обсуждения
    main_topics: List[Topic]  # Топ-5 обсуждаемых тем
    trending_topics: List[str]  # Актуальные темы последних дней
    key_discussions: List[Discussion]  # Важные обсуждения

    # Метрики активности
    activity_metrics: ActivityMetrics

    # AI-сгенерированное описание
    summary: str  # Краткое саммари чата (2-3 предложения)
    description: str  # Развёрнутое описание (1-2 параграфа)

    # Рекомендации
    recommendations: List[str]  # Что делать с этим чатом
    similar_chats: List[str]  # Похожие чаты (если есть в базе)


@dataclass
class Topic:
    """Тема обсуждения."""
    name: str
    mentions: int
    sentiment: str
    key_messages: List[int]  # Message IDs


@dataclass
class Discussion:
    """Важное обсуждение."""
    title: str
    date: datetime
    participants: int
    messages_count: int
    summary: str


@dataclass
class ActivityMetrics:
    """Метрики активности."""
    total_messages: int
    active_users: int
    messages_per_day: float
    peak_hours: List[int]  # [9, 10, 18, 19, 20]
    avg_message_length: int
    media_percentage: float
    reactions_count: int
```

### Configuration

```python
@dataclass
class AnalyzerConfig:
    """Конфигурация анализатора."""

    # Параметры выборки
    message_limit: int = 1000  # Сколько сообщений анализировать
    days_back: int = 30  # За сколько дней

    # LLM параметры
    llm_provider: str = "openai"  # "openai", "claude", "gemini"
    model: str = "gpt-4o-mini"
    temperature: float = 0.3
    max_tokens: int = 2000

    # Фильтры
    min_message_length: int = 10  # Минимальная длина сообщения
    include_media: bool = False  # Учитывать медиа
    exclude_bots: bool = True  # Исключать ботов

    # Вывод
    output_format: str = "both"  # "json", "markdown", "both"
    output_dir: Path = Path("./analysis_results")
    verbose: bool = True
```

---

## 🔌 API Design

### CLI Interface

```bash
# Базовый анализ
python chat_analyzer.py @channel_name

# С параметрами
python chat_analyzer.py @channel_name \
  --limit 2000 \
  --model gpt-4o \
  --output json \
  --save-to /path/to/results

# Batch анализ
python chat_analyzer.py --batch chats.txt

# С фильтрами
python chat_analyzer.py @channel_name \
  --days 7 \
  --min-length 50 \
  --exclude-bots
```

### Python API

```python
from chat_analyzer import ChatAnalyzer, AnalyzerConfig

# Базовое использование
analyzer = ChatAnalyzer()
result = await analyzer.analyze("@channel_name")
print(result.summary)

# С конфигурацией
config = AnalyzerConfig(
    message_limit=2000,
    model="gpt-4o",
    temperature=0.3
)
analyzer = ChatAnalyzer(config)
result = await analyzer.analyze("@channel_name")

# Batch анализ
results = await analyzer.analyze_batch([
    "@channel1",
    "@channel2",
    "https://t.me/channel3"
])

# Сохранение результатов
result.save_json("analysis.json")
result.save_markdown("analysis.md")
```

---

## 🎨 Prompts

### System Prompt для категоризации

Файл: `prompts/chat_categorizer.md`

```markdown
Ты - эксперт по анализу Telegram чатов. Твоя задача - определить тематику, тональность и ключевые характеристики чата на основе его сообщений.

## Твои задачи:

1. **Категоризация:**
   - Определи основную категорию (IT, Business, Entertainment, Education, etc.)
   - Определи подкатегории (AI/ML, Web Dev, Marketing, etc.)
   - Подбери релевантные теги

2. **Анализ тональности:**
   - Общая тональность (positive/neutral/negative)
   - Уровень токсичности (low/medium/high)
   - Профессиональность vs casualness

3. **Ключевые темы:**
   - 5 самых обсуждаемых тем
   - Актуальные темы (последние 7 дней)
   - Важные обсуждения

4. **Метрики:**
   - Уровень активности
   - Качество контента
   - Вовлечённость участников

## Формат ответа (JSON):

```json
{
  "primary_category": "IT и программирование",
  "subcategories": ["AI/ML", "Веб-разработка", "DevOps"],
  "tags": ["python", "ai", "automation", "telegram"],
  "sentiment": "positive",
  "sentiment_score": 0.7,
  "toxicity_level": "low",
  "main_topics": [
    {
      "name": "AI-инструменты для разработки",
      "mentions": 45,
      "sentiment": "positive"
    }
  ],
  "summary": "Активное IT-сообщество, обсуждающее AI-инструменты...",
  "description": "Чат объединяет разработчиков...",
  "recommendations": [
    "Отличный чат для знакомства с новыми AI-инструментами",
    "Рекомендуется для разработчиков интересующихся автоматизацией"
  ]
}
```

## Важно:
- Анализируй контекст, а не отдельные сообщения
- Учитывай культурные особенности
- Будь объективным в оценках
- Если данных недостаточно - так и скажи
```

---

## 🧪 Testing Strategy

### Unit Tests

```python
# test_chat_analyzer.py

def test_message_fetcher():
    """Тест получения сообщений из чата."""
    fetcher = MessageFetcher()
    messages = await fetcher.get_messages("@test_chat", limit=100)
    assert len(messages) <= 100
    assert all(hasattr(m, 'text') for m in messages)


def test_llm_categorization():
    """Тест категоризации через LLM."""
    analyzer = LLMAnalyzer()
    result = await analyzer.categorize(sample_messages)
    assert result.primary_category in VALID_CATEGORIES
    assert 0 <= result.sentiment_score <= 1


def test_result_serialization():
    """Тест сериализации результатов."""
    result = ChatAnalysisResult(...)
    json_str = result.to_json()
    restored = ChatAnalysisResult.from_json(json_str)
    assert result == restored
```

### Integration Tests

```python
def test_full_analysis_flow():
    """Интеграционный тест полного цикла анализа."""
    analyzer = ChatAnalyzer()
    result = await analyzer.analyze("@test_chat")

    assert result.chat_title is not None
    assert result.messages_analyzed > 0
    assert result.primary_category is not None
    assert result.summary is not None

    # Проверка сохранения
    result.save_json("test_result.json")
    assert Path("test_result.json").exists()
```

### Behave Scenarios

```gherkin
# features/chat_analyzer.feature

Feature: Chat Analysis
  Анализ Telegram чатов с помощью AI

  Scenario: Анализ IT-чата
    Given я имею доступ к чату "@vibecod3rs"
    When я запускаю анализ с лимитом 500 сообщений
    Then категория должна быть "IT и программирование"
    And тональность должна быть "positive" или "neutral"
    And результат должен содержать топ-5 тем

  Scenario: Сохранение результатов
    Given результат анализа чата "@test_chat"
    When я сохраняю результат в JSON
    Then файл должен быть создан
    And JSON должен быть валидным
    And должны быть все обязательные поля
```

---

## 📁 File Structure

```
.
├── chat_analyzer.py           # Основной модуль
├── chat_analysis/
│   ├── __init__.py
│   ├── fetcher.py            # MessageFetcher
│   ├── llm_analyzer.py       # LLMAnalyzer
│   ├── models.py             # Data models
│   ├── config.py             # Configuration
│   └── utils.py              # Утилиты
├── prompts/
│   ├── chat_categorizer.md   # Промпт категоризации
│   ├── sentiment_analyzer.md # Промпт анализа тональности
│   └── topic_extractor.md    # Промпт извлечения тем
├── analysis_results/          # Результаты анализа
│   ├── vibecod3rs_20251124.json
│   ├── vibecod3rs_20251124.md
│   └── ...
├── tests/
│   ├── test_chat_analyzer.py
│   ├── test_fetcher.py
│   └── test_llm_analyzer.py
└── features/
    ├── chat_analyzer.feature
    └── steps/
        └── chat_analyzer_steps.py
```

---

## 🚀 Implementation Plan

### Phase 1: MVP (Week 1)
- [ ] Создать базовый ChatAnalyzer
- [ ] Реализовать MessageFetcher
- [ ] Интеграция с OpenAI API
- [ ] Базовая категоризация
- [ ] CLI интерфейс
- [ ] Сохранение в JSON

### Phase 2: Enhanced Analysis (Week 2)
- [ ] Sentiment analysis
- [ ] Topic extraction
- [ ] Activity metrics
- [ ] Markdown reports
- [ ] Batch processing

### Phase 3: Optimization (Week 3)
- [ ] Кэширование результатов
- [ ] Incremental analysis (только новые сообщения)
- [ ] Multi-LLM support (Claude, Gemini)
- [ ] Embeddings для семантического поиска

### Phase 4: Polish (Week 4)
- [ ] Comprehensive testing
- [ ] Documentation
- [ ] Performance optimization
- [ ] Error handling
- [ ] CLI improvements

---

## 📊 Success Metrics

### Функциональность
- [x] Анализирует чат за < 30 секунд (1000 сообщений)
- [x] Точность категоризации > 85%
- [x] Все результаты в структурированном JSON
- [x] CLI работает без ошибок

### Качество кода
- [x] Test coverage > 80%
- [x] Все Behave сценарии проходят
- [x] Код проходит mypy type checking
- [x] Документация для всех публичных API

### UX
- [x] Понятный CLI интерфейс
- [x] Прогресс-бар при анализе
- [x] Читаемые Markdown отчёты
- [x] Полезные error messages

---

## 🔒 Security & Privacy

### Безопасность данных
- Все сообщения обрабатываются локально
- Передаются в LLM только анонимизированные тексты
- Результаты хранятся локально
- Опциональное шифрование результатов

### Privacy
- Не сохраняем личные данные пользователей
- Не отправляем метаданные третьим лицам
- Опция --anonymize для полного анонимизирования
- Предупреждение о политике конфиденциальности

---

## 🔧 Configuration

### Environment Variables

```bash
# .env
CHAT_ANALYZER_LLM_PROVIDER=openai
CHAT_ANALYZER_MODEL=gpt-4o-mini
CHAT_ANALYZER_OUTPUT_DIR=./analysis_results
CHAT_ANALYZER_CACHE_ENABLED=true
CHAT_ANALYZER_CACHE_TTL=86400  # 24 hours
```

### Config File

```yaml
# chat_analyzer_config.yml
analyzer:
  message_limit: 1000
  days_back: 30

llm:
  provider: openai
  model: gpt-4o-mini
  temperature: 0.3
  max_tokens: 2000

filters:
  min_message_length: 10
  exclude_bots: true
  include_media: false

output:
  format: both  # json, markdown, both
  directory: ./analysis_results
  verbose: true

cache:
  enabled: true
  ttl: 86400
  directory: ./cache
```

---

## 📚 References

- [OpenAI Chat Completions](https://platform.openai.com/docs/guides/chat)
- [Telethon Documentation](https://docs.telethon.dev/)
- [Sentiment Analysis Best Practices](https://arxiv.org/abs/2005.11401)
- [SPIDER Protocol](../protocols/spider-solo/protocol.md)

---

## ✅ Acceptance Criteria

### Обязательные
- [x] CLI запускается без ошибок
- [x] Анализирует любой публичный/доступный чат
- [x] Сохраняет результаты в JSON
- [x] Генерирует читаемый Markdown отчёт
- [x] Проходят все unit tests
- [x] Документирован API

### Желательные
- [ ] Поддержка 3+ LLM провайдеров
- [ ] Batch обработка
- [ ] Кэширование результатов
- [ ] Incremental updates
- [ ] Embeddings для поиска

---

**Next Steps:**
1. Review specification
2. Create implementation plan (codev/plans/)
3. Start Phase 1 development
4. Setup testing infrastructure

**Related:**
- ROADMAP.md - общий план проекта
- codev/plans/0003-chat-analyzer-plan.md - детальный план реализации
