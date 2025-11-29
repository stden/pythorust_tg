# Spec: Stories API (публикация и просмотр историй)

**ID:** 0006-stories-api  
**Статус:** Draft  
**Автор:** Codex CLI  
**Дата:** 2025-02-24

---

## 1. Проблема
- В тулките есть поддержка чатов, реакций и медиа, но нет работы с Telegram Stories (создание и просмотр).
- Контент-менеджеры публикуют истории вручную с телефонами и видят только нативную аналитику.
- Аналитика и выгрузка историй для бэкапов/AI-обработки невозможны через текущие CLI/боты.

## 2. Цели и границы
**P0 (минимум):**
- Публикация истории из CLI (Rust) с фото/видео + подписью.
- Настройка приватности (все, контакты, выбранные пользователи/исключения).
- Просмотр списка историй у пользователя/канала, отметка прочтения.
- Загрузка медиа и метаданных историй в локальную папку (`stories/<peer>/`).

**P1 (после P0):**
- Репост истории в другой чат/канал.
- Ответ на историю (reply) с опцией отправки в DM.
- Базовые метрики (просмотры, реакции) в JSON/Markdown.

**Вне скоупа:** хайлайты, монтаж (стикеры/текстовые слои), интерактив (опросы/квизы), маски/AR.

## 3. Пользовательские сценарии
- **Оператор контента:** `cargo run -- stories publish --peer @channel --photo cover.jpg --caption "Запуск"` публикует анонс в канал без телефона.
- **Аналитик:** `cargo run -- stories fetch --peer @channel --limit 10 --download` сохраняет последние истории для отчёта и пересылает метаданные в AI-анализ.
- **Саппорт/бот:** получает историю пользователя, отмечает как просмотренную, при необходимости сохраняет копию в архив.

## 4. Дизайн решения
### Архитектурные компоненты
- **stories service** (`src/stories.rs`): обёртка над grammers `stories.*` методами (send, get, read, archive).
- **CLI слой** (`stories` сабкоманда в `src/main.rs` + `src/commands/stories.rs`): парсинг аргументов, вызов сервисов, вывод.
- **storage**: локальная директория `stories/<peer>/` для медиа и `metadata.json` с id, expires_at, caption, views, reactions.
- **config re-use**: поиск peer по `config.yml`/alias либо raw @username/id, как в других командах.

### CLI интерфейс (P0)
```
cargo run -- stories publish \
  --peer @channel_or_user \
  --photo path.jpg | --video clip.mp4 \
  --caption "Текст" \
  [--ttl-hours 24] \
  [--privacy all|contacts|close-friends|include=id1,id2|exclude=id3,id4]

cargo run -- stories fetch \
  --peer @channel_or_user \
  [--limit 10] \
  [--download] \
  [--mark-read]
```
- `--photo/--video` взаимоисключающие; пока один медиафайл за историю.
- `--privacy` маппится в опции отправки `stories.sendStory` (allow/deny lists).
- `--download` сохраняет файл(ы) в `stories/<peer>/<story_id>.<ext>` и метаданные.
- `--mark-read` вызывает `stories.readStories` после получения.

### Серверный слой (Rust)
- Функция `publish_story(client, peer, media: StoryMedia, options: StoryOptions) -> Result<StoryInfo>`.
- Функция `fetch_stories(client, peer, limit, download: bool, mark_read: bool) -> Result<Vec<StoryInfo>>`.
- `StoryMedia`: `Photo { path } | Video { path, duration, spoiler? }`.
- `StoryOptions`: `ttl_hours`, `caption`, `privacy: PrivacyScope`, `silent: bool` (по умолчанию false).
- `StoryInfo`: id, peer, posted_at, expires_at, caption, views (если доступно), reactions (если доступны), local_paths (optional).
- Обработка rate limit/locks как в других командах (reuse `SessionLock`).

### Хранилище/форматы
- Метаданные: `stories/<peer>/metadata.json` с последним `fetched_at`, массивом `StoryInfo`.
- Медиа: файл оригинального расширения; имя = story_id.
- Логи: `tracing` с ключевыми полями (`peer`, `story_id`, `privacy`).

### Ошибки и граничные случаи
- Ошибка peer: явное сообщение, подсказка проверить `config.yml`.
- Unsupported media: валидация расширений (jpg/png/mp4) до вызова API.
- Большие файлы: проверка размера и early fail с советом (Telegram лимит 2GB, но P0 можно ограничить, например, 200MB).
- TTL: дефолт 24h если не задано.
- Отсутствие прав публиковать в канале: ясно сообщить и завершить.

## 5. Тестирование и приёмка
- Интеграционный smoke: загрузка фото-истории в тестовый приватный канал и чтение обратно (`--limit 1`, `--download`).
- Юнит-моки: валидация парсинга privacy/ttl/аргументов CLI.
- Ручная проверка: открыть Telegram-клиент, убедиться что история опубликована и видна в `fetch`.
- Acceptance P0:
  - Публикация фото-истории с подписью и privacy=contacts завершается без ошибок.
  - `fetch` возвращает список id, времени истечения и сохраняет файл при `--download`.
  - `--mark-read` перестаёт возвращать историю как непросмотренную (если API даёт статус).

## 6. Открытые вопросы
- Нужна ли поддержка текстовых историй (без медиа) в P0?
- Требуется ли отправка одновременно фото+видео (Carousel) или строго один медиафайл?
- Нужен ли fallback в Python (Telethon), если Rust недоступен?
- Какие лимиты по размеру файлов и битрейту видео считаем допустимыми в CLI?

## 7. Риски
- Неполная поддержка Stories в текущей версии `grammers` (нужно проверить методы в 0.8).
- Ограничения Telegram по публикации в публичных каналах без привилегий.
- Потенциальные бан/флад-лимиты при частой публикации/чтении историй.
