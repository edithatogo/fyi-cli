"""Pin Unicode/escaping canonical JSON and RFC 3339 timestamp cases across Python and Rust."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from fyi_system.acquisition_receipts import canonical_json_bytes, sha256_bytes, validate_receipt

ROOT = Path(__file__).resolve().parents[1]
CORPUS = json.loads(
    (ROOT / "conformance/acquisition/receipt-v1-unicode-utc-cases.json").read_text(
        encoding="utf-8",
    ),
)


@pytest.mark.parametrize(
    "case",
    CORPUS["canonical_cases"],
    ids=lambda case: case["name"],
)
def test_canonical_json_unicode_escaping_and_key_ordering(case: dict[str, Any]) -> None:
    assert CORPUS["schema_version"] == "1.0.0"
    canonical = canonical_json_bytes(case["value"])
    assert canonical.decode("utf-8") == case["expected_canonical"], case["name"]
    assert sha256_bytes(canonical) == case["expected_sha256"], case["name"]


@pytest.mark.parametrize(
    "case",
    CORPUS["timestamp_cases"],
    ids=lambda case: case["name"],
)
def test_timestamp_semantics_match_rust_validator(case: dict[str, Any]) -> None:
    assert CORPUS["schema_version"] == "1.0.0"
    receipt = json.loads(
        (ROOT / "conformance/acquisition/receipt-v1-python.json").read_text(encoding="utf-8"),
    )
    receipt["started_at"] = case["started_at"]
    receipt["completed_at"] = case["completed_at"]
    receipt.pop("receipt_sha256", None)
    receipt["receipt_sha256"] = sha256_bytes(canonical_json_bytes(receipt))
    expected_valid = case["schema_valid"] and case["chronology_valid"]
    if expected_valid:
        validate_receipt(receipt)
    else:
        with pytest.raises(ValueError):
            validate_receipt(receipt)
