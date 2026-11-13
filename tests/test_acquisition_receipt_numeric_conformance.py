"""Independently keep the Rust numeric oracle bound to CPython JSON bytes."""
from __future__ import annotations

import hashlib
import json
import math
import struct
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "conformance" / "acquisition"
CORPUS = json.loads((FIXTURES / "numeric-v1-python.json").read_text(encoding="utf-8"))


def canonical(value: object) -> bytes:
    return (
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
        )
        + "\n"
    ).encode("utf-8")


@pytest.mark.parametrize("case", CORPUS["cases"], ids=lambda case: case["bits"])
def test_frozen_float_bytes_match_cpython(case: dict[str, str]) -> None:
    raw = bytes.fromhex(case["bits"])
    assert len(raw) == 8
    value = struct.unpack(">d", raw)[0]
    assert math.isfinite(value)
    assert canonical(value) == case["canonical"].encode("utf-8")
    assert struct.pack(">d", json.loads(case["canonical"])) == raw


def test_numeric_oracle_is_versioned_unique_and_has_boundary_cases() -> None:
    assert CORPUS["schema"] == "urn:fyi-cli:receipt-numeric-conformance"
    assert CORPUS["schema_version"] == "1.0.0"
    bits = {case["bits"] for case in CORPUS["cases"]}
    assert len(bits) == len(CORPUS["cases"]) == 60
    assert {"0000000000000000", "8000000000000000", "0000000000000001", "7fefffffffffffff"} <= bits


def test_exponent_receipt_uses_unmodified_python_digest_and_bytes() -> None:
    stored = (FIXTURES / "receipt-v1-exponent-python.json").read_bytes()
    receipt = json.loads(stored)
    assert canonical(receipt) == stored
    expected = receipt.pop("receipt_sha256")
    assert hashlib.sha256(canonical(receipt)).hexdigest() == expected
    receipt["rate_limit"]["minimum_interval_seconds"] = 2e-7
    assert hashlib.sha256(canonical(receipt)).hexdigest() != expected
