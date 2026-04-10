# Plan: Саппорт/Helpdesk автопилот (FAQ + статусы + CRM)

> Протокол: SPIDER-SOLO  
> Связанная спецификация: `codev/specs/0002-helpdesk-autopilot.md`  
> Revenue Potential: 300,000 - 800,000 руб/мес

## Цели
- Автоматизировать 40-60% запросов поддержки через AI
- Интегрировать с CRM для статусов и безопасных обновлений
- Снизить нагрузку на операторов на 50%
- Сократить время первого ответа до <5 сек

## Фазы

### Фаза 1: Архитектура и база знаний (Status: pending)
**Objective**: Спроектировать систему и подготовить источники данных

**Tasks**:
- Выбрать CRM систему клиента (Zendesk/Freshdesk/HubSpot) и получить API доступы
- Определить список whitelist полей для обновления
- Подготовить базу знаний (топ-50 FAQ в структурированном виде)
- Спроектировать staged pipeline архитектуру:
  - Intent Classification → Routing → Specialized Handlers → Validation → Execution
- Настроить sandbox CRM окружение для тестов
- Создать data models для запросов, ответов, audit log

**Dependencies**: None

**Success Metrics**:
- База знаний с 50+ FAQ готова
- CRM API sandbox доступен
- Архитектурная диаграмма согласована
- Data models определены

**Evaluation**: Architectural review, stakeholder approval

**Commit**: Single commit after user approval with all architecture docs and data models

---

### Фаза 2: Intent Classification и Routing (Status: pending)
**Objective**: Классифицировать запросы и направлять к правильному обработчику

**Dependencies**: Phase 1 (Architecture)

**Tasks**:
- Реализовать intent classifier (FAQ/Status/Update/Unknown)
- Создать router для направления к специализированным обработчикам
- Добавить confidence scoring (порог для эскалации)
- Реализовать fallback на эскалацию при low confidence (<70%)
- Написать unit tests для всех intent типов
- Добавить логирование классификации

**Success Metrics**:
- Intent classification accuracy ≥90% на тестовых данных
- Routing работает корректно для всех типов
- Confidence scoring calibrated
- Test coverage >85%

**Evaluation**: Accuracy tests, confidence calibration review, test coverage check

**Commit**: Single commit with intent classifier, router, and tests

---

### Фаза 3: FAQ Handler (Status: pending)
**Objective**: Обработка вопросов из базы знаний

**Dependencies**: Phase 2 (Routing)

**Tasks**:
- Реализовать semantic search по базе знаний
- Создать FAQ response generator с источниками
- Добавить версионирование ответов (дата актуальности)
- Реализовать "не знаю" ответы при отсутствии информации
- Написать integration tests с реальной базой знаний
- Benchmark response time (<5 сек p95)

**Success Metrics**:
- Semantic search precision ≥95%
- Ответы содержат источники и дату
- Response time <5 сек p95
- Test coverage >85%

**Evaluation**: Search accuracy tests, response time benchmarks, user testing with sample FAQs

**Commit**: Single commit with FAQ handler and tests

---

### Фаза 4: Status Checker (CRM Read) (Status: pending)
**Objective**: Проверка статусов заказов/тикетов через CRM API

**Dependencies**: Phase 2 (Routing), CRM API access

**Tasks**:
- Реализовать CRM client (для выбранной системы)
- Создать status checker с кешированием (TTL 30 сек)
- Обработать ошибки API (rate limits, timeouts)
- Реализовать fallback при недоступности CRM
- Написать integration tests с mock CRM API
- Добавить retry logic с exponential backoff

**Success Metrics**:
- Status retrieval accuracy 100%
- Response time <3 сек p95
- Graceful handling of CRM downtime
- Test coverage >85%

**Evaluation**: Integration tests pass, failover test, latency benchmarks

**Commit**: Single commit with status checker and tests

---

### Фаза 5: Field Updater (CRM Write) (Status: pending)
**Objective**: Безопасное обновление разрешённых полей в CRM

**Dependencies**: Phase 4 (CRM Read)

**Tasks**:
- Реализовать field updater с whitelist validation
- Создать audit log для всех CRM операций
- Добавить двойную валидацию перед записью
- Реализовать rollback механизм при ошибках
- Написать security tests (попытки обновить запрещённые поля)
- Добавить alerting при подозрительных операциях

**Success Metrics**:
- 0 несанкционированных изменений
- Все операции в audit log
- Rollback работает корректно
- Security tests pass
- Test coverage >90%

**Evaluation**: Security review, audit log verification, rollback tests

**Commit**: Single commit with field updater, audit log, and security tests

---

### Фаза 6: Escalation System (Status: pending)
**Objective**: Прозрачная эскалация операторам

**Dependencies**: Phase 2 (Routing)

**Tasks**:
- Реализовать escalation triggers (low confidence, стоп-слова, ошибки API)
- Создать operator notification system
- Добавить context preservation (история диалога)
- Реализовать operator dashboard для эскалаций
- Написать tests для всех escalation сценариев
- Добавить метрики эскалаций

