"""Bounded, allowlisted historical-index retrieval adapter.

Provides read-only retrieval from approved external historical indexes
(such as Morph) so that fyi-archive does not download source indexes
directly. Every retrieval emits acquisition receipt evidence via the
shared ``observe_response`` observer and is bounded by allowlisted
domains, page caps, row caps, byte caps, and deadline enforcement.
"""

# Exception messages are part of this adapter's tested fail-closed contract.
# ruff: noqa: EM101, EM102, TRY003, C901, E501, COM812

from __future__ import annotations

import json
import os
import re
import tempfile
import time
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib.parse import parse_qsl, urlencode, urlsplit

import httpx

from .acquisition_receipts import canonical_json_bytes, sha256_bytes

# Narrowly allowlisted external historical indexes. Only these hosts may be
# queried; every other host is rejected fail-closed.
ALLOWLISTED_HOSTS = frozenset(
    {
        "morph.io",
        "www.morph.io",
    },
)

HISTORICAL_INDEX_ADAPTER_ID = "historical-index"
CHECKPOINT_SCHEMA = "urn:fyi-cli:historical-index-checkpoint:1"
MAX_PAGES = 1_000
MAX_ROWS = 100_000
MAX_RESPONSE_BYTES = 16 * 1024 * 1024
REQUEST_ATTEMPTS = 8
MAX_RETRY_BACKOFF_SECONDS = 60.0

HOST = re.compile(
    "".join(
        (
            r"^(?=.{1,253}$)(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)*",
            r"[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$",
        ),
    ),
)


class HistoricalIndexError(RuntimeError):
    """Raised when bounded historical-index retrieval cannot prove progress."""


Transport = Callable[[str, float], httpx.Response]
Observer = Callable[[httpx.Response], None]


def validate_host(value: str) -> str:
    """Validate and normalize a DNS hostname used as the query boundary."""
    normalized = value.rstrip(".").lower()
    if not HOST.fullmatch(normalized) or normalized == "localhost":
        raise ValueError("historical index host must be a valid public DNS hostname")
    return normalized


def is_allowlisted(host: str) -> bool:
    """Return True when ``host`` is on the approved historical-index allowlist."""
    return host.rstrip(".").lower() in ALLOWLISTED_HOSTS


def _atomic_write(path: Path, payload: bytes) -> None:
    if path.is_symlink():
        raise ValueError(f"refusing to replace symlink: {path}")
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


def _checkpoint_value(state: dict[str, Any]) -> dict[str, Any]:
    value = dict(state)
    unsigned = canonical_json_bytes(value)
    value["checkpoint_sha256"] = sha256_bytes(unsigned)
    return value


def _write_checkpoint(path: Path, state: dict[str, Any]) -> None:
    _atomic_write(path, canonical_json_bytes(_checkpoint_value(state)))


