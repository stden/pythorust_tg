# Spec: AI-помощник для DevOps-инженера

**ID:** 0004-devops-ai-assistant
**Статус:** Draft
**Автор:** Project Team
**Дата:** 2025-11-23

---

## 1. Проблема

DevOps/Backend специалист, который:
- Управляет несколькими серверами и сервисами (n8n, Caddy, WhatsApp-сервис, MegaCascade)
- Часто сталкивается с падением сервисов (n8n падает регулярно, порты закрываются)
- Тратит время на диагностику повторяющихся проблем
- Работает с клиентами (Bitrix24 интеграции, боты)
- Нуждается в "мощном ИИ-консультанте" для проектов

### Текущие боли (из чата):
- "n8n тоже лежит" — сервисы падают без уведомлений
- "вчера все работало, сегодня извне не доступно" — нет мониторинга
- "перезапуск помог" — типовые решения можно автоматизировать
- Порты 80/443 закрыты, Caddy не получает SSL — повторяющиеся проблемы

---

## 2. Решение

**DevOps AI Assistant** — Telegram-бот + мониторинг + база знаний.

### Ключевые функции:

| Функция | Описание | Приоритет |
|---------|----------|-----------|
| Мониторинг сервисов | Проверка HTTP endpoints каждые 5 мин, алерты в Telegram | P0 |
| Автодиагностика | При падении — проверка логов, портов, предложение решения | P0 |
| База решений | RAG по истории проблем и их решений | P1 |
| Команды управления | `/status`, `/logs`, `/restart` через Telegram | P1 |
| AI-консультант | Ответы на вопросы по DevOps, помощь с кодом | P2 |

---

## 3. Архитектура

```
┌─────────────────────────────────────────────────────────────┐
│                    Telegram Bot (Клава 2.0)                 │
│                         @devops_ai_bot                      │
└─────────────────────┬───────────────────────────────────────┘
                      │
          ┌───────────┴───────────┐
          │                       │
          ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│   Command       │     │   AI Engine     │
│   Handler       │     │   (Claude/      │
│                 │     │    Ollama)      │
│ /status         │     │                 │
│ /logs <service> │     │ + RAG Knowledge │
│ /restart <svc>  │     │   Base          │
│ /help           │     │                 │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
┌─────────────────────────────────────────┐
│           Service Monitor               │
│                                         │
│  Checks every 5 min:                    │
│  - n8n.example-monitoring.test          │
│  - megacascad.example-monitoring.test   │
│  - WhatsApp service (localhost:7373)    │
│  - Caddy status                         │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│           Knowledge Base                │
│                                         │
│  Markdown files:                        │
│  - solutions/n8n-restart.md             │
│  - solutions/caddy-ssl.md               │
│  - solutions/incus-proxy.md             │
│  - solutions/ports-check.md             │
└─────────────────────────────────────────┘
```

---

## 4. Сервисы для мониторинга

| Сервис | URL/Endpoint | Проверка | Действие при падении |
|--------|--------------|----------|---------------------|
| n8n | https://n8n.example-monitoring.test | HTTP 200 | Alert + предложить restart |
| MegaCascade | https://megacascad.example-monitoring.test | HTTP 200 | Alert + диагностика портов |
| WhatsApp | localhost:7373 | HTTP | Alert + проверка chromedriver |
| Caddy | systemctl status | Active | Alert + проверка конфига |

---

## 5. Команды бота

### Основные команды

```
/status              — статус всех сервисов
/status n8n          — статус конкретного сервиса
/logs n8n            — последние 50 строк логов
/logs n8n error      — только ошибки
/restart n8n         — перезапустить сервис (с подтверждением)
/ports               — проверить открытые порты извне
/ssl                 — статус SSL сертификатов
```

### AI-команды

```
/ask <вопрос>        — задать вопрос AI (DevOps контекст)
/fix <проблема>      — получить решение из базы знаний
/explain <команда>   — объяснить команду
```

### Примеры использования

```
Даша: /status
Бот:
✅ n8n — OK (200, 45ms)
✅ MegaCascade — OK (200, 120ms)
❌ WhatsApp — FAIL (connection refused)
✅ Caddy — Active

Даша: /fix whatsapp не работает
Бот:
🔍 Диагностика WhatsApp Service:

1. Проверяю статус сервиса...
   → whatsapp-service.service: inactive (dead)

2. Проверяю логи...
   → ChromeDriver version mismatch

💡 Решение:
Обновить ChromeDriver до версии 120.x

Команда:
```bash
sudo apt update && sudo apt install chromium-chromedriver
sudo systemctl restart whatsapp-service
```

Выполнить автоматически? [Да/Нет]
```

---

## 6. База знаний (Knowledge Base)

### Структура

