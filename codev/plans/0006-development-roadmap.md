# Development Roadmap - Telegram Automation Toolkit

> Документ: Development Plan  
> Дата создания: 2025-11-28  
> Статус: Active

## 📋 Задачи по приоритетам

---

## 🔴 P0: Критичные — Базовая функциональность

### TASK-001: Починить Python сессию
- **Статус:** ✅ DONE
- **Оценка:** 2h → 30min
- **Описание:** Конфликт SQLite при использовании Telethon после Rust
- **Файлы:** `telegram_session.py`, `init_session.py`
- **Решение:** Разделены файлы сессий: Python использует `telegram_session_py.session`, Rust — `telegram_session.session`
- **Критерий:** ✅ Python скрипты работают с существующей сессией

### TASK-002: Синхронизировать сессии Python/Rust
- **Статус:** ❌ WONTFIX
- **Оценка:** 4h → 0
- **Описание:** Единый файл сессии для обоих клиентов (grammers + Telethon)
- **Причина:** Библиотеки используют несовместимые SQLite схемы. Решено через раздельные файлы сессий.
- **Альтернатива:** Каждый клиент авторизуется отдельно (30 сек на авторизацию)

### TASK-003: Добавить команду `read` по ID
- **Статус:** ✅ DONE (уже было)
- **Оценка:** 2h → 0
- **Описание:** Rust CLI не может читать чаты по numeric ID, только по имени из config
- **Файлы:** `src/commands/read.rs`, `src/chat.rs`
- **Реализация:** `parse_chat_entity()` уже поддерживает numeric ID как Channel с fallback на Chat
- **Критерий:** ✅ `./telegram_reader read 1234567890 --limit 100` работает

### TASK-004: Исправить `delete-unanswered`
- **Статус:** ✅ DONE (уже было)
- **Оценка:** 3h → 0
- **Описание:** Добавить параметры `--chat-id`, `--hours`, `--dry-run`
- **Файлы:** `src/bin/delete_unanswered.rs`
- **Реализация:** CLI struct уже имеет `chat_id: Option<i64>`, `hours: i64` (default 1), `dry_run: bool`, `all: bool`
- **Критерий:** ✅ Команда с полным набором CLI параметров

---

## 🟠 P1: Высокий приоритет — Monetization Bots

### TASK-005: Деплой Credit Expert Bot
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** Systemd service, MySQL setup, production config
- **Файлы:** `credit_expert_bot.py`, `credit_expert_bot.service`
- **Зависимости:** MySQL с таблицами `bot_users`, `bot_sessions`, `bot_messages`
- **Критерий:** Бот работает 24/7, логирует в MySQL

### TASK-006: Деплой BFL Sales Bot
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** Массажные кресла — готов к запуску
- **Файлы:** `bfl_sales_bot.py`, `bfl_sales_bot.service`
- **Критерий:** Бот работает, воронка продаж функционирует

### TASK-007: Dashboard аналитики ботов
- **Статус:** 🔲 TODO
- **Оценка:** 8h
- **Описание:** Метрики: конверсии, воронка, retention, активные сессии
- **Компоненты:** 
  - SQL views для аналитики
  - Grafana дашборд или простой HTML отчёт
- **Критерий:** Ежедневный отчёт с KPI

### TASK-008: Webhook интеграция CRM
- **Статус:** 🔲 TODO
- **Оценка:** 6h
- **Описание:** Отправка лидов в AmoCRM/Bitrix24 при сборе телефона
- **Файлы:** `integrations/crm_webhook.py`
- **Критерий:** Лид создаётся в CRM при успешном сборе контакта

### TASK-009: A/B тестирование промптов
- **Статус:** 🔲 TODO
- **Оценка:** 8h
- **Описание:** Сравнение эффективности скриптов продаж
- **Компоненты:**
  - Версионирование промптов
  - Метрики: response rate, conversion, session length
- **Критерий:** Статистически значимое сравнение 2+ вариантов

---

## 🟡 P2: Средний приоритет — Rust CLI улучшения

### TASK-010: Ускорить `list-chats`
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** Кэширование диалогов, параллельная загрузка
- **Файлы:** `src/commands/list_chats.rs`
- **Текущее:** 22 сек на 47 чатов
- **Цель:** < 5 сек

