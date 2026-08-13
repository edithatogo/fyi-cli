"""Bounded, resumable Internet Archive CDX discovery."""

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
from pathlib import Path
from typing import Any
from urllib.parse import parse_qsl, urlencode, urlsplit

import httpx

from .acquisition_receipts import canonical_json_bytes, sha256_bytes

CDX_ENDPOINT = "https://web.archive.org/cdx/search/cdx"
CDX_ADAPTER_ID = "internet-archive-cdx"
CHECKPOINT_SCHEMA = "urn:fyi-cli:internet-archive-cdx-checkpoint:1"
CAPTURE_MODES = frozenset({"url_index", "all_captures"})
PAGINATION_MODES = frozenset({"page_count", "resume_key"})
DEFAULT_HEADER = ["original", "timestamp", "digest", "statuscode", "length"]
MAX_PAGE_SIZE = 10_000
MAX_PAGES = 10_000
MAX_ROWS = 5_000_000
MAX_RESPONSE_BYTES = 16 * 1024 * 1024
REQUEST_ATTEMPTS = 32
MAX_RETRY_BACKOFF_SECONDS = 60.0
CDX_TIMESTAMP = re.compile(r"^\d{1,14}$")
HOST = re.compile(
    "".join(  # noqa: FLY002
        (
            r"^(?=.{1,253}$)(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)*",
            r"[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$",
        ),
    ),
)

Transport = Callable[[str, float], httpx.Response]
Observer = Callable[[httpx.Response], None]


class CdxDiscoveryError(RuntimeError):
    """Raised when bounded CDX discovery cannot prove complete progress."""


@dataclass(frozen=True)
class CdxConfig:
    """Validated bounds that identify one deterministic CDX traversal."""

    url_pattern: str
    allowed_host: str
    pagination_mode: str
    capture_mode: str
    page_size: int
    max_pages: int
    max_rows: int
    max_runtime_seconds: float
    max_stall_seconds: float | None = None
    from_timestamp: str | None = None
    to_timestamp: str | None = None
    include_urlkey: bool = False

    def __post_init__(self) -> None:
        normalized_host = validate_host(self.allowed_host)
        if self.allowed_host != normalized_host:
            raise ValueError("allowed_host must be a normalized lowercase hostname")
        validate_url_pattern(self.url_pattern, allowed_host=normalized_host)
        if self.pagination_mode not in PAGINATION_MODES:
            raise ValueError(f"unsupported CDX pagination mode: {self.pagination_mode}")
        if self.capture_mode not in CAPTURE_MODES:
            raise ValueError(f"unsupported CDX capture mode: {self.capture_mode}")
        if not 1 <= self.page_size <= MAX_PAGE_SIZE:
            raise ValueError(f"page_size must be between 1 and {MAX_PAGE_SIZE}")
        if not 1 <= self.max_pages <= MAX_PAGES:
            raise ValueError(f"max_pages must be between 1 and {MAX_PAGES}")
        if not 1 <= self.max_rows <= MAX_ROWS:
            raise ValueError(f"max_rows must be between 1 and {MAX_ROWS}")
        if self.max_runtime_seconds <= 0:
            raise ValueError("max_runtime_seconds must be positive")
        if self.max_stall_seconds is not None and self.max_stall_seconds <= 0:
            raise ValueError("max_stall_seconds must be positive")
        validate_time_range(self.from_timestamp, self.to_timestamp)

    def identity(self) -> dict[str, Any]:
        """Return the checkpoint-bound subset of configuration."""
        return {
            "endpoint": CDX_ENDPOINT,
            "url_pattern": self.url_pattern,
            "allowed_host": self.allowed_host,
            "pagination_mode": self.pagination_mode,
            "capture_mode": self.capture_mode,
            "page_size": self.page_size,
            "from_timestamp": self.from_timestamp,
            "to_timestamp": self.to_timestamp,
            "include_urlkey": self.include_urlkey,
        }

    def request_bounds(self) -> dict[str, Any]:
        """Return all immutable query settings and per-run safety caps."""
        return {
            **self.identity(),
            "max_pages": self.max_pages,
            "max_rows": self.max_rows,
            "max_runtime_seconds": self.max_runtime_seconds,
            "max_stall_seconds": self.max_stall_seconds,
        }


@dataclass
class _State:
    header: list[str] | None
    rows: list[list[str]]
    fingerprints: set[str]
    next_index: int
    next_resume_key: str | None
    reported_page_count: int | None
    complete: bool


