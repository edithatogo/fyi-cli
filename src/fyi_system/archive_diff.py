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


def _validate_datetime(value: str | None, field_name: str) -> None:
    """Validate an ISO-8601 datetime string or null."""
    if value is None:
        return
    try:
        datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        msg = f"{field_name} must be an ISO-8601 datetime string or null"
        raise ValueError(msg) from exc


def validate_change_entry(entry: object) -> None:
    """Validate one content-addressed change entry."""
    if not isinstance(entry, dict):
        msg = "Change entry must be an object"
        raise TypeError(msg)
    request_id = entry.get("request_id")
    if not isinstance(request_id, int) or request_id < 1:
        msg = "Change entry request_id must be a positive integer"
        raise ValueError(msg)
    digest = entry.get("content_sha256")
    if (
        not isinstance(digest, str)
        or len(digest) != 64
        or any(char not in "0123456789abcdef" for char in digest)
    ):
        msg = "Change entry content_sha256 must be a 64-character lowercase hex string"
        raise ValueError(msg)
    previous = entry.get("previous_sha256")
    if previous is not None and (
        not isinstance(previous, str)
        or len(previous) != 64
        or any(char not in "0123456789abcdef" for char in previous)
    ):
        msg = "Change entry previous_sha256 must be null or a 64-character lowercase hex string"
        raise ValueError(msg)


def validate_changes(changes: dict[str, Any]) -> None:
    """Validate the subset required by changes.schema.json."""
    if not isinstance(changes, dict):
        msg = "Changes must be an object"
        raise TypeError(msg)
    meta = changes.get("meta")
    if not isinstance(meta, dict):
        msg = "Changes must contain object 'meta'"
        raise TypeError(msg)
    if not isinstance(meta.get("version"), str):
        msg = "Changes meta.version must be a string"
        raise TypeError(msg)
    if not isinstance(meta.get("generated_at"), str):
        msg = "Changes meta.generated_at must be a string"
        raise TypeError(msg)
    _validate_datetime(meta.get("generated_at"), "Changes meta.generated_at")
    since = meta.get("since")
    if since is not None and not isinstance(since, str):
        msg = "Changes meta.since must be a string or null"
        raise ValueError(msg)
    _validate_datetime(since, "Changes meta.since")

    for bucket in ("added", "updated", "removed"):
        entries = changes.get(bucket)
        if not isinstance(entries, list):
            msg = f"Changes {bucket} must be an array"
            raise TypeError(msg)
        for entry in entries:
            validate_change_entry(entry)


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
    validate_changes(changes)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(changes, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def read_cursor(path: Path | None) -> str | None:
    """Read diff high-water cursor."""
    if path is None or not path.exists():
        return None
    data = json.loads(path.read_text(encoding="utf-8"))
    value = data.get("last_successful_diff") or data.get("last_successful_sync")
    return str(value) if value else None


def write_cursor(path: Path, generated_at: str) -> None:
    """Write diff high-water cursor."""
    path.parent.mkdir(parents=True, exist_ok=True)
    payload: dict[str, Any] = {}
    if path.exists():
        payload = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(payload, dict):
            payload = {}
    payload["last_successful_diff"] = generated_at
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_diff(
    *,
    derived_dir: Path,
    previous_manifest: Path,
    output_path: Path,
    cursor_path: Path | None = None,
    sync_state_path: Path | None = None,
    advance_cursor: bool = False,
    since: str | None = None,
) -> dict[str, Any]:
    """Run archive diff and optionally advance cursor after successful write."""
    state_path = cursor_path or sync_state_path
    effective_since = since if since is not None else read_cursor(state_path)
    changes = diff_records(
        current=load_current_records(derived_dir),
        previous=load_previous_records(previous_manifest),
        since=effective_since,
    )
    write_changes(output_path, changes)
    if advance_cursor and state_path is not None:
        write_cursor(state_path, str(changes["meta"]["generated_at"]))
    return changes
