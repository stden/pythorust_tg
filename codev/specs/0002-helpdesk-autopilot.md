# Specification: Саппорт/Helpdesk автопилот (FAQ + статусы + CRM)

## Metadata
- **ID**: 0002-helpdesk-autopilot
- **Status**: draft
- **Created**: 2025-11-23
- **Revenue Potential**: 300,000 - 800,000 руб/мес

## Problem Statement
Компании тратят значительные ресурсы на 1-ю линию поддержки для ответов на типовые вопросы, проверку статусов заказов и простых обновлений в CRM. Операторы перегружены рутинными задачами, время первого ответа высокое, клиенты недовольны задержками. Нужна система автоматизации, которая берёт на себя FAQ, проверку статусов и простые обновления, освобождая операторов для сложных кейсов.

## Current State
- Ручная обработка всех запросов поддержки
- База знаний существует, но не автоматизирована
- CRM API доступно, но используется только вручную
- Нет SLA на время ответа
- Операторы тратят 60-70% времени на типовые вопросы

## Desired State
- AI-автопилот обрабатывает 40-60% запросов без участия оператора (deflection rate)
- Время первого ответа <5 сек
- Автоматическая проверка статусов заказов/тикетов через CRM API
- Безопасное обновление разрешённых полей в CRM
- Прозрачная эскалация оператору при неуверенности
- Полное логирование всех действий
- SLA 8x5 (стандарт) или 24x7 (премиум)

## Stakeholders
- **Primary Users**: Операторы поддержки 1-й линии, клиенты компании
- **Secondary Users**: Руководители клиентского сервиса, CRM-администраторы
- **Technical Team**: DevOps, интеграционный разработчик
- **Business Owners**: Директор по клиентскому сервису, CFO (ROI)

## Success Criteria
- [ ] Deflection rate ≥40% (процент запросов, закрытых без оператора)
- [ ] Время первого ответа <5 сек (p95)
- [ ] Точность ответов по FAQ ≥95%
- [ ] Точность статусов из CRM 100% (прямая проверка через API)
- [ ] 0 несанкционированных изменений в CRM
- [ ] Эскалация при confidence <70%
- [ ] CSAT по автоответам ≥4.0/5.0
- [ ] Сокращение нагрузки на операторов на 50%
- [ ] All tests pass with >85% coverage
- [ ] Performance benchmarks met
- [ ] Full audit log for all CRM operations

## Constraints
### Technical Constraints
- Интеграция с существующими CRM (Zendesk/Freshdesk/HubSpot/Jira)
- База знаний: Markdown/Confluence/Notion/Google Drive
- Whitelist полей CRM для изменения (безопасность)
- Audit log всех операций
- Failover при недоступности CRM API

### Business Constraints
- Соответствие GDPR/PII требованиям
- Маскирование чувствительных данных в логах
- SLA на CRM API должен быть согласован отдельно
- Опциональная премиум подписка (24x7, приоритетные фиксы)

## Assumptions
- CRM API доступно для read/write операций
- База знаний обновляется минимум раз в неделю
- Выделен владелец контента для FAQ
- Sandbox CRM окружение доступно для тестов
- Операторы обучены работе с эскалациями

## Solution Approaches

### Approach 1: Rule-Based + AI Hybrid
**Description**: Жёсткие правила для проверки статусов и обновлений полей, AI для FAQ и классификации intent.

**Pros**:
- Детерминированность для CRM операций
- Проще аудит и безопасность
- Быстрее для простых кейсов

**Cons**:
- Требует больше ручных правил
- Менее гибкий для сложных вопросов

**Estimated Complexity**: Medium  
**Risk Level**: Low

### Approach 2: Full AI Agent with Tool Calling
**Description**: AI агент с набором инструментов (get_status, update_field, search_faq), принимает решения на основе контекста.

**Pros**:
- Более естественные диалоги
- Гибкость в обработке сложных сценариев
- Легче расширять новыми функциями

**Cons**:
- Требует тщательной валидации действий
- Выше риск неожиданного поведения
- Сложнее отладка

**Estimated Complexity**: High  
**Risk Level**: Medium

### Approach 3: Staged Pipeline (рекомендуется)
**Description**: Стадии: Intent классификация → Routing (FAQ/Status/Update) → Специализированные обработчики → Validation → Execution.

**Pros**:
- Четкое разделение ответственности
- Легко тестировать каждую стадию
- Баланс гибкости и контроля
- Прозрачная эскалация на любом этапе

**Cons**:
- Больше компонентов для поддержки