def validate_host(value: str) -> str:
    """Validate and normalize a DNS hostname used as the query boundary."""
    normalized = value.rstrip(".").lower()
    if not HOST.fullmatch(normalized) or normalized == "localhost":
        raise ValueError("allowed_host must be a valid public DNS hostname")
    return normalized


def validate_url_pattern(value: str, *, allowed_host: str) -> None:
    """Require a scheme-free CDX pattern whose host exactly matches its bound."""
    if any(character.isspace() or ord(character) < 32 for character in value):
        raise ValueError("url_pattern must not contain whitespace or control characters")
    if "\\" in value or "%" in value or "://" in value:
        raise ValueError("url_pattern must be an unescaped, scheme-free host/path pattern")
    parsed = urlsplit(f"https://{value}")
    if (
        parsed.hostname != allowed_host
        or parsed.username is not None
        or parsed.password is not None
        or parsed.port is not None
        or parsed.query
        or parsed.fragment
    ):
        raise ValueError("url_pattern must stay within allowed_host")
    if any(segment == ".." for segment in parsed.path.split("/")):
        raise ValueError("url_pattern must not contain parent path segments")


def validate_time_range(from_timestamp: str | None, to_timestamp: str | None) -> None:
    """Validate optional CDX timestamp partitions."""
    for name, value in (("from_timestamp", from_timestamp), ("to_timestamp", to_timestamp)):
        if value is not None and not CDX_TIMESTAMP.fullmatch(value):
            raise ValueError(f"{name} must contain 1 to 14 digits")
    if from_timestamp is not None and to_timestamp is not None:
        width = max(len(from_timestamp), len(to_timestamp))
        if from_timestamp.ljust(width, "0") > to_timestamp.ljust(width, "9"):
            raise ValueError("from_timestamp must not be later than to_timestamp")


def default_transport(url: str, timeout: float) -> httpx.Response:
    """Issue one redirect-free request to the fixed CDX endpoint."""
    with httpx.stream(
        "GET",
        url,
        headers={"User-Agent": "fyi-cli-cdx-discovery/1.0"},
        timeout=timeout,
        follow_redirects=False,
    ) as response:
        declared_length = response.headers.get("Content-Length")
        if declared_length is not None:
            try:
                if int(declared_length) > MAX_RESPONSE_BYTES:
                    raise CdxDiscoveryError("CDX response exceeded the byte cap")
            except ValueError:
                pass

        chunks: list[bytes] = []
        received = 0
        for chunk in response.iter_bytes():
            received += len(chunk)
            if received > MAX_RESPONSE_BYTES:
                raise CdxDiscoveryError("CDX response exceeded the byte cap")
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


def _config_digest(config: CdxConfig) -> str:
    return sha256_bytes(canonical_json_bytes(config.identity()))


def _checkpoint_value(config: CdxConfig, state: _State) -> dict[str, Any]:
    value: dict[str, Any] = {
        "schema": CHECKPOINT_SCHEMA,
        "config_sha256": _config_digest(config),
        "config": config.identity(),
        "operational_bounds": {
            "max_pages": config.max_pages,
            "max_rows": config.max_rows,
        },
        "header": state.header,
        "rows": state.rows,
        "fingerprints": sorted(state.fingerprints),
        "next_index": state.next_index,
        "next_resume_key": state.next_resume_key,
        "reported_page_count": state.reported_page_count,
        "complete": state.complete,
    }
    unsigned = canonical_json_bytes(value)
    value["checkpoint_sha256"] = sha256_bytes(unsigned)
    return value


def _write_checkpoint(path: Path, config: CdxConfig, state: _State) -> None:
    _atomic_write(path, canonical_json_bytes(_checkpoint_value(config, state)))


