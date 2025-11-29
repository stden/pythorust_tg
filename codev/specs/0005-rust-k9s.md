# Spec: TUI-оболочка для Kubernetes (Rust k9s)

**ID:** 0005-rust-k9s  
**Статус:** Draft  
**Дата:** 2025-11-24  
**Автор:** Project Team  

---

## 1. Проблема
- Kubernetes CLI (kubectl) перегружен и требует длинных команд/флагов.
- Инциденты требуют быстрого обзора кластера: поды, логи, эвенты, перезапуски.
- Нужна кроссплатформенная TUI-оболочка без внешних зависимостей (как k9s), но на Rust.
- Требования безопасности: строгий контроль контекста kubeconfig, минимальные RBAC-операции, отсутствие утечек токенов в логи.

## 2. Цели
- Предоставить TUI-оболочку для основных операций kubectl: просмотр ресурсов, логи, describe, exec, port-forward.
- Работать поверх существующего kubeconfig (с поддержкой множественных контекстов/namespace).
- Обеспечить отзывчивый интерфейс (низкая задержка ввода/рендера) и стабильность даже при больших кластерах (1000+ подов).
- Встроить механизмы безопасности: запрет опасных операций по умолчанию, явное подтверждение на delete/exec, маскирование секретов в UI и логах.

## 3. Пользовательские сценарии
- **On-call обзор:** открыть k9s-rs, выбрать контекст, просмотреть namespace, найти под по имени/лейблу, открыть live-логи, фильтровать ошибки.
- **Диагностика пода:** describe пода, проверить эвенты, открыть контейнерные логи, выполнить port-forward на сервис/под.
- **Администрирование:** сменить namespace, пересоздать под (delete + информирование), наблюдать статус rollout для Deployment/StatefulSet/DaemonSet.
- **Триаж:** быстро найти CrashLoopBackOff поды, посмотреть последние эвенты, сравнить рестарты, выгрузить лог-выборку в файл.

## 4. Область (Scope)
- **Включено:** Pods, Deployments, StatefulSets, DaemonSets, Jobs/CronJobs, Services, Ingresses, Nodes, Events, Secrets/ConfigMaps (только просмотр метаданных; содержимое секретов — скрыто).
- **Операции:** list/watch, describe, логи (tail/follow, контейнер по выбору), exec (интерактивный shell с предупреждением), port-forward, delete (с подтверждением), scale (Deployment/StatefulSet) с проверкой RBAC.
- **Навигация:** Vim-подобные/стрелки, fuzzy search, контекстные панели (ресурсы, детали, эвенты, логи).
- **Настройки:** kubeconfig path override, prefer-context/namespace, горячие клавиши, темы оформления.
- **Обсервация:** минимальная телеметрия отключена по умолчанию; структурированные логи в stderr (JSON).
- **Не включено в MVP:** редактирование манифестов, apply/patch, прямое создание ресурсов, UI-консоль для Helm/ArgoCD.
- **Приоритеты (MVP → P1):**

| Функция | Детали | Приоритет |
|---------|--------|-----------|
| List/Watch ресурсов | Таблицы Pods/Deployments/Services/Nodes, быстрые фильтры | P0 |
| Логи | Tail/follow, выбор контейнера, toggle timestamp, grep/regex, экспорт в файл | P0 |
| Describe/Events | Ключевые поля + последние эвенты по ресурсу | P0 |
| Context/Namespace switch | Список из kubeconfig, быстрые хоткеи | P0 |
| Exec | Интерактивный shell с предупреждением, выбор контейнера | P0 |
| Port-forward | Подсказка портов, управление активными туннелями | P0 |
| Delete/Scale | Подтверждение, проверка RBAC, безопасное завершение | P0 |
| Темы/Keybinds | Светлая/тёмная, кастомные биндинги, сохранение в конфиг | P1 |
| Профили | Сохранённые фильтры/namespace/контекст | P1 |
| Quick-dump | Экспорт выбранных логов/эвентов в файл из UI | P1 |

