"""Versioned, public-safe process-event export for fyi-archive consumers."""
# The exporter intentionally keeps validation and projection together so the
# public boundary is enforced at the serialization point.
# ruff: noqa: C901,EM101,EM102,PLC0415,PLW0108,TC003,TRY003,TRY004

from __future__ import annotations

import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

SCHEMA_VERSION = "1.0.0"
EXCLUDED_FIELDS = frozenset({"title", "request_title", "body", "detail", "excerpt", "requester"})


def _canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode(
        "utf-8",
    )


def _sha256(value: Any) -> str:
    return hashlib.sha256(_canonical(value)).hexdigest()


def _utc(value: Any) -> tuple[str | None, str]:
    if not isinstance(value, str) or not value:
        return None, "missing"
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None, "invalid"
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=UTC)
    return parsed.astimezone(UTC).isoformat().replace("+00:00", "Z"), "valid"


def _request_id(row: dict[str, Any]) -> int:
    value = row.get("request_id", row.get("id"))
    if not isinstance(value, int) or value < 1:
        try:
            value = int(value)
        except (TypeError, ValueError) as exc:
            raise ValueError("request record must contain a positive integer id") from exc
    if value < 1:
        raise ValueError("request record must contain a positive integer id")
    return value


def _events(row: dict[str, Any]) -> list[dict[str, Any]]:
    """Use the source timeline when present; otherwise use the existing extractor."""
    direct = row.get("info_request_events")
    if isinstance(direct, list):
        return [item for item in direct if isinstance(item, dict)]
    from .fetch import extract_request_artifacts

    return extract_request_artifacts(row).get("events", [])


def _logical_request_id(request_id: int, instance_id: str) -> str:
    return f"urn:fyi:{instance_id}:request:{request_id}"


def _activity(event: dict[str, Any]) -> str:
    raw = (
        event.get("event_type")
        or event.get("type")
        or event.get("described_state")
        or event.get("state")
        or event.get("title")
        or "observed"
    )
    return str(raw).strip().lower().replace(" ", "_")


def _source_ref(event: dict[str, Any], index: int) -> str:
    return str(
        event.get("id") or event.get("event_id") or event.get("path") or f"source-index:{index}",
    )