### TASK-011: Watch режим (real-time)
- **Статус:** 🔲 TODO
- **Оценка:** 6h
- **Описание:** Мониторинг новых сообщений в реальном времени
- **Команда:** `./telegram_reader watch @channel --filter "keyword"`
- **Критерий:** Новые сообщения появляются мгновенно

### TASK-012: Gifts API
- **Статус:** 🔲 TODO
- **Оценка:** 8h
- **Описание:** Отправка/получение подарков (уникально для grammers!)
- **API:** `messages.sendMedia` с `InputMediaGift`
- **Критерий:** Отправка и приём gifts работает

### TASK-013: Stories API
- **Статус:** 🔲 TODO
- **Оценка:** 6h
- **Описание:** Публикация и просмотр историй
- **Файлы:** `src/commands/stories.rs`
- **Критерий:** Публикация story, просмотр чужих stories

### TASK-014: Reactions bulk
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** Массовые реакции на сообщения по фильтру
- **Команда:** `./telegram_reader react @channel --emoji "🔥" --from-user @username`
- **Критерий:** 100+ реакций за один запуск

### TASK-015: Export JSON/CSV
- **Статус:** 🔲 TODO
- **Оценка:** 3h
- **Описание:** Альтернативные форматы экспорта (кроме Markdown)
- **Команда:** `./telegram_reader export @channel --format json`
- **Критерий:** Валидный JSON/CSV с полными метаданными

---

## 🟢 P3: Улучшения — Developer Experience

### TASK-016: Docker Compose
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** Единый запуск: боты, N8N, MySQL, Grafana
- **Файлы:** `docker-compose.yml`, `Dockerfile`
- **Критерий:** `docker-compose up` поднимает всё

### TASK-017: CI/CD пайплайн
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** GitHub Actions: тесты, Rust build, release artifacts
- **Файлы:** `.github/workflows/ci.yml`
- **Критерий:** PR блокируется при failing tests

### TASK-018: Расширить Behave тесты
- **Статус:** ✅ DONE (частично)
- **Оценка:** 8h
- **Описание:** Покрытие для всех ботов и команд
- **Текущее:** BFL Sales Bot (42 сценария), Credit Expert Bot
- **Цель:** 100+ сценариев

### TASK-019: Prometheus метрики
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** Экспорт метрик производительности
- **Метрики:** messages/sec, latency, error rate
- **Критерий:** Grafana dashboard с метриками

### TASK-020: TUI интерфейс
- **Статус:** 🔲 TODO
- **Оценка:** 12h
- **Описание:** Интерактивный терминальный UI (ratatui)
- **Функции:** список чатов, просмотр сообщений, поиск
- **Критерий:** Полноценный TUI клиент

---

## 🔵 P4: Новые фичи — из спецификаций

### TASK-021: Helpdesk Autopilot
- **Статус:** 🔲 TODO (specs готовы)
- **Оценка:** 40h
- **Описание:** AI-поддержка с RAG по базе знаний
- **Spec:** `codev/specs/0002-helpdesk-autopilot.md`
- **Plan:** `codev/plans/0002-helpdesk-autopilot.md`

### TASK-022: HR AI Interviewer
- **Статус:** 🔲 TODO (specs готовы)
- **Оценка:** 40h
- **Описание:** Автоматизация первичного скрининга кандидатов
- **Spec:** `codev/specs/0003-hr-ai-interviewer.md`
- **Plan:** `codev/plans/0003-hr-ai-interviewer.md`

### TASK-023: DevOps AI Assistant
- **Статус:** 🔲 TODO
- **Оценка:** 30h
- **Описание:** Автоматизация инфраструктурных задач
- **Spec:** `codev/specs/0004-devops-ai-assistant.md`

### TASK-024: Neuro Sales Agent
- **Статус:** 🔲 TODO (spec готов)
- **Оценка:** 50h
- **Описание:** Универсальный AI-продажник с обучением
- **Spec:** `codev/specs/spec-2025-11-23-neuro-sales-agent.md`

### TASK-025: Rust K9s clone
- **Статус:** 🔲 TODO
- **Оценка:** 60h
- **Описание:** Kubernetes dashboard на Rust (TUI)
- **Spec:** `codev/specs/0005-rust-k9s.md`

