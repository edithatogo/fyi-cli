"""Tests for archive health signals."""

from __future__ import annotations

import json
import sqlite3
from typing import TYPE_CHECKING

from fyi_system.db import init_db
from fyi_system.archive_health import build_archive_health, write_archive_health
from fyi_system.cli import build_parser

if TYPE_CHECKING:
    from pathlib import Path


def write_jsonl(path: Path, rows: list[dict]) -> None:
    """Write test JSONL rows."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")


def write_run_log(db_path: Path, rows: list[tuple[str, str, str, str]]) -> None:
    """Write run log rows for archive-health warnings."""
    init_db(db_path)
    conn = sqlite3.connect(db_path)
    try:
        conn.executemany(
            "INSERT INTO run_log(job_name, status, detail, ran_at) VALUES (?, ?, ?, ?)",
            rows,
        )
        conn.commit()
    finally:
        conn.close()


def test_build_archive_health_reports_missing_ids_and_counts(tmp_path: Path) -> None:
    discovered = tmp_path / "data/_state/discovered_requests.jsonl"
    ledger = tmp_path / "data/_state/ledger.jsonl"
    manifest = tmp_path / "manifests/latest_manifest.json"
    sync_state = tmp_path / "data/_state/sync_state.json"
    db_path = tmp_path / "fyi_system.db"
    attachments = tmp_path / "data/attachments"
    wacz = tmp_path / "dist/site_snapshots"
    write_jsonl(
        discovered,
        [
            {"request_id": 1, "authority": "Agency A"},
            {"request_id": 2, "authority": "Agency B"},
        ],
    )
    write_jsonl(
        ledger,
        [{"request_id": 1, "status": "completed", "completed_at": "2026-01-02T00:00:00Z"}],
    )
    manifest.parent.mkdir(parents=True)
    manifest.write_text(json.dumps({"meta": {"record_count": 1}}), encoding="utf-8")
    sync_state.parent.mkdir(parents=True, exist_ok=True)
    sync_state.write_text(
        json.dumps({"last_successful_sync": "2026-01-03T00:00:00Z"}),
        encoding="utf-8",
    )
    attachments.mkdir(parents=True)
    (attachments / "file.bin").write_bytes(b"abc")
    wacz.mkdir(parents=True)
    (wacz / "snapshot.wacz").write_bytes(b"wacz")

    report = build_archive_health(
        discovered_path=discovered,
        ledger_path=ledger,
        manifest_path=manifest,
        sync_state_path=sync_state,
        db_path=db_path,
        attachments_dir=attachments,
        wacz_dir=wacz,
    )

    assert report["coverage"]["discovered_count"] == 2
    assert report["coverage"]["captured_count"] == 1
    assert report["coverage"]["missing_request_ids_sample"] == [2]
    assert report["coverage"]["authorities_with_zero_captures"] == ["Agency B"]
    assert report["counts"]["attachment_count"] == 1
    assert report["counts"]["attachment_bytes"] == 3
    assert report["counts"]["wacz_count"] == 1
    assert report["freshness"]["last_successful_diff"] == "2026-01-03T00:00:00Z"
    assert report["warnings"] == ["coverage_gaps", "stale_data"]


def test_archive_health_output_is_deterministic(tmp_path: Path) -> None:
    output = tmp_path / "health.json"
    report = build_archive_health(
        discovered_path=tmp_path / "missing-discovered.jsonl",
        ledger_path=tmp_path / "missing-ledger.jsonl",
        manifest_path=tmp_path / "missing-manifest.json",
        sync_state_path=tmp_path / "missing-sync.json",
        db_path=tmp_path / "missing.db",
        attachments_dir=tmp_path / "missing-attachments",
        wacz_dir=tmp_path / "missing-wacz",
    )

    write_archive_health(output, report)
    first = output.read_text(encoding="utf-8")
    write_archive_health(output, report)
    second = output.read_text(encoding="utf-8")

    assert first == second


def test_archive_health_warns_on_consecutive_failed_runs(tmp_path: Path) -> None:
    discovered = tmp_path / "data/_state/discovered_requests.jsonl"
    ledger = tmp_path / "data/_state/ledger.jsonl"
    manifest = tmp_path / "manifests/latest_manifest.json"
    sync_state = tmp_path / "data/_state/sync_state.json"
    db_path = tmp_path / "fyi_system.db"
    attachments = tmp_path / "data/attachments"
    wacz = tmp_path / "dist/site_snapshots"
    write_jsonl(discovered, [{"request_id": 1, "authority": "Agency A"}])
    write_jsonl(
        ledger,
        [{"request_id": 1, "status": "completed", "completed_at": "2026-07-01T00:00:00Z"}],
    )
    manifest.parent.mkdir(parents=True)
    manifest.write_text(json.dumps({"meta": {"record_count": 1}}), encoding="utf-8")
    sync_state.parent.mkdir(parents=True, exist_ok=True)
    sync_state.write_text(
        json.dumps({"last_successful_sync": "2026-07-01T00:00:00Z"}),
        encoding="utf-8",
    )
    write_run_log(
        db_path,
        [
            ("run_cycle", "ok", "done", "2026-07-01T00:00:00Z"),
            ("run_cycle", "failed", "one", "2026-07-01T00:05:00Z"),
            ("run_cycle", "failed", "two", "2026-07-01T00:10:00Z"),
        ],
    )
    attachments.mkdir(parents=True)
    wacz.mkdir(parents=True)

    report = build_archive_health(
        discovered_path=discovered,
        ledger_path=ledger,
        manifest_path=manifest,
        sync_state_path=sync_state,
        db_path=db_path,
        attachments_dir=attachments,
        wacz_dir=wacz,
    )

    assert report["runs"]["consecutive_failed_runs"] == 2
    assert "consecutive_failed_runs" in report["warnings"]


def test_archive_health_cli_parses_paths() -> None:
    args = build_parser().parse_args(
        ["archive-health", "--output", "health.json", "--db", "state.db", "--stale-after-days", "30"]
    )

    assert args.cmd == "archive-health"
    assert args.output == "health.json"
    assert args.db == "state.db"
    assert args.stale_after_days == 30
