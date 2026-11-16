"""Versioned, credential-safe acquisition receipts for source network commands."""

from __future__ import annotations

import hashlib
import json
import math
import os
import re
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
MAX_RECEIPT_BYTES = 16 * 1024 * 1024
MAX_RECEIPT_JSON_NESTING = 127
_U64_MAX = (1 << 64) - 1


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


_RFC3339_DATETIME_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:[Zz]|[+-]\d{2}:\d{2})$",
)
_ADAPTER_ID_RE = re.compile(r"[a-z0-9][a-z0-9._-]*")
_SHA256_RE = re.compile(r"[0-9a-f]{64}")


def _parse_rfc3339_datetime(value: str, field_name: str) -> datetime:
    if not _RFC3339_DATETIME_RE.fullmatch(value):
        message = f"invalid acquisition receipt: {field_name} must be an RFC 3339 date-time"
        raise ValueError(message)
    try:
        parsed = datetime.fromisoformat(value)
    except ValueError as error:
        message = f"invalid acquisition receipt: {field_name} is not a valid date-time"
        raise ValueError(message) from error
    if parsed.tzinfo is None:
        message = f"invalid acquisition receipt: {field_name} must include a UTC offset"
        raise ValueError(message)
    return parsed


def _checked_receipt_integer(value: Any, field_name: str) -> int:
    """Match the Rust verifier's checked unsigned 64-bit counters."""
    if isinstance(value, int) and not isinstance(value, bool) and 0 <= value <= _U64_MAX:
        return value
    if (
        isinstance(value, float)
        and math.isfinite(value)
        and value.is_integer()
        and 0 <= value < (1 << 64)
    ):
        return int(value)
    message = f"invalid acquisition receipt: {field_name} must be an unsigned 64-bit integer"
    raise ValueError(message)


def _validate_recorded_url(value: str) -> None:
    """Reject ambiguous or sensitive URL fields without echoing their contents."""
    try:
        parsed = urlsplit(value)
        _ = parsed.port  # Force validation of malformed and out-of-range ports.
        invalid = (
            not value.startswith(("http://", "https://"))
            or not parsed.netloc
            or not parsed.hostname
            or parsed.username not in (None, "")
            or parsed.password is not None
            or "#" in value
            or ("?" in value and parsed.query != "REDACTED")
            or any(
                character.isspace()
                or ord(character) < 32
                or 127 <= ord(character) <= 159
                or character == "\\"
                for character in value
            )
        )
    except ValueError:
        invalid = True
    if invalid:
        message = "invalid acquisition receipt: unsafe or malformed URL"
        raise ValueError(message)


def _validate_nonnegative_finite(value: Any, field_name: str) -> None:
    try:
        finite = math.isfinite(value)
    except (OverflowError, TypeError):
        finite = False
    if not finite or value < 0:
        message = f"invalid acquisition receipt: invalid {field_name}"
        raise ValueError(message)


def _response_totals(responses: list[dict[str, Any]]) -> tuple[int, int]:
    total_bytes = 0
    total_retries = 0
    for response in responses:
        _validate_recorded_url(response["url"])
        _validate_digest(response["sha256"])
        total_bytes += _checked_receipt_integer(response["bytes"], "response bytes")
        total_retries += _checked_receipt_integer(response["attempts"], "response attempts") - 1
        if total_bytes > _U64_MAX or total_retries > _U64_MAX:
            message = "invalid acquisition receipt: response totals overflow"
            raise ValueError(message)
        for delay in response["retry_delays_seconds"]:
            _validate_nonnegative_finite(delay, "retry delay")
    return total_bytes, total_retries


def _validate_digest(value: str) -> None:
    if not _SHA256_RE.fullmatch(value):
        message = "invalid acquisition receipt: malformed SHA-256 digest"
        raise ValueError(message)


def _validate_request_bounds_numbers(bounds: dict[str, Any]) -> None:
    """Reject integer bounds that cannot retain their JSON value in Rust."""
    pending: list[Any] = [bounds]
    visited: set[int] = set()
    while pending:
        value = pending.pop()
        if isinstance(value, (dict, list)):
            if id(value) in visited:
                continue
            visited.add(id(value))
            pending.extend(value.values() if isinstance(value, dict) else value)
        elif isinstance(value, int) and not isinstance(value, bool):
            if not -(1 << 63) <= value <= _U64_MAX:
                message = "invalid acquisition receipt: request bound integer is out of range"
                raise ValueError(message)


