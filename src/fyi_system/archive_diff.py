"""Content-addressed archive diff helpers."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

from fyi_system import __version__

if TYPE_CHECKING:
    from pathlib import Path


@dataclass(frozen=True)
class DiffRecord:
    """Request hash record used for archive diffs."""

    request_id: int
    url_title: str
    content_sha256: str


def canonical_json(data: dict[str, Any]) -> str:
    """Serialize JSON deterministically for hashing."""
    return json.dumps(data, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def content_sha256(data: dict[str, Any]) -> str:
    """Return canonical JSON SHA-256."""
    return hashlib.sha256(canonical_json(data).encode("utf-8")).hexdigest()


def load_current_records(derived_dir: Path) -> dict[int, DiffRecord]:
    """Load current request records from the derived capture store."""
    records = {}
    if not derived_dir.exists():
        return records
    for path in sorted(derived_dir.glob("*/*/request.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        request_id = int(data["id"] if "id" in data else data["request_id"])
        url_title = str(data.get("url_title") or f"request-{request_id}")
        records[request_id] = DiffRecord(
            request_id=request_id,
            url_title=url_title,
            content_sha256=str(data.get("content_sha256") or content_sha256(data)),
        )
    return records


def load_previous_records(manifest_path: Path) -> dict[int, DiffRecord]:
    """Load previous records from an fyi-archive manifest."""
    if not manifest_path.exists():
        return {}
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    records = {}
    for row in manifest.get("requests", []):
        request_id = int(row["request_id"])
        records[request_id] = DiffRecord(
            request_id=request_id,
            url_title=str(row.get("url_title") or f"request-{request_id}"),
            content_sha256=str(row["content_sha256"]),
        )
    return records


def change_entry(record: DiffRecord, previous_sha256: str | None = None) -> dict[str, Any]:
    """Build one changes.schema.json-compatible entry."""
    return {
        "request_id": record.request_id,
        "url_title": record.url_title,
        "content_sha256": record.content_sha256,
        "previous_sha256": previous_sha256,
    }


def diff_records(
    *,
    current: dict[int, DiffRecord],
    previous: dict[int, DiffRecord],
    since: str | None,
) -> dict[str, Any]:
    """Compare current and previous record sets."""
    added = []
    updated = []
    removed = []
    for request_id, record in sorted(current.items()):
        old = previous.get(request_id)
        if old is None:
            added.append(change_entry(record))
        elif old.content_sha256 != record.content_sha256:
            updated.append(change_entry(record, previous_sha256=old.content_sha256))
    for request_id, record in sorted(previous.items()):
        if request_id not in current:
            removed.append(change_entry(record, previous_sha256=record.content_sha256))
    return {
        "meta": {
            "generated_at": datetime.now(UTC).isoformat(),
            "since": since,
            "version": __version__,
        },
        "added": added,
        "updated": updated,
        "removed": removed,
    }


def write_changes(path: Path, changes: dict[str, Any]) -> None:
    """Write latest_changes.json."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(changes, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def read_cursor(path: Path | None) -> str | None:
    """Read diff high-water cursor."""
    if path is None or not path.exists():
        return None
    data = json.loads(path.read_text(encoding="utf-8"))
    value = data.get("last_successful_diff")
    return str(value) if value else None


def write_cursor(path: Path, generated_at: str) -> None:
    """Write diff high-water cursor."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps({"last_successful_diff": generated_at}, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def run_diff(
    *,
    derived_dir: Path,
    previous_manifest: Path,
    output_path: Path,
    cursor_path: Path | None = None,
    advance_cursor: bool = False,
    since: str | None = None,
) -> dict[str, Any]:
    """Run archive diff and optionally advance cursor after successful write."""
    effective_since = since if since is not None else read_cursor(cursor_path)
    changes = diff_records(
        current=load_current_records(derived_dir),
        previous=load_previous_records(previous_manifest),
        since=effective_since,
    )
    write_changes(output_path, changes)
    if advance_cursor and cursor_path is not None:
        write_cursor(cursor_path, str(changes["meta"]["generated_at"]))
    return changes
