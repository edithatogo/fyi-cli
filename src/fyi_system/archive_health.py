"""Machine-readable archive health signals for fyi-archive."""

from __future__ import annotations

import json
import sqlite3
from datetime import UTC, datetime, timedelta
from typing import TYPE_CHECKING, Any

from .db import query_all

if TYPE_CHECKING:
    from pathlib import Path


def load_json(path: Path) -> dict[str, Any]:
    """Load a JSON object, returning an empty object when missing."""
    if not path.exists():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    return data if isinstance(data, dict) else {}


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    """Load JSONL rows, returning an empty list when missing."""
    if not path.exists():
        return []
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            data = json.loads(line)
            if isinstance(data, dict):
                rows.append(data)
    return rows


def request_ids(rows: list[dict[str, Any]]) -> set[int]:
    """Extract request IDs from rows."""
    ids = set()
    for row in rows:
        value = row.get("request_id")
        if isinstance(value, int):
            ids.add(value)
        elif isinstance(value, str) and value.isdigit():
            ids.add(int(value))
    return ids


def captured_ids_from_ledger(rows: list[dict[str, Any]]) -> set[int]:
    """Extract completed capture request IDs from ledger rows."""
    return {
        int(row["request_id"])
        for row in rows
        if row.get("status") == "completed" and str(row.get("request_id", "")).isdigit()
    }


def manifest_record_count(path: Path) -> int:
    """Read manifest record count."""
    manifest = load_json(path)
    return int(manifest.get("meta", {}).get("record_count") or manifest.get("record_count") or 0)


def authorities_with_zero_captures(
    discovered_rows: list[dict[str, Any]],
    captured_request_ids: set[int],
) -> list[str]:
    """Return authorities that have discovered rows but no captured rows."""
    discovered_by_authority: dict[str, set[int]] = {}
    for row in discovered_rows:
        authority = str(row.get("authority") or "")
        if not authority:
            continue
        value = row.get("request_id")
        if isinstance(value, int):
            discovered_by_authority.setdefault(authority, set()).add(value)
    return sorted(
        authority
        for authority, ids in discovered_by_authority.items()
        if not ids.intersection(captured_request_ids)
    )


def directory_bytes(path: Path) -> int:
    """Sum file sizes below a directory."""
    if not path.exists():
        return 0
    return sum(file_path.stat().st_size for file_path in path.rglob("*") if file_path.is_file())


def file_count(path: Path, pattern: str = "*") -> int:
    """Count files below a directory."""
    if not path.exists():
        return 0
    return sum(1 for file_path in path.rglob(pattern) if file_path.is_file())


def latest_completed_at(ledger_rows: list[dict[str, Any]]) -> str | None:
    """Return latest completed_at from completed ledger rows."""
    values = [
        str(row.get("completed_at"))
        for row in ledger_rows
        if row.get("status") == "completed" and row.get("completed_at")
    ]
    return max(values) if values else None


def load_run_log_rows(db_path: Path) -> list[dict[str, Any]]:
    """Load run log rows from the archive state database."""
    if not db_path.exists():
        return []
    try:
        return [
            dict(row)
            for row in query_all(
                db_path,
                "SELECT job_name, status, detail, ran_at, id FROM run_log ORDER BY id DESC",
            )
        ]
    except sqlite3.OperationalError:
        return []


def consecutive_failed_runs(run_log_rows: list[dict[str, Any]]) -> int:
    """Count consecutive failed runs from newest to oldest."""
    count = 0
    for row in run_log_rows:
        if str(row.get("status")) == "ok":
            break
        count += 1
    return count


def parse_utc_timestamp(value: str | None) -> datetime | None:
    """Parse an ISO-8601 timestamp into UTC-aware datetime."""
    if not value:
        return None
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        return parsed.replace(tzinfo=UTC)
    return parsed.astimezone(UTC)


def build_archive_health(
    *,
    discovered_path: Path,
    ledger_path: Path,
    manifest_path: Path,
    sync_state_path: Path,
    db_path: Path,
    attachments_dir: Path,
    wacz_dir: Path,
    missing_sample_size: int = 25,
    stale_after_days: int = 14,
) -> dict[str, Any]:
    """Build deterministic archive health signals."""
    discovered_rows = load_jsonl(discovered_path)
    ledger_rows = load_jsonl(ledger_path)
    sync_state = load_json(sync_state_path)
    run_log_rows = load_run_log_rows(db_path)
    discovered = request_ids(discovered_rows)
    captured = captured_ids_from_ledger(ledger_rows)
    missing = sorted(discovered - captured)
    manifest_count = manifest_record_count(manifest_path)
    last_successful_capture = latest_completed_at(ledger_rows)
    last_successful_diff = sync_state.get("last_successful_diff") or sync_state.get(
        "last_successful_sync"
    )
    report = {
        "schema": "schemas/archive-health.schema.json",
        "freshness": {
            "last_successful_capture": last_successful_capture,
            "last_successful_diff": last_successful_diff,
            "last_successful_sync": sync_state.get("last_successful_sync"),
        },
        "coverage": {
            "discovered_count": len(discovered),
            "captured_count": len(captured),
            "missing_request_count": len(missing),
            "missing_request_ids_sample": missing[:missing_sample_size],
            "authorities_with_zero_captures": authorities_with_zero_captures(
                discovered_rows,
                captured,
            ),
        },
        "counts": {
            "manifest_record_count": manifest_count,
            "attachment_count": file_count(attachments_dir),
            "attachment_bytes": directory_bytes(attachments_dir),
            "wacz_count": file_count(wacz_dir, "*.wacz"),
        },
        "runs": {
            "consecutive_failed_runs": consecutive_failed_runs(run_log_rows),
            "latest_run_status": run_log_rows[0].get("status") if run_log_rows else None,
            "latest_run_at": run_log_rows[0].get("ran_at") if run_log_rows else None,
        },
        "warnings": [],
    }
    if missing:
        report["warnings"].append("coverage_gaps")
    if manifest_count and manifest_count != len(captured):
        report["warnings"].append("manifest_capture_count_mismatch")
    capture_time = parse_utc_timestamp(last_successful_capture)
    if capture_time is None or (datetime.now(UTC) - capture_time) > timedelta(days=stale_after_days):
        report["warnings"].append("stale_data")
    if report["runs"]["consecutive_failed_runs"]:
        report["warnings"].append("consecutive_failed_runs")
    return report


def write_archive_health(path: Path, report: dict[str, Any]) -> None:
    """Write archive health JSON."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