def _validate_receipt_evidence(receipt: Mapping[str, Any]) -> None:
    """Check transport counters and URL safety after schema validation."""
    if not _ADAPTER_ID_RE.fullmatch(receipt["adapter"]["id"]):
        message = "invalid acquisition receipt: malformed adapter ID"
        raise ValueError(message)
    for digest in (
        receipt["receipt_sha256"],
        receipt["result"]["sha256"],
        receipt["checkpoint"]["before_sha256"],
        receipt["checkpoint"]["after_sha256"],
    ):
        if digest is not None:
            _validate_digest(digest)
    _validate_recorded_url(receipt["source"]["url"])
    _validate_request_bounds_numbers(receipt["request_bounds"])
    responses = receipt["responses"]
    if (
        receipt["adapter"]["id"] == "alaveteli-sync"
        and receipt["command"] in {"sync-pull", "sync-push"}
        and not responses
    ):
        message = "invalid acquisition receipt: sync transport evidence is unavailable"
        raise ValueError(message)

    total_bytes, total_retries = _response_totals(responses)
    totals = receipt["totals"]
    if (
        _checked_receipt_integer(totals["requests"], "requests") != len(responses)
        or _checked_receipt_integer(totals["bytes"], "bytes") != total_bytes
        or _checked_receipt_integer(totals["retries"], "retries") != total_retries
    ):
        message = "invalid acquisition receipt: response totals do not match observations"
        raise ValueError(message)
    _checked_receipt_integer(receipt["result"]["bytes"], "result bytes")
    interval = receipt["rate_limit"]["minimum_interval_seconds"]
    if interval is not None:
        _validate_nonnegative_finite(interval, "rate limit interval")


def validate_receipt(receipt: Mapping[str, Any]) -> None:
    """Validate structure, observable evidence invariants and the self-digest."""
    try:
        Draft202012Validator(_schema(), format_checker=FormatChecker()).validate(receipt)
    except ValidationError as error:
        message = f"invalid acquisition receipt: {error.message}"
        raise ValueError(message) from error
    _validate_receipt_evidence(receipt)
    started = _parse_rfc3339_datetime(str(receipt["started_at"]), "started_at")
    completed = _parse_rfc3339_datetime(str(receipt["completed_at"]), "completed_at")
    if completed < started:
        message = "invalid acquisition receipt: completed_at precedes started_at"
        raise ValueError(message)
    unsigned = dict(receipt)
    supplied = str(unsigned.pop("receipt_sha256"))
    expected = sha256_bytes(canonical_json_bytes(unsigned))
    if supplied != expected:
        message = "invalid acquisition receipt: receipt_sha256 mismatch"
        raise ValueError(message)


def _unique_receipt_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            message = "invalid acquisition receipt: duplicate JSON member"
            raise ValueError(message)
        result[key] = value
    return result


def _finite_receipt_float(value: str) -> float:
    number = float(value)
    if not math.isfinite(number):
        message = "invalid acquisition receipt: nonfinite JSON number"
        raise ValueError(message)
    return number


def _reject_receipt_constant(_value: str) -> Any:
    message = "invalid acquisition receipt: nonfinite JSON number"
    raise ValueError(message)


def _check_receipt_nesting(text: str) -> None:
    """Bound container nesting before parsing, ignoring characters inside strings."""
    depth = 0
    in_string = False
    escaped = False
    for character in text:
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
        elif character == '"':
            in_string = True
        elif character in "[{":
            depth += 1
            if depth > MAX_RECEIPT_JSON_NESTING:
                message = "invalid acquisition receipt: JSON nesting exceeds the limit"
                raise ValueError(message)
        elif character in "]}":
            depth -= 1


def parse_receipt_bytes(data: bytes) -> dict[str, Any]:
    """Parse bounded, duplicate-free UTF-8 receipt bytes and validate them.

    Use this for untrusted receipt bytes: a pre-parsed mapping cannot reveal
    duplicate members discarded by its JSON decoder.
    """
    if len(data) > MAX_RECEIPT_BYTES:
        message = "invalid acquisition receipt: document exceeds the 16 MiB byte limit"
        raise ValueError(message)
    try:
        text = data.decode("utf-8")
        _check_receipt_nesting(text)
        receipt = json.loads(
            text,
            object_pairs_hook=_unique_receipt_object,
            parse_float=_finite_receipt_float,
            parse_constant=_reject_receipt_constant,
        )
    except (ValueError, UnicodeDecodeError, RecursionError) as error:
        message = "invalid acquisition receipt: invalid or ambiguous JSON"
        raise ValueError(message) from error
    if not isinstance(receipt, dict):
        message = "invalid acquisition receipt: document must be an object"
        raise TypeError(message)
    validate_receipt(receipt)
    return receipt


def write_receipt_atomic(path: Path, receipt: Mapping[str, Any]) -> None:
    """Atomically publish immutable receipt evidence without overwriting prior data.

    Byte-identical repeats are idempotent. A different or corrupt existing
    receipt is rejected. Same-directory hard links guarantee no-clobber
    publication even if concurrent processes race to create the same path.
    """
    validate_receipt(receipt)
    payload = canonical_json_bytes(receipt)
    # Do not publish evidence that the bounded reader cannot reopen.
    parse_receipt_bytes(payload)
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
        try:
            os.link(temporary, path)
        except FileExistsError:
            if path.is_symlink() or not path.is_file():
                message = "existing acquisition receipt is not a regular file"
                raise FileExistsError(message) from None
            with path.open("rb") as stream:
                existing = stream.read(MAX_RECEIPT_BYTES + 1)
            try:
                parse_receipt_bytes(existing)
            except (ValueError, TypeError, UnicodeDecodeError) as error:
                message = "existing acquisition receipt is invalid"
                raise FileExistsError(message) from error
            if existing != payload:
                message = "different acquisition receipt already exists"
                raise FileExistsError(message) from None
        if os.name == "posix":
            directory_fd = os.open(path.parent, os.O_RDONLY)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
    finally:
        temporary.unlink(missing_ok=True)


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