def _load_checkpoint(path: Path, config: CdxConfig) -> _State:  # noqa: C901
    if not path.exists():
        return _State(
            header=None,
            rows=[],
            fingerprints=set(),
            next_index=0,
            next_resume_key=None,
            reported_page_count=None,
            complete=False,
        )
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
    if value.get("config") != config.identity() or value.get("config_sha256") != _config_digest(
        config,
    ):
        raise ValueError("checkpoint configuration does not match this traversal")
    prior_bounds = value.get("operational_bounds")
    if not isinstance(prior_bounds, dict) or not all(
        isinstance(prior_bounds.get(name), int)
        and not isinstance(prior_bounds.get(name), bool)
        and prior_bounds[name] > 0
        for name in ("max_pages", "max_rows")
    ):
        raise TypeError("checkpoint operational bounds are invalid")
    header_value = value.get("header")
    header = None if header_value is None else _string_row(header_value, "checkpoint header")
    raw_rows = value.get("rows")
    if not isinstance(raw_rows, list):
        raise TypeError("checkpoint rows must be an array")
    rows = [_string_row(row, "checkpoint row") for row in raw_rows]
    if len(rows) > config.max_rows:
        raise ValueError("checkpoint exceeds configured row cap")
    fingerprints_value = value.get("fingerprints")
    if not isinstance(fingerprints_value, list) or not all(
        isinstance(item, str) and re.fullmatch(r"[0-9a-f]{64}", item) for item in fingerprints_value
    ):
        raise ValueError("checkpoint fingerprints are invalid")
    fingerprints = set(fingerprints_value)
    if len(fingerprints) != len(fingerprints_value):
        raise ValueError("checkpoint fingerprints contain duplicates")
    next_index = value.get("next_index")
    if (
        not isinstance(next_index, int)
        or isinstance(next_index, bool)
        or not 0 <= next_index <= config.max_pages
    ):
        raise ValueError("checkpoint next_index is invalid")
    next_resume_key = value.get("next_resume_key")
    if next_resume_key is not None and (
        not isinstance(next_resume_key, str) or not next_resume_key or len(next_resume_key) > 4096
    ):
        raise ValueError("checkpoint resume key is invalid")
    reported_page_count = value.get("reported_page_count")
    if reported_page_count is not None and (
        not isinstance(reported_page_count, int)
        or isinstance(reported_page_count, bool)
        or not 0 <= reported_page_count <= config.max_pages
    ):
        raise ValueError("checkpoint reported page count is invalid")
    complete = value.get("complete")
    if not isinstance(complete, bool):
        raise TypeError("checkpoint completion flag is invalid")
    if (
        config.pagination_mode == "resume_key"
        and next_index
        and not complete
        and not next_resume_key
    ):
        raise ValueError("incomplete resume-key checkpoint has no cursor")
    return _State(
        header,
        rows,
        fingerprints,
        next_index,
        next_resume_key,
        reported_page_count,
        complete,
    )


def _string_row(value: object, label: str) -> list[str]:
    if not isinstance(value, list) or not all(
        isinstance(item, (str, int)) and not isinstance(item, bool) for item in value
    ):
        raise ValueError(f"{label} must contain only strings or integers")
    return [str(item) for item in value]


def _base_params(config: CdxConfig) -> list[tuple[str, str]]:
    fields = (
        "urlkey,original,timestamp,digest,statuscode,length"
        if config.include_urlkey
        else "original,timestamp,digest,statuscode,length"
    )
    params = [
        ("url", config.url_pattern),
        ("output", "json"),
        ("filter", "statuscode:200"),
        ("fl", fields),
        ("limit", str(config.page_size)),
    ]
    if config.capture_mode == "url_index":
        params.append(("collapse", "urlkey"))
    if config.from_timestamp is not None:
        params.append(("from", config.from_timestamp))
    if config.to_timestamp is not None:
        params.append(("to", config.to_timestamp))
    return params


def _request_url(params: list[tuple[str, str]]) -> str:
    return f"{CDX_ENDPOINT}?{urlencode(params)}"


def _verify_response_url(response: httpx.Response, requested_url: str) -> None:
    actual = urlsplit(str(response.url))
    requested = urlsplit(requested_url)
    if (
        actual.scheme != "https"
        or actual.hostname != "web.archive.org"
        or actual.port not in {None, 443}
        or actual.username is not None
        or actual.password is not None
        or actual.path != "/cdx/search/cdx"
        or parse_qsl(actual.query, keep_blank_values=True)
        != parse_qsl(requested.query, keep_blank_values=True)
    ):
        raise CdxDiscoveryError("CDX response escaped the fixed request boundary")


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


