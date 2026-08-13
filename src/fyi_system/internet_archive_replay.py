"""Bounded replay of explicitly approved Internet Archive CDX rows."""

# Exception messages are part of this adapter's tested fail-closed contract.
# ruff: noqa: EM101, EM102, TRY003

from __future__ import annotations

import json
import os
import re
import tempfile
import time
from collections.abc import Callable
from dataclasses import dataclass
from importlib.resources import files
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast
from urllib.parse import urljoin, urlsplit, urlunsplit

import httpx
from jsonschema import Draft202012Validator, FormatChecker
from jsonschema.exceptions import ValidationError

from .acquisition_receipts import canonical_json_bytes, sha256_bytes
from .internet_archive_cdx import validate_host

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

REPLAY_ORIGIN = "https://web.archive.org"
REPLAY_HOST = "web.archive.org"
REPLAY_ADAPTER_ID = "internet-archive-replay"
SELECTION_SCHEMA = "urn:fyi-cli:internet-archive-replay-selection:1"
CHECKPOINT_SCHEMA = "urn:fyi-cli:internet-archive-replay-checkpoint:1"
RESULT_SCHEMA = "urn:fyi-cli:internet-archive-replay-result:1"
SCHEMA_RESOURCE = "schemas/internet-archive-replay-selection-v1.schema.json"
TIMESTAMP = re.compile(r"^\d{14}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
REDIRECT_STATUSES = frozenset({301, 302, 303, 307, 308})
MAX_PAYLOAD_BYTES = 16 * 1024 * 1024
MAX_ROWS = 10_000
MAX_REDIRECTS = 5

Transport = Callable[[str, float, int], httpx.Response]
Observer = Callable[[httpx.Response], None]


class ReplayError(RuntimeError):
    """Raised when replay cannot prove that an approved payload is exact."""


@dataclass(frozen=True)
class ReplayConfig:
    """Validated target boundary and finite operational limits."""

    allowed_target_host: str
    max_rows: int
    max_payload_bytes: int
    max_redirects: int
    max_runtime_seconds: float
    request_timeout_seconds: float

    def __post_init__(self) -> None:
        normalized = validate_host(self.allowed_target_host)
        if normalized != self.allowed_target_host:
            raise ValueError("allowed_target_host must be a normalized lowercase hostname")
        if not 1 <= self.max_rows <= MAX_ROWS:
            raise ValueError(f"max_rows must be between 1 and {MAX_ROWS}")
        if not 1 <= self.max_payload_bytes <= MAX_PAYLOAD_BYTES:
            raise ValueError(
                f"max_payload_bytes must be between 1 and {MAX_PAYLOAD_BYTES}",
            )
        if not 0 <= self.max_redirects <= MAX_REDIRECTS:
            raise ValueError(f"max_redirects must be between 0 and {MAX_REDIRECTS}")
        if self.max_runtime_seconds <= 0:
            raise ValueError("max_runtime_seconds must be positive")
        if self.request_timeout_seconds <= 0:
            raise ValueError("request_timeout_seconds must be positive")

    def identity(self, selection: Mapping[str, object]) -> dict[str, Any]:
        """Return the checkpoint-bound replay identity."""
        return {
            "replay_origin": REPLAY_ORIGIN,
            "allowed_target_host": self.allowed_target_host,
            "selection_sha256": selection["selection_sha256"],
            "source_cdx_sha256": selection["source_cdx_sha256"],
            "max_payload_bytes": self.max_payload_bytes,
        }

    def request_bounds(self, selection: Mapping[str, object]) -> dict[str, Any]:
        """Return immutable identity plus finite run limits for a receipt."""
        return {
            **self.identity(selection),
            "max_rows": self.max_rows,
            "max_redirects": self.max_redirects,
            "max_runtime_seconds": self.max_runtime_seconds,
            "request_timeout_seconds": self.request_timeout_seconds,
        }


def _schema() -> Mapping[str, Any]:
    value = json.loads(files("fyi_system").joinpath(SCHEMA_RESOURCE).read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise TypeError("replay selection schema must be an object")
    return value


def _selection_digest(value: Mapping[str, object]) -> str:
    unsigned = dict(value)
    unsigned.pop("selection_sha256", None)
    return sha256_bytes(canonical_json_bytes(unsigned))


def _canonical_target_url(value: str, allowed_host: str | None = None) -> str:
    if any(character.isspace() or ord(character) < 32 for character in value):
        raise ValueError("original must be a canonical HTTPS URL")
    try:
        parsed = urlsplit(value)
        port = parsed.port
    except ValueError as error:
        raise ValueError("original must be a canonical HTTPS URL") from error
    if (
        parsed.scheme != "https"
        or not parsed.hostname
        or parsed.username is not None
        or parsed.password is not None
        or parsed.fragment
        or port not in {None, 443}
        or parsed.hostname != parsed.hostname.lower()
        or "\\" in value
        or any(segment in {".", ".."} for segment in parsed.path.split("/"))
    ):
        raise ValueError("original must be a canonical HTTPS URL")
    canonical = urlunsplit(("https", parsed.netloc, parsed.path or "/", parsed.query, ""))
    if canonical != value:
        raise ValueError("original must be a canonical HTTPS URL")
    if allowed_host is not None and parsed.hostname != allowed_host:
        raise ValueError("approved row target host does not match allowed target host")
    return canonical


def seal_selection(
    *,
    source_cdx_sha256: str,
    rows: Sequence[Mapping[str, object]],
) -> dict[str, Any]:
    """Validate and self-digest one ordered approved replay selection."""
    value: dict[str, Any] = {
        "schema": SELECTION_SCHEMA,
        "source_cdx_sha256": source_cdx_sha256,
        "rows": [dict(row) for row in rows],
    }
    value["selection_sha256"] = _selection_digest(value)
    return validate_selection(value)


def validate_selection(value: Mapping[str, object]) -> dict[str, Any]:
    """Validate shape, canonical URLs, ordering, and the selection self-pin."""
    try:
        Draft202012Validator(_schema(), format_checker=FormatChecker()).validate(value)
    except ValidationError as error:
        raise ValueError(f"invalid replay selection: {error.message}") from error
    normalized = json.loads(canonical_json_bytes(value))
    if normalized["selection_sha256"] != _selection_digest(normalized):
        raise ValueError("invalid replay selection: selection_sha256 mismatch")
    indexes: list[int] = []
    identities: set[tuple[str, str]] = set()
    for row in normalized["rows"]:
        _canonical_target_url(row["original"])
        identity = (row["original"], row["timestamp"])
        if identity in identities:
            raise ValueError("approved replay rows must have unique URL/timestamp identities")
        identities.add(identity)
        indexes.append(row["row_index"])
    if indexes != sorted(indexes) or len(indexes) != len(set(indexes)):
        raise ValueError("approved replay row indexes must be unique and ordered")
    return normalized


def default_transport(url: str, timeout: float, byte_cap: int) -> httpx.Response:
    """Issue one redirect-free streaming request bounded by a byte cap."""
    with httpx.stream(
        "GET",
        url,
        headers={"User-Agent": "fyi-cli-wayback-replay/1.0"},
        timeout=timeout,
        follow_redirects=False,
    ) as response:
        declared = response.headers.get("Content-Length")
        if declared is not None:
            try:
                if int(declared) > byte_cap:
                    raise ReplayError("Wayback response exceeded the byte cap")
            except ValueError:
                pass
        chunks: list[bytes] = []
        received = 0
        for chunk in response.iter_bytes():
            received += len(chunk)
            if received > byte_cap:
                raise ReplayError("Wayback response exceeded the byte cap")
            chunks.append(chunk)
        return httpx.Response(
            response.status_code,
            headers=response.headers,
            content=b"".join(chunks),
            request=response.request,
            extensions=response.extensions,
        )


def _atomic_write(path: Path, payload: bytes) -> None:
    if path.is_symlink():
        raise ValueError(f"refusing to replace symlink: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.parent.is_symlink():
        raise ValueError(f"refusing to write through symlink: {path.parent}")
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


def _reject_symlink_ancestors(path: Path) -> None:
    """Reject any existing symlink in a write path's directory ancestry."""
    current = path.absolute()
    for candidate in (current, *current.parents):
        if candidate.exists() and candidate.is_symlink():
            raise ValueError(f"refusing symlink in replay path: {candidate}")


def _replay_url(row: Mapping[str, object]) -> str:
    return f"{REPLAY_ORIGIN}/web/{row['timestamp']}id_/{row['original']}"


def _validate_replay_url(value: str, original: str) -> str:
    try:
        parsed = urlsplit(value)
        port = parsed.port
    except ValueError as error:
        raise ReplayError("Wayback redirect URL is invalid") from error
    if (
        parsed.scheme != "https"
        or parsed.hostname != REPLAY_HOST
        or parsed.username is not None
        or parsed.password is not None
        or parsed.fragment
        or port not in {None, 443}
    ):
        raise ReplayError("Wayback redirect escaped the archive host allowlist")
    match = re.fullmatch(r"/web/(\d{14})id_/(https://.+)", parsed.path)
    if match is None:
        raise ReplayError("Wayback redirect does not identify a raw replay target")
    embedded = match.group(2)
    if parsed.query:
        embedded = f"{embedded}?{parsed.query}"
    if embedded != original:
        raise ReplayError("Wayback redirect changed the approved target")
    return value


def _fetch_row(
    row: Mapping[str, object],
    config: ReplayConfig,
    *,
    transport: Transport,
    observer: Observer | None,
    deadline: float,
    clock: Callable[[], float],
) -> tuple[httpx.Response, str]:
    original = _canonical_target_url(str(row["original"]), config.allowed_target_host)
    url = _validate_replay_url(_replay_url(row), original)
    redirects = 0
    while True:
        if clock() >= deadline:
            raise ReplayError("Wayback replay exceeded its runtime deadline")
        response = transport(url, config.request_timeout_seconds, config.max_payload_bytes)
        if len(response.content) > config.max_payload_bytes:
            raise ReplayError("Wayback response exceeded the byte cap")
        if str(response.url) != url:
            raise ReplayError("Wayback transport response URL differs from its request")
        if observer is not None:
            observer(response)
        if response.status_code not in REDIRECT_STATUSES:
            return response, url
        if redirects >= config.max_redirects:
            raise ReplayError("Wayback redirect limit exceeded")
        location = response.headers.get("Location")
        if not location or urlsplit(location).scheme == "":
            raise ReplayError("Wayback redirect must be an absolute approved URL")
        redirected = urljoin(url, location)
        url = _validate_replay_url(redirected, original)
        redirects += 1


def _verify_response(row: Mapping[str, object], response: httpx.Response) -> tuple[str, str]:
    if response.status_code != row["expected_status"]:
        raise ReplayError("Wayback response status does not match approval")
    payload = bytes(response.content)
    declared = response.headers.get("Content-Length")
    if declared is not None:
        try:
            if int(declared) != len(payload):
                raise ReplayError("Wayback declared length does not match payload")
        except ValueError as error:
            raise ReplayError("Wayback declared length is invalid") from error
    if len(payload) != row["expected_payload_bytes"]:
        raise ReplayError("Wayback payload length does not match approval")
    digest = sha256_bytes(payload)
    if digest != row["expected_payload_sha256"]:
        raise ReplayError("Wayback payload digest does not match approval")
    media_type = response.headers.get("Content-Type", "").split(";", 1)[0].strip().lower()
    expected_media_type = row.get("expected_media_type")
    if expected_media_type is not None and media_type != expected_media_type:
        raise ReplayError("Wayback media type does not match approval")
    return digest, media_type


def _checkpoint_value(
    config: ReplayConfig,
    selection: Mapping[str, object],
    completed: Sequence[Mapping[str, object]],
) -> dict[str, Any]:
    value: dict[str, Any] = {
        "schema": CHECKPOINT_SCHEMA,
        "config": config.identity(selection),
        "config_sha256": sha256_bytes(canonical_json_bytes(config.identity(selection))),
        "completed": [dict(item) for item in completed],
        "next_position": len(completed),
    }
    value["checkpoint_sha256"] = sha256_bytes(canonical_json_bytes(value))
    return value


def _load_checkpoint(  # noqa: C901
    path: Path,
    config: ReplayConfig,
    selection: Mapping[str, object],
    output_dir: Path,
) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    if path.is_symlink() or not path.is_file():
        raise ValueError("replay checkpoint must be a regular file")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("replay checkpoint is unreadable") from error
    if not isinstance(value, dict):
        raise TypeError("replay checkpoint must be an object")
    supplied = value.pop("checkpoint_sha256", None)
    if supplied != sha256_bytes(canonical_json_bytes(value)):
        raise ValueError("replay checkpoint self-digest mismatch")
    expected = config.identity(selection)
    if value.get("schema") != CHECKPOINT_SCHEMA or value.get("config") != expected:
        raise ValueError("replay checkpoint configuration mismatch")
    if value.get("config_sha256") != sha256_bytes(canonical_json_bytes(expected)):
        raise ValueError("replay checkpoint configuration digest mismatch")
    completed = value.get("completed")
    if not isinstance(completed, list) or value.get("next_position") != len(completed):
        raise ValueError("replay checkpoint progress is invalid")
    rows = cast("list[dict[str, Any]]", selection["rows"])
    if len(completed) > len(rows):
        raise ValueError("replay checkpoint exceeds approved selection")
    normalized: list[dict[str, Any]] = []
    for position, item in enumerate(completed):
        if not isinstance(item, dict) or item.get("row_index") != rows[position]["row_index"]:
            raise ValueError("replay checkpoint ordering changed")
        expected_name = f"{rows[position]['row_index']:06d}-{item.get('sha256')}.raw"
        if item.get("path") != expected_name:
            raise ValueError("replay checkpoint object path is invalid")
        object_path = output_dir / expected_name
        if object_path.is_symlink() or not object_path.is_file():
            raise ValueError("replay checkpoint object is missing or unsafe")
        payload = object_path.read_bytes()
        if len(payload) != item.get("bytes") or sha256_bytes(payload) != item.get("sha256"):
            raise ValueError("replay checkpoint object integrity mismatch")
        normalized.append(dict(item))
    return normalized


def _result(
    selection: Mapping[str, object],
    completed: Sequence[Mapping[str, object]],
) -> dict[str, Any]:
    return {
        "schema": RESULT_SCHEMA,
        "selection_sha256": selection["selection_sha256"],
        "source_cdx_sha256": selection["source_cdx_sha256"],
        "objects": [dict(item) for item in completed],
    }


def replay_approved_rows(  # noqa: C901
    selection: Mapping[str, object],
    config: ReplayConfig,
    *,
    output_dir: str | Path,
    checkpoint_path: str | Path,
    transport: Transport = default_transport,
    observer: Observer | None = None,
    clock: Callable[[], float] = time.monotonic,
) -> dict[str, Any]:
    """Replay, verify, and atomically persist an ordered approved selection."""
    approved = validate_selection(selection)
    rows = approved["rows"]
    if len(rows) > config.max_rows:
        raise ValueError("approved replay selection exceeds max_rows")
    for row in rows:
        _canonical_target_url(row["original"], config.allowed_target_host)
        if row["expected_payload_bytes"] > config.max_payload_bytes:
            raise ValueError("approved payload length exceeds max_payload_bytes")
    output = Path(output_dir)
    checkpoint = Path(checkpoint_path)
    _reject_symlink_ancestors(output)
    _reject_symlink_ancestors(checkpoint.parent)
    if output.exists() and (output.is_symlink() or not output.is_dir()):
        raise ValueError("replay output must be a plain directory")
    output.mkdir(parents=True, exist_ok=True)
    if output.is_symlink():
        raise ValueError("replay output must be a plain directory")
    if checkpoint.resolve(strict=False).is_relative_to(output.resolve(strict=False)):
        raise ValueError("replay checkpoint must be outside the payload directory")
    completed = _load_checkpoint(checkpoint, config, approved, output)
    deadline = clock() + config.max_runtime_seconds
    for row in rows[len(completed) :]:
        response, final_url = _fetch_row(
            row,
            config,
            transport=transport,
            observer=observer,
            deadline=deadline,
            clock=clock,
        )
        digest, media_type = _verify_response(row, response)
        name = f"{row['row_index']:06d}-{digest}.raw"
        destination = output / name
        payload = bytes(response.content)
        if destination.exists() or destination.is_symlink():
            if destination.is_symlink() or not destination.is_file():
                raise ValueError("replay object path is not a regular file")
            if destination.read_bytes() != payload:
                raise ValueError("replay object collision or corruption")
        else:
            _atomic_write(destination, payload)
        item = {
            "row_index": row["row_index"],
            "original": row["original"],
            "timestamp": row["timestamp"],
            "cdx_digest": row["cdx_digest"],
            "status": response.status_code,
            "bytes": len(payload),
            "sha256": digest,
            "media_type": media_type,
            "final_url": final_url,
            "path": name,
        }
        completed.append(item)
        checkpoint_value = _checkpoint_value(config, approved, completed)
        _atomic_write(checkpoint, canonical_json_bytes(checkpoint_value))
    return _result(approved, completed)


def write_replay_result(path: str | Path, result: Mapping[str, object]) -> None:
    """Atomically persist the deterministic replay result projection."""
    _atomic_write(Path(path), canonical_json_bytes(result))