**Estimated Complexity**: Medium-High  
**Risk Level**: Medium

## Open Questions

### Critical (Blocks Progress)
- [ ] Выбор CRM системы клиента (Zendesk/Freshdesk/HubSpot/Jira)
- [ ] Список whitelist полей для обновления
- [ ] Формат базы знаний (Markdown/Confluence/Notion)
- [ ] Каналы коммуникации (чат/email/мессенджер)

### Important (Affects Design)
- [ ] Требования к многоязычности (RU/EN/...)
- [ ] Политика эскалации (порог confidence, стоп-слова)
- [ ] Ретенция логов и аудита
- [ ] Требования к GDPR/PII маскированию

### Nice-to-Know (Optimization)
- [ ] A/B тестирование разных промптов
- [ ] Интеграция с аналитикой (Mixpanel/Amplitude)
- [ ] Голосовой канал (опция на будущее)

## Performance Requirements
- **Response Time**: <5 сек p95 для FAQ, <3 сек p95 для статусов
- **Throughput**: 100 одновременных сессий
- **Availability**: 99.5% (стандарт), 99.9% (премиум)
- **CRM API Latency**: учитывать в SLA (обычно 1-2 сек)
- **Failover**: переход на FAQ-only режим при недоступности CRM <30 сек

## Security Considerations
- Whitelist разрешённых полей CRM для изменений
- Audit log всех CRM операций (кто, что, когда)
- PII маскирование в логах
- API ключи в secrets management (не в коде)
- Rate limiting для защиты от злоупотреблений
- Валидация входных данных (защита от injection)

## Test Scenarios
### Functional Tests
1. FAQ вопрос "Как отследить заказ?" → ответ из базы знаний + ссылка на трекинг
2. Запрос статуса заказа → проверка через CRM API → корректный статус
3. Обновление приоритета тикета → валидация → запись в CRM → подтверждение
4. Неизвестный вопрос → эскалация оператору с контекстом
5. CRM API недоступен → fallback на FAQ-only режим

### Non-Functional Tests
1. Нагрузка: 100 одновременных запросов → p95 <5 сек
2. Безопасность: попытка обновить запрещённое поле → отказ и алерт
3. Failover: отключение CRM → переход в FAQ режим <30 сек
4. Аудит: все CRM операции записаны в audit log

## Dependencies
- **External Services**: CRM API (Zendesk/Freshdesk/HubSpot/Jira), база знаний
- **Internal Systems**: OpenAI API или локальная LLM, логирование, мониторинг
- **Libraries/Frameworks**: zenpy/freshdesk-sdk/hubspot-api/jira-python, openai/ollama, pytest

## Revenue Model
### Pricing
- **Setup Fee**: 100,000 - 200,000 руб (разовый)
- **Standard Subscription**: 80,000 - 120,000 руб/мес
  - SLA 8x5
  - До 10,000 запросов/мес
  - Weekly review
- **Premium Subscription**: 150,000 - 200,000 руб/мес
  - SLA 24x7
  - Unlimited запросы
  - Приоритетные фиксы
  - Custom интеграции

### Target Market
- B2B SaaS компании с 5+ операторами поддержки
- E-commerce с высоким объёмом запросов
- Финтех/телеком компании

### Expected ROI for Client
- Экономия: ~200,000 руб/мес на операторах (при deflection 50%)
- Окупаемость: 1-2 месяца
- Улучшение CSAT и времени ответа

## Risks and Mitigation
| Risk | Probability | Impact | Mitigation Strategy |
|------|-------------|--------|---------------------|
| Устаревшая база знаний | High | High | Еженедельный контент-апдейт, владелец контента |
| CRM API лимиты | Medium | High | Кеширование, rate limiting, sandbox тесты |
| Несанкционированные изменения | Low | Critical | Whitelist полей, validation, audit log |
| PII утечка в логах | Medium | Critical | Автоматическое маскирование, access control |
| Низкая точность ответов | Medium | High | Continuous monitoring, feedback loop, retraining |

## References
- `integrations/openai_client.py` - OpenAI integration
- `integrations/ollama_client.py` - Local LLM option
- CRM API документация (выбранной системы)

## Expert Consultation
N/A (SPIDER-SOLO)

## Approval
- [ ] Technical Lead Review
- [ ] Product Owner Review
- [ ] Stakeholder Sign-off

## Notes
- Рекомендуется Approach 3 (Staged Pipeline) для баланса гибкости и контроля
- Начать с 1-2 каналов для пилота (чат + email)
- Пилот на 10-20% трафика первые 2 недели