## 5. Функциональные требования
- Поддержка нескольких контекстов и namespaces с быстрым переключением (горячие клавиши).
- Поддержка multi-container подов (выбор контейнера для логов/exec).
- Фильтрация и поиск: по имени, namespace, статусу, лейблам; fuzzy search; фильтр CrashLoopBackOff/Failed/NotReady.
- Логи: tail N, follow, timestamp toggle, grep/regex фильтр, экспорт в файл.
- Exec: интерактивная сессия, явное предупреждение и подтверждение, настройка shell по умолчанию (/bin/sh, /bin/bash).
- Port-forward: подсказка портов из Service/Pod, вывод активных туннелей, безопасное завершение.
- Describe и Events: вывод ключевых полей (conditions, restarts, node, image), последние эвенты.
- RBAC-aware: если операция запрещена, показывать понятное сообщение и не падать.
- Темы: светлая/тёмная, настраиваемые цвета; читабельность при цветовой слепоте.

## 6. Нефункциональные требования
- Производительность: старт < 500 мс на локальном kubeconfig, обновление таблиц ≤ 200 мс при изменениях; плавная прокрутка.
- Масштабируемость: корректная работа на кластерах с 1000+ подов в namespace; эффективный watch (kube-runtime/stream).
- Кроссплатформенность: Linux, macOS, Windows (поддержка ввода/терминала без зависимостей от ncurses).
- Надёжность: UI не падает при ошибках API; сетевые ошибки отображаются и восстанавливаются.
- Безопасность: не логировать токены/секреты; содержимое Secret скрывать (только metadata/name/type); exec/delete требуют подтверждения; конфиг не содержит секретов.

## 7. Архитектура (высокоуровнево)
- **Ядро:** async runtime (tokio) + kube-rs client (watch/list/exec/log/portforward).
- **UI:** ratatui/crossterm (или tui-rs) с компонентами: ResourceTable, DetailPane, LogsPane, StatusBar, CommandPalette.
- **State management:** центральный store (context, namespace, filters, selection), подписки на watch-потоки.
- **Кэш:** in-memory кэш ресурсов с incremental updates (watch), деградация до list при сбоях.
- **Операции:** сервисы для logs, exec, port-forward с контролем времени/отмены (cancellation tokens).
- **Конфиг:** toml/yaml в XDG/OS-специфичном пути (~/.config/k9s-rs/config.{toml,yml}); загрузка kubeconfig из $KUBECONFIG или defaults.
- **Безопасность:** redaction слой для секретов, запрет на запись чувствительных данных в логи/файлы.
- **Компоненты:** ConfigLoader → KubeClientFactory → WatchService (per kind) → Store (Arc<RwLock<>>) → UI Renderer → ActionQueue (операции) → Feedback в Store.
- **Поток данных:**
```
CLI args/env
   ↓
ConfigLoader ──→ KubeClientFactory ──→ Watchers (kube-runtime) ──→ Store/cache
                                                    ↓
                                  UI renderer (ratatui) ← selection/events
                                                    ↓
                                  Action executor (logs/exec/pf/delete/scale)
                                                    ↓
                                  Kube API → updates → watchers/store
```
- **Надёжность:** backoff и повторное подключение watch, circuit-breaker на операции exec/port-forward, таймауты на логи/describe, централизованные cancellation tokens для закрытия долгоживущих потоков.

## 8. UX / Навигация
- Таблица ресурсов с колонками: name, namespace, ready, restarts, age, node (для подов), status.
- Панель деталей: describe summary + последние эвенты.
- Панель логов: tail/follow, выбор контейнера, фильтры.
- Command palette (":") для операций (context, ns, logs, exec, pf, delete, scale).
- Горячие клавиши (предварительно): `c` смена контекста, `n` смена namespace, `/` поиск, `l` логи, `e` exec, `p` port-forward, `d` describe, `x` delete (с подтверждением), `s` scale, `?` помощь, `q` выход.
- Типовые потоки: старт (выбор контекста/namespace) → поиск (`/` или fuzzy) → выбор ресурса → действие (логи/describe/exec/pf). Для многоконтейнерных подов дополнительный слой выбора контейнера. Command palette показывает подсказки и текущий контекст.
- Уведомления об ошибках/запросах подтверждения выводятся в статус-баре и не блокируют основной ввод (кроме delete/exec/scale, где требуется явное подтверждение).