def _fetch_json(  # noqa: C901
    params: list[tuple[str, str]],
    *,
    transport: Transport,
    observer: Observer | None,
    deadline: float,
    clock: Callable[[], float],
    sleep: Callable[[float], None],
    retry_page_400: bool,
) -> Any:
    url = _request_url(params)
    delays: list[float] = []
    last_error: Exception | None = None
    last_response: httpx.Response | None = None
    for attempt in range(REQUEST_ATTEMPTS):
        remaining = deadline - clock()
        if remaining <= 0:
            raise CdxDiscoveryError("CDX acquisition exceeded its deadline") from last_error
        response: httpx.Response | None = None
        try:
            response = transport(url, min(60.0, remaining))
            last_response = response
            _verify_response_url(response, url)
            retryable = response.status_code in {429, 500, 502, 503, 504} or (
                retry_page_400 and response.status_code == 400
            )
            if response.status_code != 200:
                if not retryable:
                    _observe(response, observer, attempt + 1, delays)
                    raise CdxDiscoveryError(f"CDX returned HTTP {response.status_code}")
                last_error = CdxDiscoveryError(f"CDX returned HTTP {response.status_code}")
            elif len(response.content) > MAX_RESPONSE_BYTES:
                _observe(response, observer, attempt + 1, delays)
                raise CdxDiscoveryError("CDX response exceeded the byte cap")
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
    raise CdxDiscoveryError("CDX request failed after bounded retries") from last_error


def _parse_page(
    payload: object,
    *,
    expected_header: list[str] | None,
    label: str,
) -> tuple[list[str], list[list[str]]]:
    if not isinstance(payload, list) or not payload or not isinstance(payload[0], list):
        raise CdxDiscoveryError(f"CDX returned an invalid {label} payload")
    header = _string_row(payload[0], f"{label} header")
    if expected_header is not None and header != expected_header:
        raise CdxDiscoveryError(f"CDX {label} header changed during acquisition")
    rows = [_string_row(row, f"{label} row") for row in payload[1:]]
    if any(len(row) != len(header) for row in rows):
        raise CdxDiscoveryError(f"CDX {label} row width differs from its header")
    return header, rows


def _add_rows(config: CdxConfig, state: _State, rows: list[list[str]], label: str) -> str:
    fingerprint = sha256_bytes(canonical_json_bytes(rows))
    if fingerprint in state.fingerprints:
        raise CdxDiscoveryError(f"CDX {label} repeated during acquisition")
    if len(state.rows) + len(rows) > config.max_rows:
        raise CdxDiscoveryError("CDX traversal exceeded the configured row cap")
    state.fingerprints.add(fingerprint)
    state.rows.extend(rows)
    return fingerprint


def _page_count_traversal(
    config: CdxConfig,
    state: _State,
    checkpoint: Path,
    *,
    transport: Transport,
    observer: Observer | None,
    deadline: float,
    clock: Callable[[], float],
    sleep: Callable[[float], None],
) -> None:
    base = _base_params(config)
    count_payload = _fetch_json(
        [*base, ("showNumPages", "true")],
        transport=transport,
        observer=observer,
        deadline=deadline,
        clock=clock,
        sleep=sleep,
        retry_page_400=False,
    )
    try:
        raw_count = count_payload[1][0]
        page_count = None if raw_count is None else int(raw_count)
    except (IndexError, TypeError, ValueError) as error:
        raise CdxDiscoveryError("CDX returned an invalid page count") from error
    if page_count is not None and not 0 <= page_count <= config.max_pages:
        raise CdxDiscoveryError("CDX reported a page count outside the configured cap")
    if state.reported_page_count is not None and page_count != state.reported_page_count:
        raise CdxDiscoveryError("CDX page count changed since checkpoint creation")
    if page_count is not None and state.next_index > page_count:
        raise CdxDiscoveryError("checkpoint starts beyond reported CDX coverage")
    state.reported_page_count = page_count
    while page_count is None or state.next_index < page_count:
        if state.next_index >= config.max_pages:
            raise CdxDiscoveryError("CDX traversal reached the configured page cap")
        page = state.next_index
        payload = _fetch_json(
            [*base, ("page", str(page))],
            transport=transport,
            observer=observer,
            deadline=deadline,
            clock=clock,
            sleep=sleep,
            retry_page_400=True,
        )
        if payload == [] or payload == [[]]:
            if page_count is None:
                state.complete = True
                _write_checkpoint(checkpoint, config, state)
                return
            raise CdxDiscoveryError("CDX page was empty before reported coverage completed")
        header, rows = _parse_page(payload, expected_header=state.header, label="page")
        if not rows:
            raise CdxDiscoveryError("CDX page was empty before reported coverage completed")
        state.header = header
        _add_rows(config, state, rows, "page")
        state.next_index += 1
        state.complete = page_count is not None and state.next_index == page_count
        _write_checkpoint(checkpoint, config, state)
    state.complete = True
    _write_checkpoint(checkpoint, config, state)