**Success Metrics**:
- Все эскалации с полным контекстом
- Notification latency <2 мин
- Dashboard отображает все эскалации
- Test coverage >85%

**Evaluation**: Escalation flow tests, operator UX review, notification timing

**Commit**: Single commit with escalation system and dashboard

---

### Фаза 7: Multi-channel Integration (Status: pending)
**Objective**: Поддержка чат/email/мессенджер каналов

**Dependencies**: Phase 3-6 (All handlers)

**Tasks**:
- Реализовать channel adapters (чат/email/telegram/...)
- Унифицировать message format внутри системы
- Добавить channel-specific formatting ответов
- Реализовать rate limiting per channel
- Написать integration tests для каждого канала
- Benchmark throughput (100 одновременных сессий)

**Success Metrics**:
- Все каналы работают с единым core
- Channel-specific formatting корректен
- Throughput ≥100 сессий одновременно
- Test coverage >85%

**Evaluation**: Multi-channel tests, load testing, message formatting review

**Commit**: Single commit with channel adapters and tests

---

### Фаза 8: Monitoring & Analytics (Status: pending)
**Objective**: Наблюдаемость и метрики

**Dependencies**: Phase 7 (All components integrated)

**Tasks**:
- Реализовать structured logging (JSON)
- Создать metrics dashboard (deflection rate, response time, CSAT)
- Добавить alerting (high error rate, API downtime, low confidence spike)
- Реализовать weekly report generator
- Написать tests для metrics collection
- Интегрировать с существующим мониторингом (если есть)

**Success Metrics**:
- Все ключевые метрики в dashboard
- Alerting работает корректно
- Weekly reports автоматические
- Logs структурированы и searchable

**Evaluation**: Dashboard review, alerting tests, report format approval

**Commit**: Single commit with monitoring, analytics, and alerting

---

### Фаза 9: Pilot Deployment (Status: pending)
**Objective**: Пилотный запуск на 10-20% трафика

**Dependencies**: Phase 8 (Monitoring)

**Tasks**:
- Подготовить production infrastructure
- Настроить traffic splitting (10-20% на автопилот)
- Обучить операторов работе с эскалациями
- Запустить monitoring 24/7
- Собрать feedback от операторов и клиентов
- Провести weekly review с клиентом

**Success Metrics**:
- Deflection rate ≥40%
- CSAT ≥4.0/5.0
- Response time <5 сек p95
- 0 критических инцидентов
- Positive operator feedback

**Evaluation**: Pilot metrics review, stakeholder feedback, weekly retrospective

**Commit**: Single commit with production configs and pilot results

---

### Фаза 10: Scale & Optimize (Status: pending)
**Objective**: Масштабирование на 100% трафика и оптимизация

**Dependencies**: Phase 9 (Pilot success)

**Tasks**:
- Увеличить traffic splitting до 100%
- Оптимизировать response time (кеширование, warm-up)
- Расширить базу знаний на основе новых вопросов
- Настроить continuous improvement pipeline
- Реализовать A/B testing для промптов
- Провести final review с клиентом

**Success Metrics**:
- Deflection rate ≥50%
- Response time <3 сек p95
- CSAT ≥4.2/5.0
- 50% reduction в нагрузке на операторов
- Client satisfaction

**Evaluation**: Full metrics review, ROI calculation, client sign-off

**Commit**: Single commit with optimization improvements and final metrics

---

## Открытые вопросы
- [ ] Выбор CRM системы клиента (требует решения в Фазе 1)
- [ ] Список whitelist полей для CRM обновлений (требует согласования)
- [ ] Формат базы знаний (Markdown/Confluence/Notion)
- [ ] Каналы для пилота (приоритеты: чат > email > мессенджер)
- [ ] Требования к многоязычности (RU only или RU+EN)

## Success Metrics (Overall)
- Deflection rate ≥50% после полного развёртывания
- Time to first response <5 сек p95
- CSAT ≥4.0/5.0
- 50% reduction в операторской нагрузке
- 99.5%+ availability (SLA)
- ROI окупаемость <2 месяцев

## Risks & Mitigation
- **Риск**: Устаревшая база знаний → **Mitigation**: Weekly content review
- **Риск**: CRM API лимиты → **Mitigation**: Caching, rate limiting, failover
- **Риск**: Low adoption → **Mitigation**: Operator training, feedback loop
- **Риск**: Security issues → **Mitigation**: Whitelist validation, audit log, security tests

## Артефакты
- Спецификация: `codev/specs/0002-helpdesk-autopilot.md`
- План: `codev/plans/0002-helpdesk-autopilot.md` (текущий документ)
- Ревью: `codev/reviews/0002-helpdesk-autopilot.md` (создать после завершения)

## Timeline Note
⚠️ **NO TIME ESTIMATES** - Progress measured by completed phases, not calendar time. Focus on deliverables and quality.
