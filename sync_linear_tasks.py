#!/usr/bin/env python3
"""
Sync tasks to Linear based on completed work.
Creates tasks from chat analyses and project plans.
"""

import os
import sys
from linear_client import LinearClient, LinearError
from dotenv import load_dotenv

load_dotenv()

TEAM_KEY = (os.getenv("LINEAR_TEAM_KEY") or "").strip()
if not TEAM_KEY:
    print("❌ LINEAR_TEAM_KEY не задан (добавьте в .env).")
    print("   Получите API ключ на: https://linear.app/settings/api")
    sys.exit(1)


# Tasks from the latest work session.
TASKS = {
    "completed": [],
    "project_backlog": [],
}


def create_tasks_in_linear(client: LinearClient, dry_run: bool = False):
    """Create tasks in Linear."""
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
            # Skip completed tasks; they are kept for documentation only.
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

    parser = argparse.ArgumentParser(description="Sync tasks to Linear")
    parser.add_argument(
        "--dry-run", action="store_true", help="Show what would be created without creating tasks"
    )
    parser.add_argument("--category", type=str, help="Create tasks only from the selected category")

    args = parser.parse_args()

    # Filter by category if specified.
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

    # Temporarily replace TASKS for filtering.
    original_tasks = TASKS
    if args.category:
        TASKS.clear()
        TASKS.update(tasks_to_process)

    created, errors, skipped = create_tasks_in_linear(client, dry_run=args.dry_run)

    # Restore the original tasks.
    TASKS.clear()
    TASKS.update(original_tasks)

    if args.dry_run:
        print("\n💡 Запустите без --dry-run для создания задач в Linear")

    sys.exit(0 if errors == 0 else 1)


if __name__ == "__main__":
    main()
