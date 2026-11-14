"""Characterize the retained Python lifecycle and explicit native differences."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import sysconfig
from datetime import datetime
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest

from fyi_system.cli import build_parser
from fyi_system.db import connect, init_db, query_all
from fyi_system.importers import import_authorities_rows

if TYPE_CHECKING:
    from collections.abc import Mapping


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_PATH = ROOT / "conformance" / "local-management" / "requests-v1-python.json"
CONTRACT = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))


def _database(tmp_path: Path) -> Path:
    database = tmp_path / "local.db"
    init_db(database)
    assert import_authorities_rows([{"slug": "alpha", "name": "Alpha Authority"}], database) == 1
    return database


def _invoke(arguments: list[str], database: Path) -> None:
    parsed = build_parser().parse_args([*arguments, "--db", str(database)])
    assert parsed.func(parsed) is None


def _python_runtime() -> str:
    """Launch this Python environment, even when a profiler replaces sys.executable."""
    if sys.prefix != sys.base_prefix:
        # Keep the virtualenv launcher path: resolving its symlink loses installed extras.
        name = "python.exe" if os.name == "nt" else "python"
        executable = Path(sysconfig.get_path("scripts")) / name
    else:
        executable = Path(getattr(sys, "_base_executable", sys.executable))
    assert executable.is_file()
    return str(executable)


@pytest.mark.parametrize("case", CONTRACT["valid_cases"], ids=lambda case: case["id"])
def test_python_registration_fields_and_repeated_local_identity(
    case: Mapping[str, Any],
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Preserve nonblank content and keep local IDs separate from optional remote IDs."""
    assert CONTRACT["schema_version"] == 1
    database = _database(tmp_path)
    for identifier in (1, 2):
        _invoke(case["arguments"], database)
        captured = capsys.readouterr()
        assert captured.out == case["stdout"]
        assert captured.err == ""
        row = dict(
            query_all(database, "SELECT * FROM tracked_requests WHERE id=?", (identifier,))[0],
        )
        created_at = row.pop("created_at")
        assert created_at == row.pop("updated_at")
        assert datetime.fromisoformat(created_at)
        expected = {**case["expected_row_without_timestamps"], "id": identifier}
        assert row == expected
    assert len(query_all(database, "SELECT id FROM tracked_requests")) == 2
    _invoke(["list-requests"], database)
    captured = capsys.readouterr()
    first = case["list_stdout"]
    assert captured.out == first + first.replace("1\t", "2\t", 1)
    assert captured.err == ""
    assert "SYNTHETIC-PRIVATE-BODY" not in captured.out


@pytest.mark.parametrize(
    "case",
    CONTRACT["legacy_registration_differences"],
    ids=lambda case: case["id"],
)
def test_python_registration_validation_differences_are_frozen(
    case: Mapping[str, Any],
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Legacy acceptance does not require Rust to preserve invalid inputs or orphan rows."""
    database = _database(tmp_path)
    assert case["rust_behavior"]
    _invoke(case["arguments"], database)
    captured = capsys.readouterr()
    assert captured.out == "Registered request\n"
    assert captured.err == ""
    rows = query_all(database, "SELECT * FROM tracked_requests")
    assert len(rows) == 1
    assert rows[0][case["field"]] == case["expected_value"]


@pytest.mark.parametrize(
    "case",
    CONTRACT["status_reference"]["cases"],
    ids=lambda case: case["id"],
)
def test_python_status_timestamp_and_timeline_differences(
    case: Mapping[str, Any],
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """Freeze the clock while exercising the real legacy status and timeline handlers."""
    database = _database(tmp_path)
    _invoke(CONTRACT["valid_cases"][0]["arguments"], database)
    capsys.readouterr()
    seed = CONTRACT["status_reference"]["seed_timestamp"]
    connection = connect(database)
    try:
        connection.execute(
            "UPDATE tracked_requests SET created_at=?, updated_at=? WHERE id=1",
            (seed, seed),
        )
        connection.commit()
    finally:
        connection.close()
    timestamps = iter(case["timestamps"])
    monkeypatch.setattr("fyi_system.db._utc_now_iso", lambda: next(timestamps))
    assert case["rust_behavior"]
    for status in case["statuses"]:
        _invoke(["set-status", "1", status], database)
        captured = capsys.readouterr()
        assert captured.out == f"Updated request 1 to {status}\n"
        assert captured.err == ""
    row = query_all(database, "SELECT status, created_at, updated_at FROM tracked_requests")[0]
    assert row["created_at"] == seed
    assert row["status"] == case["statuses"][-1]
    assert row["updated_at"] == case["timestamps"][-1]
    _invoke(["request-timeline", "1"], database)
    captured = capsys.readouterr()
    assert captured.out == json.dumps(case["timeline"], indent=2, ensure_ascii=False) + "\n"
    assert captured.err == ""
    assert "SYNTHETIC-PRIVATE-BODY" not in captured.out


@pytest.mark.parametrize("case", CONTRACT["exit_cases"], ids=lambda case: case["id"])
def test_python_lifecycle_exit_codes_are_observed_in_real_processes(
    case: Mapping[str, Any],
    tmp_path: Path,
) -> None:
    """Distinguish parser errors, runtime errors and legacy missing-target success."""
    database = _database(tmp_path)
    assert case["rust_behavior"]
    result = subprocess.run(  # noqa: S603 - fixed interpreter and checked-in offline command fixtures
        [_python_runtime(), "-m", "fyi_system.cli", *case["arguments"], "--db", str(database)],
        cwd=tmp_path,
        env={**os.environ, "PYTHONPATH": str(ROOT / "src")},
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    assert result.returncode == case["exit_code"]
    assert result.stdout == case["stdout"]
    if case["stderr_contains"]:
        assert case["stderr_contains"] in result.stderr
    else:
        assert result.stderr == ""
    assert query_all(database, "SELECT * FROM tracked_requests") == []