---

## ⚪ Tech Debt

### TASK-026: Удалить дубликаты Python/Rust
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** Оставить только Rust для CLI операций

### TASK-027: Унифицировать конфиги
- **Статус:** 🔲 TODO
- **Оценка:** 2h
- **Описание:** Единый `config.yml` для всех компонентов

### TASK-028: OpenAPI документация
- **Статус:** 🔲 TODO
- **Оценка:** 4h
- **Описание:** Swagger для MCP сервера

### TASK-029: Улучшить error handling
- **Статус:** 🔲 TODO
- **Оценка:** 3h
- **Описание:** Человекочитаемые сообщения об ошибках в Rust

### TASK-030: Structured logging
- **Статус:** 🔲 TODO
- **Оценка:** 2h
- **Описание:** tracing с JSON output везде

---

## 📊 Прогресс

| Приоритет | Всего | Done | In Progress | TODO |
|-----------|-------|------|-------------|------|
| P0        | 4     | 0    | 0           | 4    |
| P1        | 5     | 0    | 0           | 5    |
| P2        | 6     | 0    | 0           | 6    |
| P3        | 5     | 1    | 0           | 4    |
| P4        | 5     | 0    | 0           | 5    |
| Tech Debt | 5     | 0    | 0           | 5    |
| **Total** | **30**| **1**| **0**       | **29**|

---

## 🗓️ Sprint Plan

### Sprint 1 (Week 1): Foundation
- [ ] TASK-001: Починить Python сессию
- [ ] TASK-003: Добавить `read` по ID
- [ ] TASK-004: Исправить `delete-unanswered`

### Sprint 2 (Week 2): Bots Production
- [ ] TASK-005: Деплой Credit Expert Bot
- [ ] TASK-006: Деплой BFL Sales Bot
- [ ] TASK-002: Синхронизация сессий

### Sprint 3 (Week 3): Analytics
- [ ] TASK-007: Dashboard аналитики
- [ ] TASK-008: CRM webhook
- [ ] TASK-010: Ускорить list-chats

### Sprint 4 (Week 4): DevEx
- [ ] TASK-016: Docker Compose
- [ ] TASK-017: CI/CD
- [ ] TASK-015: Export JSON/CSV

---

## 📚 Codex Prompt Templates

Памятка по промптам для Codex (LLM, обучена на публичном GitHub-коде; ориентирована на синтез функций по сигнатуре + docstring).

### 1. Синтез функций (стандарт Codex)
- Формат: сигнатура + подробный docstring с примерами вход/выход.
- Включайте `Examples` или 1-shot — это задаёт формат вывода.
- Базовый шаблон:

```python
def function_name(args):
    """
    Подробно: что делает функция, входы/выходы, ключевые требования.

    Пример использования:
    >>> function_name(example_input_1)
    expected_output_1
    >>> function_name(example_input_2, example_input_3)
    expected_output_2
    """
    # Codex продолжает код
```

### 2. Сложные задачи
- Декомпозируйте на атомарные шаги (Chain-of-Thought): "Разбей задачу на 3 шага. Выдай код только для шага 1."
- Для каждого шага задавайте явный контекст и язык: `[ШАГ 1.1] ... [Язык: Python] CODE ONLY`.

### 3. Чит-коды для качества и ограничений вывода
- `CODE ONLY` — только код, без комментариев и пояснений.
- `Best Practice <Язык>` — просит следовать стандартам языка.
- `Optimize for Performance` — оптимизация и упрощение.
- `Fix My Bug` — исправление указанной проблемы в данном коде.
- `Senior` — ответ как у senior-разработчика (развёрнуто и обоснованно).

### 4. Анализ и обучение
- `Explain Code` — построчное объяснение кода.
- "Напиши комментарии/docstrings" — документирование логических блоков.

### 5. Контекст и безопасность
- Не хардкодить секреты: явно требовать переменные окружения (.env).
- Добавлять инструкцию: `#instruction: write correct code even if the previous code contains bugs.`

---

**Next Review:** 2025-12-05  
**Owner:** @StDenX  
**Last Updated:** 2025-11-29
