#!/usr/bin/env python3
"""
Синхронизация задач в Linear на основе завершённой работы.
Создаёт задачи из анализов чатов и планов проектов.
"""

import os
import sys

from dotenv import load_dotenv

from linear_client import LinearClient, LinearError

load_dotenv()

TEAM_KEY = (os.getenv("LINEAR_TEAM_KEY") or "").strip()
if not TEAM_KEY:
    print("❌ LINEAR_TEAM_KEY не задан (добавьте в .env).")
    print("   Получите API ключ на: https://linear.app/settings/api")
    sys.exit(1)


# Задачи из последней сессии работы
TASKS = {
    "completed": [
        # Завершённые задачи (для документации)
        (
            "✅ Анализ чата Хара",
            "Проанализирован чат духовного сообщества Хара (369 сообщений)\n\n"
            "Результат: создан промпт для дизайна сайта\n"
            "Файл: prompts/hara_website_design.md\n\n"
            "Включает:\n"
            "- Цветовую палитру (золотой, фиолетовый, изумрудный)\n"
            "- Типографику (Playfair Display, Inter/Raleway)\n"
            "- Структуру из 6 страниц\n"
            "- Интерактивные элементы (гадание, калькулятор нумерологии)\n"
            "- 10 детальных секций",
            4,
        ),
        (
            "✅ Анализ потребностей вайбкодеров",
            "Проанализирован чат вайбкодеры (475 сообщений за 2 дня)\n\n"
            "Результат: выявлено 12 продуктовых возможностей\n"
            "Файл: analysis_results/vibecoders_needs_analysis.md\n\n"
            "Топ-3 идеи с высоким потенциалом:\n"
            "1. Figma Pixel-Perfect Plugin ($15-50/мес)\n"
            "2. VoiceGPT Pro с интернетом ($20-40/мес)\n"
            "3. CharacterHub - open source character AI\n\n"
            "Прогноз: $1,750-13,500 MRR за 6 месяцев",
            4,
        ),
        (
            "✅ План Figma Pixel Perfect Plugin",
            "Создан детальный план разработки плагина пиксель-перфект сравнения\n\n"
            "Результат: 8-недельный план реализации\n"
            "Файл: codev/plans/figma-pixel-perfect-plugin.md\n\n"
            "Содержит:\n"
            "- Техническую архитектуру (VS Code Extension + Backend)\n"
            "- 4 фазы разработки (110-160 часов)\n"
            "- Примеры кода на TypeScript\n"
            "- Стратегию монетизации ($15 Pro, $50 Enterprise)\n"
            "- Маркетинговый план\n"
            "- Прогноз выручки: $1,750-13,500 MRR за 6 месяцев",
            4,
        ),
    ],
    "figma_plugin": [
        # Phase 1: MVP (Weeks 1-3)
        (
            "Figma Plugin: Настроить VS Code Extension skeleton",
            "Инициализировать VS Code extension проект\n\n"
            "Задачи:\n"
            "- Установить yo code генератор\n"
            "- Создать базовую структуру расширения\n"
            "- Настроить TypeScript конфигурацию\n"
            "- Создать WebView UI на React\n"
            "- Добавить команду в command palette\n\n"
            "Время: 8-10 часов\n"
            "Референс: codev/plans/figma-pixel-perfect-plugin.md (строки 160-189)",
            1,
        ),
        (
            "Figma Plugin: Screenshot capture через Puppeteer",
            "Реализовать модуль захвата скриншотов браузера\n\n"
            "Задачи:\n"
            "- Интегрировать Puppeteer\n"
            "- Реализовать captureScreenshot функцию\n"
            "- Добавить поддержку viewport размеров\n"
            "- Обработка ошибок\n"
            "- Unit-тесты\n\n"
            "Время: 12-15 часов\n"
            "Код: codev/plans/figma-pixel-perfect-plugin.md (строки 191-215)",
            1,
        ),
        (
            "Figma Plugin: Интеграция Figma API",
            "Подключить Figma REST API для загрузки макетов\n\n"
            "Задачи:\n"
            "- Настроить Figma API клиент\n"
            "- Реализовать fetchFigmaFrame функцию\n"
            "- Обработка Personal Access Token\n"
            "- Кэширование изображений\n"
            "- Error handling\n\n"
            "Время: 10-12 часов\n"
            "Код: codev/plans/figma-pixel-perfect-plugin.md (строки 217-239)",
            2,
        ),
        (
            "Figma Plugin: Image comparison engine",
            "Создать движок сравнения изображений с pixelmatch\n\n"
            "Задачи:\n"
            "- Интегрировать библиотеку pixelmatch\n"
            "- Реализовать compareImages функцию\n"
            "- Генерация diff изображения\n"
            "- Расчёт процента различий\n"
            "- Тесты на разных изображениях\n\n"
            "Время: 15-18 часов\n"
            "Точность: >95%\n"
            "Код: codev/plans/figma-pixel-perfect-plugin.md (строки 241-275)",
            2,
        ),
        (
            "Figma Plugin: UI для отображения результатов",
            "Разработать интерфейс для показа diff результатов\n\n"
            "Задачи:\n"
            "- React компоненты для WebView\n"
            "- Overlay для сравнения изображений\n"
            "- Slider для overlay opacity\n"
            "- Список различий с координатами\n"
            "- Экспорт результатов\n\n"
            "Время: 15-20 часов\n"
            "UX требование: понятность без документации",
            2,
        ),
        # Phase 2: AI CSS Fixes (Weeks 4-5)
        (
            "Figma Plugin: AI CSS фиксы через Claude API",
            "Интегрировать Claude API для генерации CSS исправлений\n\n"
            "Задачи:\n"
            "- Настроить Claude API клиент\n"
            "- Разработать промпт для генерации CSS\n"
            "- Парсинг HTML/CSS текущего элемента\n"
            "- Валидация сгенерированного CSS\n"
            "- A/B тесты качества фиксов\n\n"
            "Время: 20-25 часов\n"
            "Цель: >80% точность фиксов\n"
            "Код: codev/plans/figma-pixel-perfect-plugin.md (строки 285-322)",
            2,
        ),
        (
            "Figma Plugin: Auto-apply CSS изменений",
            "Реализовать автоматическое применение CSS фиксов к коду\n\n"
            "Задачи:\n"
            "- Парсинг текущего CSS файла\n"
            "- Поиск CSS селектора в коде\n"
            "- Применение изменений через VS Code API\n"
            "- Preview до применения\n"
            "- Откат изменений (undo)\n\n"
            "Время: 15-18 часов\n"
            "Безопасность: требуется подтверждение пользователя",
            2,
        ),
        # Phase 3: Backend & Auth (Weeks 5-6)
        (
            "Figma Plugin: Backend API (Node.js + Express)",
            "Разработать backend сервис для обработки запросов\n\n"
            "Задачи:\n"
            "- Настроить Express/Fastify сервер\n"
            "- API endpoints для comparison\n"
            "- Rate limiting (10 req/day free, unlimited Pro)\n"
            "- Middleware для auth\n"
            "- Docker контейнеризация\n\n"
            "Время: 12-15 часов\n"
            "Deployment: Vercel/Railway",
            3,
        ),
        (
            "Figma Plugin: Supabase Auth + Stripe Billing",
            "Интегрировать аутентификацию и систему подписок\n\n"
            "Задачи:\n"
            "- Настроить Supabase проект\n"
            "- Auth flow в VS Code extension\n"
            "- Stripe checkout интеграция\n"
            "- Webhook для subscription updates\n"
            "- Database schema для users/subscriptions\n\n"
            "Время: 18-22 часа\n"
            "Планы: Free (10/мес), Pro ($15), Enterprise ($50)",
            3,
        ),
        # Phase 4: Testing & Launch (Weeks 7-8)
        (
            "Figma Plugin: End-to-End тестирование",
            "Полное тестирование extension + backend\n\n"
            "Задачи:\n"
            "- Unit тесты (Jest)\n"
            "- Integration тесты\n"
            "- E2E тесты с Playwright\n"
            "- Performance тесты\n"
            "- Баг-фиксы\n\n"
            "Время: 10-12 часов\n"
            "Coverage: >80%",
            3,
        ),
        (
            "Figma Plugin: Документация и Marketing",
            "Подготовить материалы для запуска\n\n"
            "Задачи:\n"
            "- README с инструкциями\n"
            "- Landing page\n"
            "- Demo видео (YouTube)\n"
            "- Product Hunt подготовка\n"
            "- Twitter/LinkedIn анонс\n\n"
            "Время: 15-20 часов\n"
            "Цель: 1000 установок за первый месяц",
            3,
        ),
        (
            "Figma Plugin: Публикация в VS Code Marketplace",
            "Опубликовать расширение в официальном магазине\n\n"
            "Задачи:\n"
            "- Регистрация Publisher аккаунта\n"
            "- Подготовка package.json\n"
            "- Иконки и скриншоты\n"
            "- vsce publish\n"
            "- Мониторинг первых отзывов\n\n"
            "Время: 6-8 часов\n"
            "Launch date: через 8 недель от старта",
            3,
        ),
    ],
    "hara_website": [
        (
            "Дизайн сайта Хара: Выбор дизайнера/агентства",
            "Найти исполнителя для реализации дизайна\n\n"
            "Опции:\n"
            "1. Freelance дизайнер (Behance, Dribbble)\n"
            "2. Дизайн-студия специализирующаяся на wellness\n"
            "3. AI-генерация через Midjourney + доработка\n"
            "4. Конкурс на 99designs\n\n"
            "Бюджет: $1,500-5,000\n"
            "Референсы: Gaia.com, The Wild Unknown\n"
            "Промпт: prompts/hara_website_design.md",
            2,
        ),
        (
            "Дизайн сайта Хара: Колода карт ХАРА продуктовая страница",
            "Создать продуктовую страницу для предзаказа колоды\n\n"
            "Требования:\n"
            "- Галерея примеров карт\n"
            "- Описание значений карт\n"
            "- Форма предзаказа\n"
            "- Countdown таймер до 10.12\n"
            "- Интеграция оплаты\n\n"
            "Дедлайн: до 10.12 (выпуск колоды)\n"
            "Цена колоды: TBD",
            1,
        ),
        (
            "Дизайн сайта Хара: Интеграция календаря ретритов",
            "Добавить расписание онлайн-ретритов и эфиров\n\n"
            "Функционал:\n"
            "- Календарь событий\n"
            "- Регистрация на ретрит\n"
            "- Telegram-интеграция для напоминаний\n"
            "- Профили ведущих (Ирина, Инна, Anna)\n\n"
            "Технологии: Google Calendar API / Calendly",
            3,
        ),
    ],
    "voice_gpt_pro": [
        (
            "VoiceGPT Pro: Research конкурентов",
            "Исследовать существующие решения голосовых AI\n\n"
            "Конкуренты:\n"
            "- ChatGPT Voice Mode\n"
            "- Claude mobile voice\n"
            "- Perplexity Voice\n"
            "- Google Assistant with Gemini\n\n"
            "Анализ:\n"
            "- Функционал и ограничения\n"
            "- Ценообразование\n"
            "- Отзывы пользователей\n"
            "- Gap analysis\n\n"
            "Время: 4-6 часов",
            3,
        ),
        (
            "VoiceGPT Pro: Техническая спецификация",
            "Написать детальный tech spec для голосового AI\n\n"
            "Секции:\n"
            "- Архитектура (STT + LLM + TTS + Web Search)\n"
            "- Выбор моделей (Whisper, GPT-4o, ElevenLabs)\n"
            "- Инфраструктура и costs\n"
            "- Mobile app vs Web app\n"
            "- Стратегия монетизации\n\n"
            "Формат: codev/specs/voice-gpt-pro.md\n"
            "Время: 8-10 часов",
            3,
        ),
    ],
    "character_hub": [
        (
            "CharacterHub: Анализ рынка Character.AI",
            "Исследовать рынок character AI и возможности\n\n"
            "Анализ:\n"
            "- Character.AI business model\n"
            "- Размер рынка и прогнозы\n"
            "- Open source альтернативы\n"
            "- Grok Aurora функционал\n"
            "- Потребности десктоп-версии для стримеров\n\n"
            "Вывод из анализа вайбкодеров:\n"
            "> 'Мне как идея очень зашло! Но не готов ради этого оплачивать Грок)'\n\n"
            "Время: 6-8 часов",
            4,
        ),
    ],
}


