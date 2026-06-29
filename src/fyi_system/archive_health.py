"""Machine-readable archive health signals for fyi-archive."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

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


def build_archive_health(
    *,
    discovered_path: Path,
    ledger_path: Path,
    manifest_path: Path,
    sync_state_path: Path,
    attachments_dir: Path,
    wacz_dir: Path,
    missing_sample_size: int = 25,
) -> dict[str, Any]:
    """Build deterministic archive health signals."""
    discovered_rows = load_jsonl(discovered_path)
    ledger_rows = load_jsonl(ledger_path)
    sync_state = load_json(sync_state_path)
    discovered = request_ids(discovered_rows)
    captured = captured_ids_from_ledger(ledger_rows)
    missing = sorted(discovered - captured)
    manifest_count = manifest_record_count(manifest_path)
    report = {
        "schema": "schemas/archive-health.schema.json",
        "freshness": {
            "last_successful_capture": latest_completed_at(ledger_rows),
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
        "warnings": [],
    }
    if missing:
        report["warnings"].append("coverage_gaps")
    if manifest_count and manifest_count != len(captured):
        report["warnings"].append("manifest_capture_count_mismatch")
    return report


def write_archive_health(path: Path, report: dict[str, Any]) -> None:
    """Write archive health JSON."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
