"""Keep the lossless legacy SQLite fixture tied to the real Python reference."""

from __future__ import annotations

import ast
import hashlib
import json
import sqlite3
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from fyi_system.cli import build_parser
from fyi_system.db import export_tracked_requests, request_timeline

if TYPE_CHECKING:
    from collections.abc import Iterator


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "conformance" / "local-management"
SQL = (FIXTURES / "legacy-v1.sql").read_text(encoding="utf-8")
EXPECTED = json.loads((FIXTURES / "legacy-v1-expected.json").read_text(encoding="utf-8"))
TABLE_QUERIES = {
    "authorities": "SELECT * FROM authorities ORDER BY slug",
    "tracked_requests": "SELECT * FROM tracked_requests ORDER BY id",
    "feed_events": "SELECT * FROM feed_events ORDER BY id",
    "request_snapshots": "SELECT * FROM request_snapshots ORDER BY id",
    "run_log": "SELECT * FROM run_log ORDER BY id",
    "shared_rate_limit_state": "SELECT * FROM shared_rate_limit_state ORDER BY name",
    "shared_rate_limit_events": "SELECT * FROM shared_rate_limit_events ORDER BY id",
    "http_cache": "SELECT * FROM http_cache ORDER BY url",
    "sqlite_sequence": "SELECT * FROM sqlite_sequence ORDER BY name",
}


@pytest.fixture
def legacy_database() -> Iterator[sqlite3.Connection]:
    """Construct the exact legacy schema without any external service."""
    connection = sqlite3.connect(":memory:")
    connection.row_factory = sqlite3.Row
    connection.executescript(SQL)
    try:
        yield connection
    finally:
        connection.close()


def _assert_all_rows(connection: sqlite3.Connection) -> None:
    assert set(TABLE_QUERIES) == set(EXPECTED["tables"])
    for table, query in TABLE_QUERIES.items():
        assert [dict(row) for row in connection.execute(query)] == EXPECTED["tables"][table]


def _persist_fixture(connection: sqlite3.Connection, tmp_path: Path) -> Path:
    database = tmp_path / "legacy.db"
    persisted = sqlite3.connect(database)
    try:
        connection.backup(persisted)
    finally:
        persisted.close()
    return database


def test_legacy_fixture_schema_and_all_original_rows(legacy_database: sqlite3.Connection) -> None:
    """Schema, NULLs, payload bytes and unrelated tables form one frozen contract."""
    module = ast.parse((ROOT / "src" / "fyi_system" / "db.py").read_text(encoding="utf-8"))
    schema = next(
        ast.literal_eval(node.value)
        for node in module.body
        if isinstance(node, ast.Assign)
        and any(isinstance(target, ast.Name) and target.id == "SCHEMA" for target in node.targets)
    )
    assert schema in SQL
    assert hashlib.sha256(schema.encode("utf-8")).hexdigest() == EXPECTED["schema_sha256"]
    assert EXPECTED["schema_version"] == 1
    assert EXPECTED["legacy_database_format"] == "python-unversioned"
    assert legacy_database.execute("PRAGMA user_version").fetchone()[0] == 0
    assert legacy_database.execute("PRAGMA application_id").fetchone()[0] == 0
    assert legacy_database.execute("PRAGMA foreign_keys").fetchone()[0] == 1
    assert legacy_database.execute("PRAGMA foreign_key_check").fetchall() == []
    assert legacy_database.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
    schema_rows = legacy_database.execute(
        "SELECT type, name, tbl_name, sql FROM sqlite_schema ORDER BY type, name",
    )
    assert [dict(row) for row in schema_rows] == EXPECTED["sqlite_schema"]
    for table in TABLE_QUERIES:
        columns = legacy_database.execute("SELECT * FROM pragma_table_info(?)", (table,))
        assert [dict(row) for row in columns] == EXPECTED["table_columns"][table]
        foreign_keys = legacy_database.execute("SELECT * FROM pragma_foreign_key_list(?)", (table,))
        assert [dict(row) for row in foreign_keys] == EXPECTED["foreign_keys"][table]
    _assert_all_rows(legacy_database)


def test_legacy_python_list_and_timeline_outputs(
    legacy_database: sqlite3.Connection,
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Local and remote IDs stay distinct, including IDs outside signed 32-bit range."""
    database = _persist_fixture(legacy_database, tmp_path)
    parser = build_parser()
    arguments = parser.parse_args(["list-requests", "--db", str(database)])
    assert arguments.func(arguments) is None
    captured = capsys.readouterr()
    assert captured.out == EXPECTED["python_outputs"]["list_requests"]["stdout"]
    assert captured.err == ""
    assert "SYNTHETIC-PRIVATE-BODY" not in captured.out

    for identifier, timeline in EXPECTED["python_outputs"]["request_timeline"].items():
        assert request_timeline(database, int(identifier)) == timeline
        arguments = parser.parse_args(["request-timeline", identifier, "--db", str(database)])
        assert arguments.func(arguments) is None
        captured = capsys.readouterr()
        assert captured.out == json.dumps(timeline, indent=2, ensure_ascii=False) + "\n"
        assert captured.err == ""
        assert "SYNTHETIC-PRIVATE-BODY" not in captured.out

    persisted = sqlite3.connect(database)
    persisted.row_factory = sqlite3.Row
    try:
        _assert_all_rows(persisted)
    finally:
        persisted.close()


def test_legacy_python_export_preserves_all_request_fields(
    legacy_database: sqlite3.Connection,
    tmp_path: Path,
) -> None:
    """The explicit private export retains original IDs, NULLs, timestamps and body bytes."""
    database = _persist_fixture(legacy_database, tmp_path)
    output = tmp_path / "requests.json"
    assert export_tracked_requests(database, output) == output
    rows = EXPECTED["python_outputs"]["export_requests"]
    assert output.read_bytes() == json.dumps(rows, indent=2, ensure_ascii=False).encode("utf-8")
    assert json.loads(output.read_text(encoding="utf-8")) == EXPECTED["tables"]["tracked_requests"]


def test_legacy_fixture_next_id_foreign_key_and_rollback(
    legacy_database: sqlite3.Connection,
) -> None:
    """Migration must preserve AUTOINCREMENT state and parent references atomically."""
    with pytest.raises(sqlite3.IntegrityError, match="FOREIGN KEY constraint failed"):
        legacy_database.execute(
            "INSERT INTO tracked_requests(authority_slug, title, body) VALUES (?, ?, ?)",
            ("synthetic-missing", "Synthetic invalid parent", "Synthetic body"),
        )
    legacy_database.rollback()
    _assert_all_rows(legacy_database)

    inserted = legacy_database.execute(
        "INSERT INTO tracked_requests(authority_slug, title, body) VALUES (?, ?, ?)",
        ("synthetic-local", "Synthetic next request", "Synthetic body"),
    )
    assert inserted.lastrowid == EXPECTED["next_autoincrement"]["tracked_requests"]
    legacy_database.rollback()
    _assert_all_rows(legacy_database)
