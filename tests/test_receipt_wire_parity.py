"""Offline receipt byte-ingestion parity with the Rust verifier."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from fyi_system import acquisition_receipts as receipts

ROOT = Path(__file__).resolve().parents[1]
GOLDEN = (ROOT / "conformance/acquisition/receipt-v1-python.json").read_bytes()


def _parse(data: bytes):
    parser = getattr(receipts, "parse_receipt_bytes", None)
    assert callable(parser), "bounded duplicate-safe receipt parser is unavailable"
    return parser(data)


def _duplicate_top_level() -> bytes:
    assert GOLDEN.startswith(b"{")
    return b'{"schema":"urn:fyi-cli:acquisition-receipt",' + GOLDEN[1:]


def _duplicate_nested() -> bytes:
    raw = GOLDEN.replace(
        b'"adapter": {',
        b'"adapter": {"id":"alaveteli-search-feed",',
        1,
    )
    assert raw != GOLDEN
    return raw


def test_bounded_parser_accepts_valid_whitespace_and_key_order() -> None:
    value = json.loads(GOLDEN)
    raw = json.dumps(value, indent=2, ensure_ascii=False).encode("utf-8")
    assert _parse(raw) == value


@pytest.mark.parametrize(
    "raw",
    [
        _duplicate_top_level(),
        b'{"\\u0073chema":"urn:fyi-cli:acquisition-receipt",' + GOLDEN[1:],
        _duplicate_nested(),
        b'{"not_finite":NaN}',
        json.dumps(json.loads(GOLDEN)).encode("utf-16"),
    ],
    ids=["top-level-duplicate", "escaped-duplicate", "nested-duplicate", "nan", "utf16"],
)
def test_bounded_parser_rejects_ambiguous_or_non_utf8_json(raw: bytes) -> None:
    with pytest.raises(ValueError, match="receipt"):
        _parse(raw)


def test_bounded_parser_rejects_oversized_documents() -> None:
    with pytest.raises(ValueError, match="receipt"):
        _parse(GOLDEN + b" " * (16 * 1024 * 1024))


def test_bounded_parser_accepts_exact_byte_limit() -> None:
    raw = GOLDEN + b" " * (16 * 1024 * 1024 - len(GOLDEN))
    assert _parse(raw) == json.loads(GOLDEN)


@pytest.mark.parametrize("number", [-(1 << 63) - 1, 1 << 64])
def test_bounded_parser_rejects_unrepresentable_request_bound_integer(number: int) -> None:
    receipt = json.loads(GOLDEN)
    receipt["request_bounds"] = {"nested": [number]}
    receipt.pop("receipt_sha256")
    receipt["receipt_sha256"] = receipts.sha256_bytes(receipts.canonical_json_bytes(receipt))
    with pytest.raises(ValueError, match="receipt"):
        _parse(receipts.canonical_json_bytes(receipt))


@pytest.mark.parametrize("number", [-(1 << 63), (1 << 64) - 1])
def test_bounded_parser_accepts_request_bound_integer_limits(number: int) -> None:
    receipt = json.loads(GOLDEN)
    receipt["request_bounds"] = {"nested": [number]}
    receipt.pop("receipt_sha256")
    receipt["receipt_sha256"] = receipts.sha256_bytes(receipts.canonical_json_bytes(receipt))
    assert _parse(receipts.canonical_json_bytes(receipt)) == receipt


@pytest.mark.parametrize(("nested_arrays", "valid"), [(125, True), (126, False)])
def test_bounded_parser_matches_rust_recursion_limit(*, nested_arrays: int, valid: bool) -> None:
    receipt = json.loads(GOLDEN)
    nested = 0
    for _ in range(nested_arrays):
        nested = [nested]
    receipt["request_bounds"] = {"nested": nested, "brackets_in_string": '[{"\\\\]'}
    receipt.pop("receipt_sha256")
    receipt["receipt_sha256"] = receipts.sha256_bytes(receipts.canonical_json_bytes(receipt))
    raw = receipts.canonical_json_bytes(receipt)
    if valid:
        assert _parse(raw) == receipt
    else:
        with pytest.raises(ValueError, match="receipt"):
            _parse(raw)


def test_atomic_writer_preserves_existing_ambiguous_receipt(tmp_path: Path) -> None:
    destination = tmp_path / "receipt.json"
    existing = _duplicate_top_level()
    destination.write_bytes(existing)
    with pytest.raises(FileExistsError, match="existing acquisition receipt is invalid"):
        receipts.write_receipt_atomic(destination, json.loads(GOLDEN))
    assert destination.read_bytes() == existing


def test_atomic_writer_preserves_existing_oversized_receipt(tmp_path: Path) -> None:
    destination = tmp_path / "receipt.json"
    existing = GOLDEN + b" " * (16 * 1024 * 1024)
    destination.write_bytes(existing)
    with pytest.raises(FileExistsError, match="existing acquisition receipt is invalid"):
        receipts.write_receipt_atomic(destination, json.loads(GOLDEN))
    assert destination.read_bytes() == existing