def _load_checkpoint(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {
            "schema": CHECKPOINT_SCHEMA,
            "pages_fetched": 0,
            "rows_collected": 0,
            "next_cursor": None,
            "complete": False,
        }
    if path.is_symlink() or not path.is_file():
        raise ValueError("checkpoint must be a regular file")
    try:
        value = json.loads(path.read_bytes().decode("utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("checkpoint must be strict UTF-8 JSON") from error
    if not isinstance(value, dict):
        raise TypeError("checkpoint must be a JSON object")
    supplied_digest = value.get("checkpoint_sha256")
    unsigned = dict(value)
    unsigned.pop("checkpoint_sha256", None)
    if supplied_digest != sha256_bytes(canonical_json_bytes(unsigned)):
        raise ValueError("checkpoint self-digest mismatch")
    if value.get("schema") != CHECKPOINT_SCHEMA:
        raise ValueError("unsupported checkpoint schema")
    return value


def _verify_response_url(response: httpx.Response, requested_url: str) -> None:
    actual = urlsplit(str(response.url))
    requested = urlsplit(requested_url)
    if (
        actual.scheme != "https"
        or actual.username is not None
        or actual.password is not None
        or parse_qsl(actual.query, keep_blank_values=True)
        != parse_qsl(requested.query, keep_blank_values=True)
    ):
        raise HistoricalIndexError("historical index response escaped the request boundary")


def _observe(
    response: httpx.Response,
    observer: Observer | None,
    attempts: int,
    delays: list[float],
) -> None:
    response.extensions["fyi_attempts"] = attempts
    response.extensions["fyi_retry_delays_seconds"] = delays
    if observer is not None:
        observer(response)


def _retry_delay(response: httpx.Response | None, attempt: int) -> float:
    if response is not None:
        raw = response.headers.get("Retry-After")
        if raw is not None:
            try:
                return min(max(float(raw), 0.0), MAX_RETRY_BACKOFF_SECONDS)
            except ValueError:
                pass
    return min(2 ** (attempt + 1), MAX_RETRY_BACKOFF_SECONDS)


def _fetch_json(
    url: str,
    *,
    transport: Transport,
    observer: Observer | None,
    deadline: float,
    clock: Callable[[], float],
    sleep: Callable[[float], None],
) -> Any:
    delays: list[float] = []
    last_error: Exception | None = None
    last_response: httpx.Response | None = None
    for attempt in range(REQUEST_ATTEMPTS):
        remaining = deadline - clock()
        if remaining <= 0:
            raise HistoricalIndexError("historical index acquisition exceeded its deadline") from last_error
        response: httpx.Response | None = None
        try:
            response = transport(url, min(60.0, remaining))
            last_response = response
            _verify_response_url(response, url)
            if response.status_code != 200:
                retryable = response.status_code in {429, 500, 502, 503, 504}
                if not retryable:
                    _observe(response, observer, attempt + 1, delays)
                    raise HistoricalIndexError(f"historical index returned HTTP {response.status_code}")
                last_error = HistoricalIndexError(f"historical index returned HTTP {response.status_code}")
            elif len(response.content) > MAX_RESPONSE_BYTES:
                _observe(response, observer, attempt + 1, delays)
                raise HistoricalIndexError("historical index response exceeded the byte cap")
            else:
                try:
                    payload = json.loads(response.content.decode("utf-8"))
                except (UnicodeDecodeError, json.JSONDecodeError) as error:
                    last_error = error
                else:
                    _observe(response, observer, attempt + 1, delays)
                    return payload
        except (httpx.TimeoutException, httpx.TransportError, OSError) as error:
            last_error = error
        if attempt < REQUEST_ATTEMPTS - 1:
            delay = min(_retry_delay(response, attempt), max(0.0, deadline - clock()))
            delays.append(delay)
            sleep(delay)
    if last_response is not None:
        _observe(last_response, observer, REQUEST_ATTEMPTS, delays)
    raise HistoricalIndexError("historical index request failed after bounded retries") from last_error


@dataclass(frozen=True)
class HistoricalIndexConfig:
    """Validated bounds for one deterministic historical-index traversal."""

    base_url: str
    index_path: str
    query: dict[str, str]
    page_size: int
    max_pages: int
    max_rows: int
    max_runtime_seconds: float
    max_stall_seconds: float | None = None
    cursor_param: str = "cursor"

    def __post_init__(self) -> None:
        normalized_host = validate_host(urlsplit(self.base_url).hostname or "")
        if not is_allowlisted(normalized_host):
            raise ValueError(
                f"historical index host {self.base_url!r} is not on the allowlist: "
                f"{sorted(ALLOWLISTED_HOSTS)}",
            )
        parsed = urlsplit(self.base_url)
        if parsed.scheme != "https" or parsed.username or parsed.password or parsed.port:
            raise ValueError("historical index base_url must be a simple https:// URL")
        if not 1 <= self.page_size <= MAX_PAGES:
            raise ValueError(f"page_size must be between 1 and {MAX_PAGES}")
        if not 1 <= self.max_pages <= MAX_PAGES:
            raise ValueError(f"max_pages must be between 1 and {MAX_PAGES}")
        if not 1 <= self.max_rows <= MAX_ROWS:
            raise ValueError(f"max_rows must be between 1 and {MAX_ROWS}")
        if self.max_runtime_seconds <= 0:
            raise ValueError("max_runtime_seconds must be positive")
        if self.max_stall_seconds is not None and self.max_stall_seconds <= 0:
            raise ValueError("max_stall_seconds must be positive")

    def request_bounds(self) -> dict[str, Any]:
        return {
            "base_url": self.base_url,
            "index_path": self.index_path,
            "query": dict(self.query),
            "page_size": self.page_size,
            "max_pages": self.max_pages,
            "max_rows": self.max_rows,
            "max_runtime_seconds": self.max_runtime_seconds,
            "allowlisted_hosts": sorted(ALLOWLISTED_HOSTS),
        }


def _page_url(config: HistoricalIndexConfig, cursor: str | None) -> str:
    params = dict(config.query)
    params["page_size"] = str(config.page_size)
    if cursor is not None:
        params[config.cursor_param] = cursor
    return f"{config.base_url.rstrip('/')}/{config.index_path.lstrip('/')}?{urlencode(params)}"


def _extract_rows(payload: Any) -> list[Any]:
    if isinstance(payload, dict):
        for key in ("rows", "results", "items", "records", "data"):
            value = payload.get(key)
            if isinstance(value, list):
                return value
        if isinstance(payload.get("row"), list):
            return payload["row"]
    if isinstance(payload, list):
        return payload
    raise HistoricalIndexError("historical index returned an invalid payload shape")


def _extract_cursor(payload: Any) -> str | None:
    if isinstance(payload, dict):
        for key in ("next_cursor", "cursor", "next", "pagination"):
            value = payload.get(key)
            if isinstance(value, str) and value:
                return value
            if isinstance(value, dict):
                for inner in ("cursor", "next", "url"):
                    inner_value = value.get(inner)
                    if isinstance(inner_value, str) and inner_value:
                        return inner_value
    return None


def retrieve_historical_index(
    config: HistoricalIndexConfig,
    *,
    output_path: str | Path,
    checkpoint_path: str | Path,
    transport: Transport,
    observer: Observer | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> list[Any]:
    """Retrieve a bounded, allowlisted historical index and persist proof of progress.

    The index is fetched page by page. Each page response is observed through
    the acquisition receipt observer (recording status, bytes, digest, retry
    evidence). Progress is checkpointed atomically so a retrieval can resume
    from its last verified cursor. Output and checkpoint paths must differ.
    """
    output = Path(output_path)
    checkpoint = Path(checkpoint_path)
    if output.resolve(strict=False) == checkpoint.resolve(strict=False):
        raise ValueError("output and checkpoint paths must differ")
    state = _load_checkpoint(checkpoint)
    if state.get("complete") and state.get("next_cursor") is None:
        rows: list[Any] = []
        _atomic_write(output, canonical_json_bytes(rows))
        return rows
    deadline = clock() + config.max_runtime_seconds
    stall_deadline = (
        clock() + config.max_stall_seconds if config.max_stall_seconds is not None else None
    )
    rows: list[Any] = []
    cursor = state.get("next_cursor")
    pages_fetched = int(state.get("pages_fetched", 0))
    while pages_fetched < config.max_pages:
        request_deadline = (
            min(deadline, stall_deadline) if stall_deadline is not None else deadline
        )
        url = _page_url(config, cursor)
        payload = _fetch_json(
            url,
            transport=transport,
            observer=observer,
            deadline=request_deadline,
            clock=clock,
            sleep=sleep,
        )
        page_rows = _extract_rows(payload)
        if not page_rows:
            state["complete"] = True
            state["next_cursor"] = None
            _write_checkpoint(checkpoint, state)
            break
        next_cursor = _extract_cursor(payload)
        rows.extend(page_rows)
        if len(rows) > config.max_rows:
            raise HistoricalIndexError("historical index traversal exceeded the row cap")
        pages_fetched += 1
        state["pages_fetched"] = pages_fetched
        state["rows_collected"] = len(rows)
        state["next_cursor"] = next_cursor
        _write_checkpoint(checkpoint, state)
        if next_cursor is None:
            state["complete"] = True
            _write_checkpoint(checkpoint, state)
            break
        cursor = next_cursor
        if stall_deadline is not None and clock() >= stall_deadline:
            raise HistoricalIndexError("historical index acquisition exceeded the stall deadline")
    result = [
        {
            "rows": rows,
            "pages_fetched": pages_fetched,
            "complete": state.get("complete", False),
        }
    ]
    _atomic_write(output, canonical_json_bytes(result))
    return result
