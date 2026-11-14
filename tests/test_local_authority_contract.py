"""Freeze the real Python local authority behavior for Rust compatibility fixtures."""

from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest

from fyi_system.cli import build_parser
from fyi_system.db import init_db, query_all

if TYPE_CHECKING:
    from collections.abc import Mapping


FIXTURE_PATH = (
    Path(__file__).resolve().parents[1]
    / "conformance"
    / "local-management"
    / "authorities-v1-python.json"
)
CONTRACT = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))


def _assert_reference_case(
    case: Mapping[str, Any],
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    database = tmp_path / "local.db"
    catalogue = tmp_path / "authorities.csv"
    catalogue.write_bytes(case["csv"].encode("utf-8"))
    init_db(database)
    parser = build_parser()

    # Repeated imports preserve the same rows but count every valid CSV row.
    for _ in range(2):
        arguments = parser.parse_args(
            ["import-authorities", str(catalogue), "--db", str(database)],
        )
        arguments.func(arguments)
        captured = capsys.readouterr()
        assert captured.out == f"Imported {case['imported_count']} authorities\n"
        assert captured.err == ""
        rows = query_all(
            database,
            "SELECT slug, name, url FROM authorities ORDER BY name, slug",
        )
        assert [dict(row) for row in rows] == case["rows"]

        arguments = parser.parse_args(["list-authorities", "--db", str(database)])
        arguments.func(arguments)
        captured = capsys.readouterr()
        assert captured.out == case["stdout"]
        assert captured.err == ""


@pytest.mark.parametrize(
    "case",
    CONTRACT["valid_cases"],
    ids=lambda case: case["id"],
)
def test_local_authority_python_compatibility_fixture(
    case: Mapping[str, Any],
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Exercise local CLI handlers, CSV decoding, persistence and text output."""
    assert CONTRACT["schema_version"] == 1
    _assert_reference_case(case, tmp_path, capsys)


@pytest.mark.parametrize(
    "case",
    CONTRACT["legacy_differences"],
    ids=lambda case: case["id"],
)
def test_local_authority_legacy_differences_are_explicit(
    case: Mapping[str, Any],
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Skipped invalid rows and implicit tie ordering are not Rust requirements."""
    assert case["rust_behavior"]
    _assert_reference_case(case, tmp_path, capsys)