def _split_resume_payload(
    payload: object,
    expected_header: list[str] | None,
) -> tuple[list[str], list[list[str]], str | None]:
    if payload == []:
        return expected_header or DEFAULT_HEADER, [], None
    if not isinstance(payload, list) or not payload or not isinstance(payload[0], list):
        raise CdxDiscoveryError("CDX returned an invalid resume-key payload")
    next_key: str | None = None
    data_end = len(payload)
    if len(payload) >= 2 and payload[-2] == []:
        marker = payload[-1]
        if not isinstance(marker, list) or len(marker) != 1 or not isinstance(marker[0], str):
            raise CdxDiscoveryError("CDX returned an invalid resumption key")
        if not marker[0] or len(marker[0]) > 4096:
            raise CdxDiscoveryError("CDX returned an invalid resumption key")
        next_key = marker[0]
        data_end -= 2
    elif any(value == [] for value in payload[1:]):
        raise CdxDiscoveryError("CDX returned a malformed resumption-key separator")
    header, rows = _parse_page(payload[:data_end], expected_header=expected_header, label="chunk")
    if next_key is not None and not rows:
        raise CdxDiscoveryError("CDX returned a resumption key without records")
    return header, rows, next_key


def _resume_key_traversal(
    config: CdxConfig,
    state: _State,
    checkpoint: Path,
    *,
    transport: Transport,
    observer: Observer | None,
    deadline: float,
    clock: Callable[[], float],
    sleep: Callable[[float], None],
) -> None:
    base = [*_base_params(config), ("showResumeKey", "true")]
    seen_keys = {state.next_resume_key} if state.next_resume_key else set()
    stall_deadline = (
        clock() + config.max_stall_seconds if config.max_stall_seconds is not None else None
    )
    while state.next_index < config.max_pages:
        params = [*base]
        if state.next_resume_key is not None:
            params.append(("resumeKey", state.next_resume_key))
        request_deadline = min(deadline, stall_deadline) if stall_deadline is not None else deadline
        try:
            payload = _fetch_json(
                params,
                transport=transport,
                observer=observer,
                deadline=request_deadline,
                clock=clock,
                sleep=sleep,
                retry_page_400=False,
            )
        except CdxDiscoveryError as error:
            if stall_deadline is not None and clock() >= stall_deadline:
                raise CdxDiscoveryError(
                    "CDX acquisition exceeded the progress-stall deadline",
                ) from error
            raise
        header, rows, next_key = _split_resume_payload(payload, state.header)
        if not rows and next_key is None:
            state.header = header
            state.complete = True
            _write_checkpoint(checkpoint, config, state)
            return
        state.header = header
        _add_rows(config, state, rows, "chunk")
        if next_key is not None and next_key in seen_keys:
            raise CdxDiscoveryError("CDX resumption key repeated during acquisition")
        if next_key is not None:
            seen_keys.add(next_key)
        state.next_index += 1
        state.next_resume_key = next_key
        state.complete = next_key is None
        _write_checkpoint(checkpoint, config, state)
        if state.complete:
            return
        if config.max_stall_seconds is not None:
            stall_deadline = clock() + config.max_stall_seconds
    raise CdxDiscoveryError("CDX traversal reached the configured chunk cap")


def discover_cdx(
    config: CdxConfig,
    *,
    output_path: str | Path,
    checkpoint_path: str | Path,
    transport: Transport = default_transport,
    observer: Observer | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> list[list[str]]:
    """Discover a complete CDX inventory and atomically persist proof of progress."""
    output = Path(output_path)
    checkpoint = Path(checkpoint_path)
    if output.resolve(strict=False) == checkpoint.resolve(strict=False):
        raise ValueError("output and checkpoint paths must differ")
    state = _load_checkpoint(checkpoint, config)
    if not state.complete:
        deadline = clock() + config.max_runtime_seconds
        if config.pagination_mode == "page_count":
            _page_count_traversal(
                config,
                state,
                checkpoint,
                transport=transport,
                observer=observer,
                deadline=deadline,
                clock=clock,
                sleep=sleep,
            )
        else:
            _resume_key_traversal(
                config,
                state,
                checkpoint,
                transport=transport,
                observer=observer,
                deadline=deadline,
                clock=clock,
                sleep=sleep,
            )
    header = state.header or (
        ["urlkey", *DEFAULT_HEADER] if config.include_urlkey else list(DEFAULT_HEADER)
    )
    result = [header, *state.rows]
    _atomic_write(output, canonical_json_bytes(result))
    return result
