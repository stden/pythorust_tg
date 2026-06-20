"""Tests for chat_analysis.utils helpers."""

from __future__ import annotations

from datetime import datetime
from pathlib import Path

import pytest

from chat_analysis.utils import VerboseLogger, ensure_dir, load_prompt_template, parse_datetime


def test_ensure_dir_creates_parent_for_file_path(tmp_path: Path) -> None:
    file_path = tmp_path / "out" / "file.txt"
    ensure_dir(file_path)
    assert (tmp_path / "out").is_dir()


def test_ensure_dir_creates_directory_path(tmp_path: Path) -> None:
    dir_path = tmp_path / "nested" / "dir"
    ensure_dir(dir_path)
    assert dir_path.is_dir()


def test_parse_datetime_parses_isoformat() -> None:
    assert parse_datetime("2025-01-02T03:04:05") == datetime(2025, 1, 2, 3, 4, 5)


def test_parse_datetime_returns_default_on_failure() -> None:
    default = datetime(2020, 1, 1, 0, 0, 0)
    assert parse_datetime("not-a-date", default=default) == default


def test_load_prompt_template_reads_from_prompts_dir(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    prompts_dir = tmp_path / "prompts"
    prompts_dir.mkdir()

    (prompts_dir / "test.md").write_text("hello", encoding="utf-8")
    monkeypatch.setenv("PROMPTS_DIR", str(prompts_dir))

    assert load_prompt_template("test") == "hello"


def test_load_prompt_template_raises_when_missing(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    prompts_dir = tmp_path / "prompts"
    prompts_dir.mkdir()
    monkeypatch.setenv("PROMPTS_DIR", str(prompts_dir))

    with pytest.raises(FileNotFoundError):
        load_prompt_template("missing")


def test_verbose_logger_suppresses_output_when_not_verbose(capsys: pytest.CaptureFixture[str]) -> None:
    logger = VerboseLogger(verbose=False)
    logger.log("hidden")
    logger.info("hidden")

    captured = capsys.readouterr()
    assert captured.out == ""


def test_verbose_logger_prints_error_even_when_not_verbose(capsys: pytest.CaptureFixture[str]) -> None:
    logger = VerboseLogger(verbose=False)
    logger.error("boom")

    captured = capsys.readouterr()
    assert "Error: boom" in captured.out


def test_verbose_logger_formats_success_and_warning(capsys: pytest.CaptureFixture[str]) -> None:
    logger = VerboseLogger(verbose=True)
    logger.success("ok")
    logger.warning("careful")

    out = capsys.readouterr().out
    assert "✅ ok" in out
    assert "⚠️" in out
