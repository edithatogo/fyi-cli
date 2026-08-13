"""Versioned, credential-safe acquisition receipts for source network commands."""

from __future__ import annotations

import hashlib
import json
import os
import tempfile
from dataclasses import dataclass, field
from datetime import UTC, datetime
from importlib.metadata import PackageNotFoundError, version
from importlib.resources import files
from pathlib import Path
from typing import TYPE_CHECKING, Any
from urllib.parse import urlsplit, urlunsplit

from jsonschema import Draft202012Validator, FormatChecker
from jsonschema.exceptions import ValidationError

from fyi_system import __version__

if TYPE_CHECKING:
    from collections.abc import Mapping

RECEIPT_SCHEMA = "urn:fyi-cli:acquisition-receipt"
RECEIPT_SCHEMA_VERSION = "1.0.0"
SCHEMA_RESOURCE = "schemas/acquisition-receipt-v1.schema.json"


def installed_adapter_version() -> str:
    """Return installed distribution version with a source-tree fallback."""
    try:
        return version("fyi-cli")
    except PackageNotFoundError:
        return __version__


ADAPTER_VERSION = installed_adapter_version()


def utc_now() -> str:
    """Return a UTC RFC 3339 timestamp."""
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def canonical_json_bytes(value: object) -> bytes:
    """Serialize JSON deterministically with a final newline."""
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            allow_nan=False,
            separators=(",", ":"),
            sort_keys=True,
        )
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    """Return a lowercase SHA-256 digest."""
    return hashlib.sha256(value).hexdigest()


def file_sha256(path: Path | None) -> str | None:
    """Hash a regular checkpoint file when present."""
    if path is None or not path.is_file():
        return None
    return sha256_bytes(path.read_bytes())


def sanitize_url(value: str) -> str:
    """Remove credentials, query values, and fragments from receipt URLs."""
    parsed = urlsplit(value)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        message = "acquisition source must be an absolute HTTP(S) URL"
        raise ValueError(message)
    host = parsed.hostname
    if parsed.port is not None:
        host = f"{host}:{parsed.port}"
    redacted_query = "REDACTED" if parsed.query else ""
    return urlunsplit((parsed.scheme, host, parsed.path or "/", redacted_query, ""))


def _schema() -> Mapping[str, Any]:
    raw = files("fyi_system").joinpath(SCHEMA_RESOURCE).read_text(encoding="utf-8")
    value = json.loads(raw)
    if not isinstance(value, dict):
        message = "acquisition receipt schema must be an object"
        raise TypeError(message)
    return value


def validate_receipt(receipt: Mapping[str, Any]) -> None:
    """Validate structure and the self-digest of a receipt."""
    try:
        Draft202012Validator(_schema(), format_checker=FormatChecker()).validate(receipt)
    except ValidationError as error:
        message = f"invalid acquisition receipt: {error.message}"
        raise ValueError(message) from error
    unsigned = dict(receipt)
    supplied = str(unsigned.pop("receipt_sha256"))
    expected = sha256_bytes(canonical_json_bytes(unsigned))
    if supplied != expected:
        message = "invalid acquisition receipt: receipt_sha256 mismatch"
        raise ValueError(message)


def write_receipt_atomic(path: Path, receipt: Mapping[str, Any]) -> None:
    """Validate and atomically replace a receipt without clobbering valid evidence."""
    validate_receipt(receipt)
    payload = canonical_json_bytes(receipt)
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.",
        suffix=".tmp",
        dir=path.parent,
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(handle, "wb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.replace(path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


@dataclass
class AcquisitionRecorder:
    """Collect bounded HTTP evidence and emit one acquisition receipt."""

    command: str
    adapter_id: str
    source_url: str
    request_bounds: dict[str, Any]
    rate_limit_name: str | None = None
    minimum_interval_seconds: float | None = None
    checkpoint_path: Path | None = None
    adapter_version: str = ADAPTER_VERSION
    started_at: str = field(default_factory=utc_now)
    responses: list[dict[str, Any]] = field(default_factory=list)
    _checkpoint_before: str | None = field(init=False)

    def __post_init__(self) -> None:
        self.source_url = sanitize_url(self.source_url)
        self._checkpoint_before = file_sha256(self.checkpoint_path)

    def observe_response(self, response: Any) -> None:
        """Record a response without headers or other credential-bearing material."""
        payload = bytes(response.content)
        extensions = getattr(response, "extensions", {}) or {}
        attempts = int(extensions.get("fyi_attempts", 1))
        delays = [float(item) for item in extensions.get("fyi_retry_delays_seconds", [])]
        self.responses.append(
            {
                "url": sanitize_url(str(response.url)),
                "status": int(response.status_code),
                "bytes": len(payload),
                "sha256": sha256_bytes(payload),
                "attempts": attempts,
                "retry_delays_seconds": delays,
            },
        )

    def build(
        self,
        *,
        result_projection: bytes,
        result_media_type: str,
        status: str = "succeeded",
        failure_type: str | None = None,
        completed_at: str | None = None,
    ) -> dict[str, Any]:
        """Build and self-digest a schema-valid receipt."""
        receipt: dict[str, Any] = {
            "schema": RECEIPT_SCHEMA,
            "schema_version": RECEIPT_SCHEMA_VERSION,
            "command": self.command,
            "adapter": {"id": self.adapter_id, "version": self.adapter_version},
            "source": {"url": self.source_url},
            "request_bounds": self.request_bounds,
            "started_at": self.started_at,
            "completed_at": completed_at or utc_now(),
            "status": status,
            "responses": self.responses,
            "totals": {
                "requests": len(self.responses),
                "bytes": sum(item["bytes"] for item in self.responses),
                "retries": sum(item["attempts"] - 1 for item in self.responses),
            },
            "rate_limit": {
                "name": self.rate_limit_name,
                "minimum_interval_seconds": self.minimum_interval_seconds,
            },
            "checkpoint": {
                "before_sha256": self._checkpoint_before,
                "after_sha256": file_sha256(self.checkpoint_path),
            },
            "result": {
                "representation": "canonical_result_projection",
                "media_type": result_media_type,
                "bytes": len(result_projection),
                "sha256": sha256_bytes(result_projection),
            },
        }
        if status == "failed":
            receipt["failure"] = {"type": failure_type or "AcquisitionError"}
        receipt["receipt_sha256"] = sha256_bytes(canonical_json_bytes(receipt))
        validate_receipt(receipt)
        return receipt

    def write(
        self,
        path: str | Path,
        *,
        result_projection: bytes,
        result_media_type: str,
    ) -> dict[str, Any]:
        """Build, validate, and atomically write a successful receipt."""
        receipt = self.build(
            result_projection=result_projection,
            result_media_type=result_media_type,
        )
        write_receipt_atomic(Path(path), receipt)
        return receipt

    def write_failure(self, path: str | Path, *, failure_type: str) -> dict[str, Any]:
        """Atomically write a failed receipt without exception text or credentials."""
        receipt = self.build(
            result_projection=canonical_json_bytes({"status": "failed"}),
            result_media_type="application/json",
            status="failed",
            failure_type=failure_type,
        )
        write_receipt_atomic(Path(path), receipt)
        return receipt


def observe_response(recorder: AcquisitionRecorder | None, response: Any) -> None:
    """Add a response when receipt collection is enabled."""
    if recorder is not None:
        recorder.observe_response(response)
