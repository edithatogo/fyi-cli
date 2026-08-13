"""Pure, bounded fuzz targets for untrusted acquisition data."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

from fyi_system.acquisition_receipts import sanitize_url, validate_receipt
from fyi_system.internet_archive_cdx import (
    CdxDiscoveryError,
    _parse_page,
    _split_resume_payload,
)
from fyi_system.internet_archive_replay import validate_selection
from fyi_system.security import redact_text, sanitize_payload

if TYPE_CHECKING:
    from collections.abc import Callable

MAX_INPUT_BYTES = 64 * 1024
MAX_JSON_DEPTH = 32
MAX_JSON_NODES = 4_096

EXPECTED_INPUT_ERRORS = (
    CdxDiscoveryError,
    RecursionError,
    TypeError,
    UnicodeDecodeError,
    ValueError,
    json.JSONDecodeError,
)


def _bounded_json(data: bytes) -> Any:
    if len(data) > MAX_INPUT_BYTES:
        return None
    value = json.loads(data.decode("utf-8"))
    pending: list[tuple[Any, int]] = [(value, 0)]
    nodes = 0
    while pending:
        current, depth = pending.pop()
        nodes += 1
        if nodes > MAX_JSON_NODES or depth > MAX_JSON_DEPTH:
            message = "JSON structure exceeds fuzz harness bounds"
            raise ValueError(message)
        if isinstance(current, dict):
            pending.extend((key, depth + 1) for key in current)
            pending.extend((item, depth + 1) for item in current.values())
        elif isinstance(current, list):
            pending.extend((item, depth + 1) for item in current)
    return value


def fuzz_receipt(data: bytes) -> None:
    """Exercise URL scrubbing and acquisition receipt schema validation."""
    if len(data) > MAX_INPUT_BYTES:
        return
    try:
        sanitize_url(data.decode("utf-8"))
        value = _bounded_json(data)
        if isinstance(value, dict):
            validate_receipt(value)
    except EXPECTED_INPUT_ERRORS:
        return


def fuzz_cdx(data: bytes) -> None:
    """Exercise both bounded CDX page formats."""
    try:
        value = _bounded_json(data)
        _parse_page(value, expected_header=None, label="fuzz")
        _split_resume_payload(value, None)
    except EXPECTED_INPUT_ERRORS:
        return


def fuzz_wayback(data: bytes) -> None:
    """Exercise replay selection schema, URL, ordering, and digest checks."""
    try:
        value = _bounded_json(data)
        if isinstance(value, dict):
            validate_selection(value)
    except EXPECTED_INPUT_ERRORS:
        return


def fuzz_redaction(data: bytes) -> None:
    """Exercise recursive payload sanitization and text redaction."""
    if len(data) > MAX_INPUT_BYTES:
        return
    try:
        redact_text(data.decode("utf-8"))
        sanitize_payload(_bounded_json(data))
    except EXPECTED_INPUT_ERRORS:
        return


TARGETS: dict[str, Callable[[bytes], None]] = {
    "receipt": fuzz_receipt,
    "cdx": fuzz_cdx,
    "wayback": fuzz_wayback,
    "redaction": fuzz_redaction,
}
