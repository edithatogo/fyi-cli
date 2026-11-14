"""Independent CPython oracle for shortest-decimal ties-to-even cases."""
from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

import pytest

FIXTURES = Path(__file__).resolve().parents[1] / "conformance" / "acquisition"
CORPUS = json.loads((FIXTURES / "numeric-v1-midpoints-python.json").read_text())


def canonical(value: object) -> bytes:
    return (
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
        )
        + "\n"
    ).encode("utf-8")


@pytest.mark.parametrize("case", CORPUS["cases"], ids=lambda case: case["bits"])
def test_frozen_midpoint_bytes_match_cpython(case: dict[str, str]) -> None:
    bits = bytes.fromhex(case["bits"])
    value = struct.unpack(">d", bits)[0]
    assert canonical(value) == case["canonical"].encode("utf-8")
    assert struct.pack(">d", json.loads(case["canonical"])) == bits


def test_midpoint_corpus_is_versioned_and_unique() -> None:
    assert CORPUS["schema"] == "urn:fyi-cli:receipt-numeric-conformance"
    assert CORPUS["schema_version"] == "1.0.0"
    assert len(CORPUS["cases"]) == 15
    assert len({case["bits"] for case in CORPUS["cases"]}) == 15


def test_midpoint_receipt_keeps_its_independent_python_digest() -> None:
    stored = (FIXTURES / "receipt-v1-midpoints-python.json").read_bytes()
    receipt = json.loads(stored)
    assert canonical(receipt) == stored
    expected = receipt.pop("receipt_sha256")
    assert hashlib.sha256(canonical(receipt)).hexdigest() == expected