def export_process_events(
    *,
    derived_dir: Path,
    output: Path,
    captured_at: str,
    checkpoint: Path | None = None,
    instance_id: str = "nz-fyi",
    source: str = "urn:fyi-cli:site:fyi.org.nz",
    base_url: str = "https://fyi.org.nz",
    attachments_output: Path | None = None,
) -> dict[str, Any]:
    """Emit deterministic NDJSON events and advance a resumable checkpoint."""
    if not derived_dir.is_dir():
        raise FileNotFoundError(f"derived request store does not exist: {derived_dir}")
    previous: dict[str, dict[str, Any]] = {}
    if checkpoint and checkpoint.exists():
        payload = json.loads(checkpoint.read_text(encoding="utf-8"))
        if not isinstance(payload, dict) or not isinstance(payload.get("event_digests", {}), dict):
            raise ValueError("checkpoint must contain an event_digests object")
        for key, value in payload["event_digests"].items():
            previous[str(key)] = (
                {"digest": str(value), "revision": 1} if isinstance(value, str) else value
            )

    rows = []
    for path in sorted(derived_dir.glob("*/*/request.json")):
        row = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(row, dict):
            raise ValueError(f"request record must be an object: {path}")
        rows.append(row)
    rows.sort(key=lambda row: _request_id(row))

    events: list[dict[str, Any]] = []
    attachments: list[dict[str, Any]] = []
    digests: dict[str, dict[str, Any]] = {}
    current_ids: set[str] = set()
    for row in rows:
        request_id = _request_id(row)
        logical_id = _logical_request_id(request_id, instance_id)
        source_events = _events(row)
        from .fetch import extract_request_artifacts

        source_attachments = row.get("attachments")
        if not isinstance(source_attachments, list):
            source_attachments = extract_request_artifacts(row).get("attachments", [])
        for index, raw in enumerate(source_events):
            source_ref = _source_ref(raw, index)
            event_id = f"{logical_id}:event:{_sha256((source_ref, index))}"
            timestamp, timestamp_status = _utc(
                raw.get("occurred_at") or raw.get("created_at") or raw.get("updated_at"),
            )
            item = {
                "schema_version": SCHEMA_VERSION,
                "event_id": event_id,
                "logical_request_id": logical_id,
                "activity": _activity(raw),
                "state": str(raw.get("state") or raw.get("described_state") or "") or None,
                "timestamp": timestamp,
                "timestamp_status": timestamp_status,
                "source_order": {
                    "source": source,
                    "request_sequence": request_id,
                    "event_sequence": index,
                },
                "provenance": {
                    "source_ref": source_ref,
                    "message_reference_id": str(
                        raw.get("message_id") or raw.get("message_reference_id") or "",
                    )
                    or None,
                    "capture_uri": f"{base_url.rstrip('/')}/request/{request_id}.json",
                    "captured_at": captured_at,
                },
            }
            digest = _sha256(item)
            old = previous.get(event_id)
            revision = int(old.get("revision", 0)) + 1 if old else 1
            item["revision"] = revision
            item["operation"] = "upsert"
            current_ids.add(event_id)
            digests[event_id] = {"digest": digest, "revision": revision}
            if old and not old.get("deleted") and old.get("digest") == digest:
                continue
            events.append(item)

        for attachment_index, attachment in enumerate(source_attachments):
            if not isinstance(attachment, dict):
                continue
            url = str(attachment.get("url") or "")
            attachment_id = f"{logical_id}:attachment:{_sha256((url, attachment_index))}"
            metadata = {
                "schema_version": SCHEMA_VERSION,
                "attachment_id": attachment_id,
                "logical_request_id": logical_id,
                "source_order": {
                    "source": source,
                    "request_sequence": request_id,
                    "attachment_sequence": attachment_index,
                },
                "content_type": str(attachment.get("content_type") or "") or None,
                "byte_size": attachment.get("size")
                if isinstance(attachment.get("size"), int)
                else None,
                "locator": {"uri": url} if url else None,
                "warc_record_id": row.get("warc_record_id") or row.get("warc_uri") or None,
                "provenance": {
                    "captured_at": captured_at,
                    "source_path": str(attachment.get("path") or ""),
                },
            }
            attachments.append(metadata)

    for event_id, old in sorted(previous.items()):
        if event_id not in current_ids and not old.get("deleted"):
            logical_id = event_id.split(":event:", 1)[0]
            events.append(
                {
                    "schema_version": SCHEMA_VERSION,
                    "event_id": event_id,
                    "logical_request_id": logical_id,
                    "activity": "removed",
                    "timestamp": captured_at,
                    "source_order": {"source": source, "event_sequence": len(events)},
                    "provenance": {"captured_at": captured_at},
                    "operation": "delete",
                    "revision": int(old.get("revision", 1)) + 1,
                },
            )
            digests[event_id] = {
                "digest": str(old.get("digest", "")),
                "revision": int(old.get("revision", 1)) + 1,
                "deleted": True,
            }

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(b"".join(_canonical(item) + b"\n" for item in events))
    if attachments_output is not None:
        attachments_output.parent.mkdir(parents=True, exist_ok=True)
        attachments_output.write_bytes(b"".join(_canonical(item) + b"\n" for item in attachments))
    if checkpoint:
        checkpoint.parent.mkdir(parents=True, exist_ok=True)
        checkpoint.write_text(
            json.dumps(
                {
                    "schema_version": SCHEMA_VERSION,
                    "captured_at": captured_at,
                    "event_digests": digests,
                },
                sort_keys=True,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "event_count": len(events),
        "total_event_count": len(digests),
        "attachment_count": len(attachments),
        "output": str(output),
    }


def validate_public_event(event: dict[str, Any]) -> None:
    """Fail closed if an exported event contains a prohibited field."""

    def walk(value: Any) -> set[str]:
        if isinstance(value, dict):
            return set(value).intersection(EXCLUDED_FIELDS) | set().union(
                *(walk(item) for item in value.values()),
            )
        if isinstance(value, list):
            return set().union(*(walk(item) for item in value))
        return set()

    leaked = walk(event)
    if leaked:
        raise ValueError(f"public event contains excluded fields: {sorted(leaked)}")
    required = {
        "schema_version",
        "event_id",
        "logical_request_id",
        "activity",
        "timestamp",
        "source_order",
        "provenance",
    }
    missing = required.difference(event)
    if missing:
        raise ValueError(f"public event is missing fields: {sorted(missing)}")


def validate_process_event_file(path: Path) -> dict[str, int]:
    """Validate an exported NDJSON file and return its row count."""
    count = 0
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"invalid JSON at line {line_number}") from exc
        if not isinstance(value, dict):
            raise TypeError(f"row at line {line_number} must be an object")
        if value.get("schema_version") != SCHEMA_VERSION:
            raise ValueError(f"unsupported schema version at line {line_number}")
        validate_public_event(value)
        count += 1
    return {"row_count": count}
