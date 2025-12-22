# -*- coding: utf-8 -*-
"""Behave steps for N8N utilities (backup/restore/cleanup)."""

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

# Safe environment defaults before importing the module
os.environ.setdefault("N8N_URL", "https://example.com")
os.environ.setdefault("BACKUP_DIR", "/tmp/n8n_behave")
os.environ.setdefault("RETENTION_DAYS", "7")
os.environ.setdefault("MAX_BACKUPS", "3")

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

import n8n_backup
from n8n_backup import N8NBackup  # noqa: E402


def _get_loop():
    """Return an event loop, creating a new one if needed."""
    try:
        return asyncio.get_event_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        return loop


@given("the backup directory is a temporary folder")
def step_backup_dir_temp(context):
    """Create a temporary folder for backups and patch the module."""
    tmp_dir = tempfile.mkdtemp(prefix="n8n_backup_")
    context.temp_dirs = getattr(context, "temp_dirs", [])
    context.temp_dirs.append(tmp_dir)

    n8n_backup.BACKUP_DIR = Path(tmp_dir)
    context.backup_dir = n8n_backup.BACKUP_DIR


@given("the retention policy is {retention:d} days and max {max_count:d} backups")
def step_set_policy(context, retention, max_count):
    """Configure retention and limits."""
    n8n_backup.RETENTION_DAYS = retention
    n8n_backup.MAX_BACKUPS = max_count


@given("the API returns {wf_count:d} workflows and {cred_count:d} credential")
def step_mock_api_data(context, wf_count, cred_count):
    """Mock API responses to avoid network calls."""
    workflows = [{"id": i} for i in range(wf_count)]
    credentials = [{"id": i} for i in range(cred_count)]
    context.backup = N8NBackup()
    context.backup.get_workflows = AsyncMock(return_value=workflows)
    context.backup.get_credentials = AsyncMock(return_value=credentials)


@given("backups exist with dates")
def step_existing_backups(context):
    """Create placeholder files with the requested modification times."""
    context.backup = getattr(context, "backup", N8NBackup())
    for row in context.table:
        name = row["name"]
        days_ago = int(row["days_ago"])
        file_path = context.backup_dir / name
        file_path.write_text("", encoding="utf-8")
        target_time = datetime.now() - timedelta(days=days_ago)
        os.utime(file_path, (target_time.timestamp(), target_time.timestamp()))


@when("an N8N backup is created")
def step_create_backup(context):
    """Run backup creation (async)."""
    loop = _get_loop()
    context.archive_path = loop.run_until_complete(context.backup.create_backup())


@when("backup cleanup runs")
def step_cleanup_backups(context):
    """Run cleanup of old/extra archives."""
    loop = _get_loop()
    loop.run_until_complete(context.backup.cleanup_old_backups())


@when('restore is executed from "{filename}"')
def step_restore_backup(context, filename):
    """Attempt to restore the given archive."""
    context.backup = getattr(context, "backup", N8NBackup())
    target_file = context.backup_dir / filename
    loop = _get_loop()
    context.restore_result = loop.run_until_complete(context.backup.restore_backup(target_file))


@then("a backup archive is created")
def step_archive_created(context):
    """Assert that the tar.gz archive exists."""
    assert context.archive_path.exists(), "Backup archive was not created"


@then("the archive contains info for {wf_count:d} workflows and {cred_count:d} credential")
def step_archive_contents(context, wf_count, cred_count):
    """Read files inside the archive and verify metadata."""
    with tarfile.open(context.archive_path, "r:gz") as tar:
        members = tar.getnames()
        info_member = next(m for m in members if m.endswith("backup_info.json"))
        workflows_member = next(m for m in members if m.endswith("workflows.json"))
        credentials_member = next((m for m in members if m.endswith("credentials_meta.json")), None)

        info_data = json.load(tar.extractfile(info_member))
        wf_data = json.load(tar.extractfile(workflows_member))
        cred_data = json.load(tar.extractfile(credentials_member)) if credentials_member else []

    assert info_data["workflows_count"] == wf_count
    assert info_data["credentials_count"] == cred_count
    assert len(wf_data) == wf_count
    assert len(cred_data) == cred_count


@then("{count:d} recent archives remain")
def step_count_backups(context, count):
    """Assert the number of remaining archives after cleanup."""
    backups = list(context.backup_dir.glob("n8n_backup_*.tar.gz"))
    assert len(backups) == count, f"Expected {count} archives, found {len(backups)}"


@then('the archive "{filename}" is removed')
def step_backup_removed(context, filename):
    """Assert the archive was removed."""
    assert not (context.backup_dir / filename).exists(), f"Archive {filename} was not removed"


@then("restore is unsuccessful")
def step_restore_failed(context):
    """Assert restore returned False."""
    assert context.restore_result is False