## 9. Конфигурация и безопасность
- Конфиг UI/шорткатов в пользовательском файле, без секретов.
- Kubeconfig читается только из стандартных путей/переменных; не кешируется в собственных файлах.
- Маскирование секретов и токенов в UI, логах и дампах ошибок.
- Подтверждение на exec/delete/scale; опциональный "safe mode" (только read-only).
- Порядок разрешения kubeconfig: CLI `--kubeconfig` → `$KUBECONFIG` → `~/.kube/config`. Падение на отсутствии файла с понятной ошибкой.
- Конфиг (`~/.config/k9s-rs/config.toml|yml`): `theme`, `keybindings`, `default_context`, `default_namespace`, `safe_mode`, `log_follow_lines`, `port_forward.confirm=true`.
- Шифрование/редакция: все секретные поля удаляются из panic/log; ошибки kube-rs с токенами перехватываются и обрезаются.

## 10. Обсервация
- Логи приложения: stdout/stderr, структурированные (JSON) в debug режиме; по умолчанию info.
- Метрики (опционально): время отклика API, количество ошибок watch, число рендеров/сек; выключены по умолчанию.
- Диагностика: команда `--debug` для трассировки API запросов (без токенов).
- Формат логов: `{ts, level, target, context, namespace, resource_kind, msg}`. CLI флаг `--log-format text|json`.
- Метрики: экспорт в Prometheus-подобный формат в локальный файл (`--metrics-file`) или stdout; показатели: latency p95/p99 на list/watch, успешность операций exec/pf/delete, FPS UI.
- Debug dump: `:dump` команда сохраняет snapshot состояния UI/selection без данных пользователя/секретов.

## 11. Тестирование
- Юниты: парсинг kubeconfig, фильтры, форматирование таблиц/времени, редактирование state.
- Интеграционные (с mock kube-apiserver или kube-rs test server): list/watch, логи, exec (mock), port-forward (mock), RBAC ошибки.
- Snapshot-тесты UI-компонентов (ratatui) для стабильности макета.
- Smoke-тест с реальным kind-кластером (CI optional, помечено как slow).
- Test matrix: Linux/macOS/Windows (CI — Linux + macOS); версии Kubernetes 1.24–1.30 (использовать kind образы). Инструменты: `insta` для снапшотов, `assert_cmd` для CLI.
- Фикстуры watch/list: предзаписанные JSON ответы kube-apiserver для офлайн тестов, флаг `--mock` в тестовом бинарнике.

## 12. Релизы и совместимость
- Таргет: Linux/macOS/Windows (x86_64, aarch64).
- Минимальная версия Rust: 1.80+ (stable).
- Kubernetes API: поддержка ≥1.24; graceful downgrade для устаревших полей.
- Сборки: статические бинарники (musl) для Linux, universal2 для macOS, MSVC для Windows. Поставки через GitHub Releases + Homebrew tap + Scoop манифест.
- Версионирование: semver; `0.1.0` — MVP, `0.2.x` — улучшения UX/тёмная тема, `0.3.x` — расширенный фичсет.

## 13. План релизов / Milestones
- **MVP (0.1.0):** список/watch Pods/Deployments/Services, поиск, describe/events, логи (tail/follow), exec с подтверждением, port-forward, delete/scale c RBAC проверкой, конфиг safe mode.
- **Beta (0.2.x):** P1 фичи (темы, keybindings, профили), экспорт логов/эвентов в файл, улучшенная командная палитра, стабильность watch (reconnect/backoff метрики).
- **v1.0:** полная кроссплатформенная поставка, оптимизация производительности (таблицы на 1000+ подов), тонкая настройка фильтров, документация/hotkey overlay, автообновления port-forward статуса.
- **Post-1.0 опции:** чтение/редактирование манифестов (read-only diff), интеграция top/metrics-среза, плагины для кастомных ресурсов (CRD) с schema discovery.

## 14. Открытые вопросы
- Нужно ли редактирование ресурсов (kubectl edit/apply) в последующих релизах?
- Нужна ли встроенная аутентификация OIDC refresh или полагаться на kubeconfig/exec-плагины?
- Нужны ли профили (набор фильтров/namespace) для быстрого переключения?