```
knowledge/
├── services/
│   ├── n8n.md           # Описание, endpoints, типичные проблемы
│   ├── caddy.md         # Конфигурация, SSL, proxy
│   ├── whatsapp.md      # ChromeDriver, авторизация
│   └── megacascade.md   # Bitrix24, порты
├── solutions/
│   ├── n8n-restart.md   # Как перезапустить n8n
│   ├── caddy-ssl.md     # Проблемы с SSL
│   ├── incus-proxy.md   # Проброс портов в Incus
│   ├── ports-debug.md   # Диагностика портов
│   └── chromedriver.md  # Обновление ChromeDriver
├── commands/
│   ├── systemctl.md     # systemctl команды
│   ├── incus.md         # incus/lxc команды
│   └── caddy.md         # caddy команды
└── history/
    └── incidents.md     # История инцидентов и решений
```

### Пример файла решения

```markdown
# solutions/n8n-restart.md

## Проблема
n8n недоступен извне, возвращает 502/503 или не отвечает.

## Диагностика
1. Проверить статус: `systemctl status n8n`
2. Проверить логи: `journalctl -u n8n -n 100`
3. Проверить порт: `curl localhost:5678`

## Решение
```bash
sudo systemctl restart n8n
```

## Если не помогло
1. Проверить Caddy: `systemctl status caddy`
2. Проверить DNS: `dig n8n.example-monitoring.test`
3. Проверить сертификат: `caddy validate`

## Теги
n8n, restart, 502, недоступен
```

---

## 7. Технический стек

| Компонент | Технология | Обоснование |
|-----------|------------|-------------|
| Telegram Bot | Python (aiogram) или Rust (grammers) | Уже есть код |
| AI Engine | Claude API / Ollama (Qwen 7B) | Claude для качества, Ollama для экономии |
| Мониторинг | Python + asyncio + httpx | Простой polling |
| База знаний | Markdown + RAG (embeddings) | Легко редактировать |
| Хранение | SQLite / JSON files | Минимум зависимостей |
| SSH выполнение | paramiko / asyncssh | Для удалённых команд |

---

## 8. Безопасность

### Ограничения доступа
- Whitelist Telegram user IDs (настраивается в config.yml)
- Команды restart/logs требуют подтверждения
- Логирование всех действий

### Ограничения выполнения
- Только предопределённые команды (не произвольный shell)
- Таймаут на выполнение (30 сек)
- Нет sudo без явного разрешения в конфиге

---

## 9. Конфигурация

```yaml
# config.yml
bot:
  token: ${TELEGRAM_BOT_TOKEN}
  allowed_users:
    - ${ALLOWED_USER_1}  # Задать в .env
    - ${ALLOWED_USER_2}

services:
  n8n:
    url: https://n8n.example-monitoring.test
    check_interval: 300  # 5 минут
    restart_command: systemctl restart n8n
    log_command: journalctl -u n8n -n 50

  megacascade:
    url: https://megacascad.example-monitoring.test
    check_interval: 300

  whatsapp:
    url: http://localhost:7373
    check_interval: 60
    restart_command: systemctl restart whatsapp-service

ai:
  provider: claude  # или ollama
  model: claude-sonnet-4-20250514  # или qwen2:7b
  knowledge_path: ./knowledge/

alerts:
  telegram_chat_id: ${ALERT_CHAT_ID}
  cooldown: 300  # Не спамить алертами чаще 5 мин
```

---

## 10. MVP (Phase 1) — 2-3 дня

### Что делаем:
1. ✅ Telegram бот с командами `/status`, `/logs`, `/restart`
2. ✅ Мониторинг 3 сервисов (n8n, MegaCascade, WhatsApp)
3. ✅ Алерты в Telegram при падении
4. ✅ Базовая диагностика (проверка портов, логов)

### Что НЕ делаем в MVP:
- ❌ RAG и база знаний (Phase 2)
- ❌ AI-консультант (Phase 2)
- ❌ Автоматическое исправление (Phase 3)

---

## 11. Метрики успеха

| Метрика | Цель | Как измерять |
|---------|------|--------------|
| Время обнаружения проблемы | < 5 мин | Timestamp алерта vs падения |
| Время восстановления | < 10 мин | От алерта до OK |
| Ложные алерты | < 5% | Алерты / реальные проблемы |
| Использование бота | > 10 команд/день | Счётчик команд |

---

## 12. Риски

| Риск | Вероятность | Влияние | Митигация |
|------|-------------|---------|-----------|
| Бот сам упадёт | Средняя | Высокое | Запуск через systemd, мониторинг извне |
| Ложные алерты | Средняя | Среднее | Retry логика, cooldown |
| SSH доступ скомпрометирован | Низкая | Высокое | Whitelist команд, аудит |

---

## 13. Альтернативы

| Вариант | Плюсы | Минусы |
|---------|-------|--------|
| Uptime Kuma | Готовое решение, веб-UI | Нет AI, нет команд |
| Grafana + Prometheus | Мощный мониторинг | Сложная настройка |
| n8n workflows | Уже есть | Зависит от n8n (который падает) |
| **Свой бот** | Полный контроль, AI | Разработка |

Выбор: **Свой бот** — потому что нужен AI + команды + специфика Дашиной инфраструктуры.

---

## 14. Связанные документы

- `codev/plans/0004-devops-ai-assistant.md` — план разработки
- `codev/specs/0002-helpdesk-autopilot.md` — похожий проект для клиентов
