# -*- coding: utf-8 -*-
"""Шаги Behave для утилит N8N (backup/restore/cleanup)."""

import asyncio
import json
import os
import sys
import tarfile
import tempfile
from datetime import datetime, timedelta
from pathlib import Path
from unittest.mock import AsyncMock

from behave import given, when, then

# Безопасные значения окружения до импорта модуля
os.environ.setdefault("N8N_URL", "https://example.com")
os.environ.setdefault("BACKUP_DIR", "/tmp/n8n_behave")
os.environ.setdefault("RETENTION_DAYS", "7")
os.environ.setdefault("MAX_BACKUPS", "3")

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

import n8n_backup
from n8n_backup import N8NBackup  # noqa: E402


def _get_loop():
    """Возвращает event loop, создавая новый при необходимости."""
    try:
        return asyncio.get_event_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        return loop


@given("бэкап-директория во временной папке")
def step_backup_dir_temp(context):
    """Создаём временную папку для бэкапов и патчим модуль."""
    tmp_dir = tempfile.mkdtemp(prefix="n8n_backup_")
    context.temp_dirs = getattr(context, "temp_dirs", [])
    context.temp_dirs.append(tmp_dir)

    n8n_backup.BACKUP_DIR = Path(tmp_dir)
    context.backup_dir = n8n_backup.BACKUP_DIR


@given('политика хранения: {retention:d} дней и максимум {max_count:d} бэкапов')
def step_set_policy(context, retention, max_count):
    """Настраиваем сроки хранения и лимиты."""
    n8n_backup.RETENTION_DAYS = retention
    n8n_backup.MAX_BACKUPS = max_count


@given('API отдаёт {wf_count:d} воркфлоу и {cred_count:d} credential')
def step_mock_api_data(context, wf_count, cred_count):
    """Мокаем ответы API, чтобы не ходить в сеть."""
    workflows = [{"id": i} for i in range(wf_count)]
    credentials = [{"id": i} for i in range(cred_count)]
    context.backup = N8NBackup()
    context.backup.get_workflows = AsyncMock(return_value=workflows)
    context.backup.get_credentials = AsyncMock(return_value=credentials)


@given("существуют бэкапы с датами")
def step_existing_backups(context):
    """Создаём архивы с нужными датами модификации."""
    context.backup = getattr(context, "backup", N8NBackup())
    for row in context.table:
        name = row["имя"]
        days_ago = int(row["дней_назад"])
        file_path = context.backup_dir / name
        file_path.write_text("", encoding="utf-8")
        target_time = datetime.now() - timedelta(days=days_ago)
        os.utime(file_path, (target_time.timestamp(), target_time.timestamp()))


@when("создаётся бэкап N8N")
def step_create_backup(context):
    """Запускаем создание бэкапа (async)."""
    loop = _get_loop()
    context.archive_path = loop.run_until_complete(context.backup.create_backup())


@when("запускается очистка бэкапов")
def step_cleanup_backups(context):
    """Запускаем очистку старых/лишних архивов."""
    loop = _get_loop()
    loop.run_until_complete(context.backup.cleanup_old_backups())


@when('выполняется восстановление из "{filename}"')
def step_restore_backup(context, filename):
    """Пытаемся восстановить указанный архив."""
    context.backup = getattr(context, "backup", N8NBackup())
    target_file = context.backup_dir / filename
    loop = _get_loop()
    context.restore_result = loop.run_until_complete(
        context.backup.restore_backup(target_file)
    )


@then("создан архив бэкапа")
def step_archive_created(context):
    """Проверяем наличие tar.gz архива."""
    assert context.archive_path.exists(), "Архив бэкапа не создан"


@then('архив содержит информацию о {wf_count:d} воркфлоу и {cred_count:d} credential')
def step_archive_contents(context, wf_count, cred_count):
    """Читаем файлы внутри архива и проверяем метаданные."""
    with tarfile.open(context.archive_path, "r:gz") as tar:
        members = tar.getnames()
        info_member = next(m for m in members if m.endswith("backup_info.json"))
        workflows_member = next(m for m in members if m.endswith("workflows.json"))
        credentials_member = next(
            (m for m in members if m.endswith("credentials_meta.json")), None
        )

        info_data = json.load(tar.extractfile(info_member))
        wf_data = json.load(tar.extractfile(workflows_member))
        cred_data = json.load(tar.extractfile(credentials_member)) if credentials_member else []

    assert info_data["workflows_count"] == wf_count
    assert info_data["credentials_count"] == cred_count
    assert len(wf_data) == wf_count
    assert len(cred_data) == cred_count


@then("остаётся {count:d} свежих архива")
def step_count_backups(context, count):
    """Проверяем количество архивов после очистки."""
    backups = list(context.backup_dir.glob("n8n_backup_*.tar.gz"))
    assert len(backups) == count, f"Ожидалось {count} архивов, найдено {len(backups)}"


@then('архив "{filename}" удалён')
def step_backup_removed(context, filename):
    """Убеждаемся, что старый архив удалён."""
    assert not (context.backup_dir / filename).exists(), \
        f"Архив {filename} не был удалён"


@then("восстановление неуспешно")
def step_restore_failed(context):
    """Проверяем, что restore вернул False."""
    assert context.restore_result is False