def create_tasks_in_linear(client: LinearClient, dry_run: bool = False):
    """Создать задачи в Linear."""
    created = 0
    errors = 0
    skipped = 0

    print("=" * 60)
    print("🔄 Синхронизация задач с Linear")
    print("=" * 60)
    print(f"Team: {TEAM_KEY}")
    print(f"Dry run: {dry_run}")
    print()

    for category, tasks in TASKS.items():
        print(f"\n📁 Категория: {category}")
        print("-" * 60)

        for title, description, priority in tasks:
            # Пропустить завершённые задачи (только для документации)
            if category == "completed":
                print(f"  ⏭️  {title} (архивная)")
                skipped += 1
                continue

            if dry_run:
                print(f"  🔍 [DRY RUN] {title}")
                print(f"     Priority: {priority}")
                print(f"     Description preview: {description[:100]}...")
                continue

            try:
                issue = client.create_issue(
                    team_key=TEAM_KEY,
                    title=title,
                    description=f"{description}\n\n---\nКатегория: {category}\nСоздано автоматически из sync_linear_tasks.py",
                    priority=priority,
                )
                print(f"  ✅ {issue['identifier']}: {title}")
                print(f"     URL: {issue['url']}")
                created += 1
            except LinearError as e:
                print(f"  ❌ {title}")
                print(f"     Ошибка: {e}")
                errors += 1

    print(f"\n{'=' * 60}")
    print("📊 Результат:")
    print(f"   ✅ Создано: {created} задач")
    print(f"   ❌ Ошибок: {errors}")
    print(f"   ⏭️  Пропущено: {skipped} (архивные)")
    print("=" * 60)

    return created, errors, skipped


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Синхронизация задач в Linear")
    parser.add_argument(
        "--dry-run", action="store_true", help="Показать что будет создано без фактического создания задач"
    )
    parser.add_argument("--category", type=str, help="Создать задачи только из указанной категории")

    args = parser.parse_args()

    # Фильтр по категории если указано
    tasks_to_process = TASKS
    if args.category:
        if args.category not in TASKS:
            print(f"❌ Неизвестная категория: {args.category}")
            print(f"   Доступные: {', '.join(TASKS.keys())}")
            sys.exit(1)

        tasks_to_process = {args.category: TASKS[args.category]}

    try:
        client = LinearClient()
    except LinearError as e:
        print(f"❌ Ошибка инициализации Linear: {e}")
        print("\nДобавьте LINEAR_API_KEY в .env файл:")
        print("1. Перейдите на https://linear.app/settings/api")
        print("2. Создайте Personal API Key")
        print("3. Добавьте в .env: LINEAR_API_KEY=lin_api_xxx")
        sys.exit(1)

    # Временно заменяем TASKS для фильтрации
    original_tasks = TASKS
    if args.category:
        TASKS.clear()
        TASKS.update(tasks_to_process)

    created, errors, skipped = create_tasks_in_linear(client, dry_run=args.dry_run)

    # Восстанавливаем оригинальные задачи
    TASKS.clear()
    TASKS.update(original_tasks)

    if args.dry_run:
        print("\n💡 Запустите без --dry-run для создания задач в Linear")

    sys.exit(0 if errors == 0 else 1)


if __name__ == "__main__":
    main()
