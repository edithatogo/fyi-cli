"""Experimental EvidenceDelta emitter for the fyi-process integration."""

from __future__ import annotations

import hashlib
import json
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path


def _canonical(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _sha256(value: Any) -> str:
    return hashlib.sha256(_canonical(value).encode("utf-8")).hexdigest()


def _digest(value: dict[str, Any]) -> str:
    candidate = str(value.get("content_sha256") or "").lower()
    if len(candidate) == 64 and all(char in "0123456789abcdef" for char in candidate):
        return candidate
    return _sha256(value)


def _timestamp(value: Any, fallback: str) -> str:
    if isinstance(value, str) and value:
        try:
            parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
            if parsed.tzinfo is None:
                parsed = parsed.replace(tzinfo=UTC)
            return parsed.astimezone(UTC).isoformat().replace("+00:00", "Z")
        except ValueError:
            pass
    return fallback


def _request_url(request: dict[str, Any], base_url: str) -> str:
    url = str(request.get("request_url") or "").strip()
    if url:
        return url
    title = str(request.get("url_title") or request.get("id") or "").strip()
    return f"{base_url.rstrip('/')}/request/{title}"


def _logical_id(request_id: int, instance_id: str) -> str:
    return f"urn:fyi-archive:{instance_id}:request:{request_id}:manifest"


def _request_hint(request_id: int, instance_id: str) -> str:
    return f"urn:alaveteli:{instance_id}:request:{request_id}"


def _delta_id(
    logical_id: str,
    revision: int,
    operation: str,
    digest: str | None,
    sequence: int,
) -> str:
    payload = (logical_id, revision, operation, digest, sequence)
    return f"urn:foi-process:delta:sha256:{_sha256(payload)}"


def _record_delta(
    *,
    request: dict[str, Any],
    instance_id: str,
    jurisdiction: str,
    site: str,
    source: str,
    partition: str,
    sequence: int,
    captured_at: str,
    revision: int,
    operation: str,
    previous_digest: str | None,
    current_digest: str | None,
    base_url: str,
) -> dict[str, Any]:
    request_id = int(request.get("request_id") or request["id"])
    logical_id = _logical_id(request_id, instance_id)
    event_time = _timestamp(
        request.get("last_updated") or request.get("updated_at") or request.get("date"),
        captured_at,
    )
    attributes = {
        "platform_activity": "platform_state_observed",
        "platform_state": str(request.get("state") or ""),
        "request_title": str(request.get("title") or ""),
        "url_title": str(request.get("url_title") or ""),
        "request_url": _request_url(request, base_url),
        "authority_name": str(request.get("authority") or request.get("public_body") or ""),
        "event_time": event_time,
    }
    delta: dict[str, Any] = {
        "schema_version": "1.0.0-draft.1",
        "delta_id": _delta_id(logical_id, revision, operation, current_digest, sequence),
        "logical_record_id": logical_id,
        "revision": revision,
        "operation": operation,
        "site": site,
        "jurisdiction": f"urn:jurisdiction:{jurisdiction}",
        "position": {"source": source, "partition": partition, "sequence": sequence},
        "observed_at": captured_at,
        "captured_at": captured_at,
        "previous_content_sha256": previous_digest,
        "current_content_sha256": current_digest,
        "request_hint": _request_hint(request_id, instance_id),
        "attributes": attributes,
    }
    if operation != "delete" and current_digest is not None:
        delta["evidence"] = {
            "schema_version": "1.0.0-draft.1",
            "evidence_id": (
                f"urn:foi-process:evidence:sha256:{_sha256((logical_id, revision, current_digest))}"
            ),
            "logical_record_id": logical_id,
            "revision": revision,
            "source_kind": "foip:AlaveteliJson",
            "media_type": "application/json",
            "locator": {"uri": _request_url(request, base_url)},
            "content_sha256": current_digest,
            "captured_at": captured_at,
            "privacy": {
                "sensitivity": "unknown",
                "access_tier": "restricted",
                "disposition": "needs_review",
                "reason_codes": ["privacy:not_assessed"],
                "human_reviewed": False,
            },
        }
    return delta


def _load_current(derived_dir: Path) -> list[dict[str, Any]]:
    rows = []
    for path in sorted(derived_dir.glob("*/*/request.json")):
        value = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(value, dict):
            message = f"request record must be an object: {path}"
            raise TypeError(message)
        rows.append(value)
    return sorted(rows, key=lambda row: int(row.get("request_id", row.get("id", 0))))


def _load_previous(path: Path | None) -> dict[int, dict[str, Any]]:
    if path is None or not path.exists():
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    rows = payload.get("requests", [])
    return {int(row["request_id"]): row for row in rows}


def emit_evidence_deltas(
    *,
    derived_dir: Path,
    output: Path,
    captured_at: str,
    previous_manifest: Path | None = None,
    instance_id: str = "nz-fyi",
    jurisdiction: str = "NZ",
    site: str = "urn:alaveteli:site:fyi.org.nz",
    source: str = "urn:fyi-cli:site:fyi.org.nz",
    partition: str = "requests",
    base_url: str = "https://fyi.org.nz",
) -> list[dict[str, Any]]:
    """Emit deterministic upserts and deletions from a derived request store."""
    current = _load_current(derived_dir)
    previous = _load_previous(previous_manifest)
    deltas = []
    current_ids: set[int] = set()
    for offset, request in enumerate(current, start=1):
        request_id = int(request.get("request_id") or request["id"])
        current_ids.add(request_id)
        digest = _digest(request)
        old = previous.get(request_id)
        old_digest = str(old.get("content_sha256", "")).lower() if old else None
        if old is not None and old_digest == digest:
            continue
        revision = int(old.get("revision", 0)) + 1 if old else 1
        deltas.append(
            _record_delta(
                request=request,
                instance_id=instance_id,
                jurisdiction=jurisdiction,
                site=site,
                source=source,
                partition=partition,
                sequence=offset,
                captured_at=captured_at,
                revision=revision,
                operation="upsert",
                previous_digest=old_digest,
                current_digest=digest,
                base_url=base_url,
            ),
        )
    for request_id, old in sorted(previous.items()):
        if request_id in current_ids:
            continue
        offset = len(deltas) + 1
        old_digest = str(old["content_sha256"]).lower()
        tombstone = {"request_id": request_id, "url_title": old.get("url_title", "")}
        deltas.append(
            _record_delta(
                request=tombstone,
                instance_id=instance_id,
                jurisdiction=jurisdiction,
                site=site,
                source=source,
                partition=partition,
                sequence=offset,
                captured_at=captured_at,
                revision=int(old.get("revision", 1)) + 1,
                operation="delete",
                previous_digest=old_digest,
                current_digest=None,
                base_url=base_url,
            ),
        )
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        "".join(json.dumps(row, sort_keys=True, ensure_ascii=False) + "\n" for row in deltas),
        encoding="utf-8",
    )
    return deltas
