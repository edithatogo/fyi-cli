"""Tests for archive content diffing."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

from fyi_system.archive_diff import canonical_json, content_sha256, run_diff
from fyi_system.cli import build_parser

if TYPE_CHECKING:
    from pathlib import Path


def write_current(root: Path, request_id: int, payload: dict) -> None:
    """Write a derived current request."""
    path = root / "Agency" / str(request_id)
    path.mkdir(parents=True, exist_ok=True)
    data = {"id": request_id, "url_title": f"request-{request_id}", **payload}
    path.joinpath("request.json").write_text(json.dumps(data), encoding="utf-8")


def write_previous(path: Path, rows: list[dict]) -> None:
    """Write a previous fyi-archive manifest."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"requests": rows}), encoding="utf-8")


def test_canonical_hash_stable_across_key_order() -> None:
    left = {"b": 2, "a": 1}
    right = {"a": 1, "b": 2}

    assert canonical_json(left) == canonical_json(right)
    assert content_sha256(left) == content_sha256(right)


def test_run_diff_classifies_added_updated_removed(tmp_path: Path) -> None:
    derived = tmp_path / "data/raw/requests"
    previous = tmp_path / "manifests/latest_manifest.json"
    output = tmp_path / "manifests/latest_changes.json"
    write_current(derived, 1, {"title": "changed"})
    write_current(derived, 3, {"title": "new"})
    previous_hash = content_sha256({"id": 1, "title": "old", "url_title": "request-1"})
    removed_hash = content_sha256({"id": 2, "title": "removed", "url_title": "request-2"})
    write_previous(
        previous,
        [
            {"request_id": 1, "url_title": "request-1", "content_sha256": previous_hash},
            {"request_id": 2, "url_title": "request-2", "content_sha256": removed_hash},
        ],
    )

    changes = run_diff(derived_dir=derived, previous_manifest=previous, output_path=output)

    assert [row["request_id"] for row in changes["added"]] == [3]
    assert [row["request_id"] for row in changes["updated"]] == [1]
    assert changes["updated"][0]["previous_sha256"] == previous_hash
    assert [row["request_id"] for row in changes["removed"]] == [2]
    assert output.exists()


def test_run_diff_empty_does_not_advance_cursor_without_flag(tmp_path: Path) -> None:
    derived = tmp_path / "data/raw/requests"
    previous = tmp_path / "manifests/latest_manifest.json"
    output = tmp_path / "manifests/latest_changes.json"
    cursor = tmp_path / "data/_state/diff_state.json"
    payload = {"id": 1, "url_title": "request-1", "title": "same"}
    write_current(derived, 1, {"title": "same"})
    write_previous(
        previous,
        [{"request_id": 1, "url_title": "request-1", "content_sha256": content_sha256(payload)}],
    )

    changes = run_diff(
        derived_dir=derived,
        previous_manifest=previous,
        output_path=output,
        cursor_path=cursor,
    )

    assert changes["added"] == []
    assert changes["updated"] == []
    assert changes["removed"] == []
    assert not cursor.exists()


def test_run_diff_advances_cursor_on_success(tmp_path: Path) -> None:
    cursor = tmp_path / "data/_state/diff_state.json"

    changes = run_diff(
        derived_dir=tmp_path / "missing-current",
        previous_manifest=tmp_path / "missing-previous.json",
        output_path=tmp_path / "latest_changes.json",
        cursor_path=cursor,
        advance_cursor=True,
        since="2026-01-01T00:00:00Z",
    )

    state = json.loads(cursor.read_text(encoding="utf-8"))
    assert state["last_successful_diff"] == changes["meta"]["generated_at"]
    assert changes["meta"]["since"] == "2026-01-01T00:00:00Z"


def test_diff_cli_parses() -> None:
    args = build_parser().parse_args(["diff", "--advance-cursor"])

    assert args.cmd == "diff"
    assert args.advance_cursor is True
