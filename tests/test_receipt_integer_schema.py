"""Independent Python validation of the shared receipt numeric-schema corpus."""

import json
from pathlib import Path

import pytest

from fyi_system.acquisition_receipts import (
    canonical_json_bytes,
    sha256_bytes,
    validate_receipt,
)

ROOT = Path(__file__).resolve().parents[1]
CORPUS = json.loads(
    (ROOT / "conformance/acquisition/integer-schema-cases-v1.json").read_text(
        encoding="utf-8"
    )
)


@pytest.mark.parametrize("case", CORPUS["cases"], ids=lambda case: case["id"])
def test_python_schema_contract_for_shared_numeric_cases(case):
    receipt = json.loads(
        (ROOT / "conformance/acquisition/receipt-v1-python.json").read_text(
            encoding="utf-8"
        )
    )
    for pointer, replacement in case["patches"].items():
        tokens = pointer.removeprefix("/").split("/")
        target = receipt
        for token in tokens[:-1]:
            target = target[int(token)] if isinstance(target, list) else target[token]
        key = int(tokens[-1]) if isinstance(target, list) else tokens[-1]
        target[key] = replacement
    receipt.pop("receipt_sha256")
    receipt["receipt_sha256"] = sha256_bytes(canonical_json_bytes(receipt))
    if case["schema_valid"]:
        validate_receipt(receipt)
    else:
        with pytest.raises(ValueError, match="invalid acquisition receipt"):
            validate_receipt(receipt)


def test_frozen_decimal_fixture_matches_python_without_rehashing() -> None:
    raw = (ROOT / "conformance/acquisition/receipt-v1-integral-decimal.json").read_bytes()
    receipt = json.loads(raw)
    validate_receipt(receipt)
    assert canonical_json_bytes(receipt) == raw
    assert receipt["responses"][0]["sha256"] == sha256_bytes(b"payload")
    assert receipt["result"]["sha256"] == sha256_bytes(b"[]\n")


def test_integer_conformance_runs_in_cross_platform_receipt_gate() -> None:
    workflow = (ROOT / ".github/workflows/receipt-portability.yml").read_text(
        encoding="utf-8"
    )
    assert "--test receipt_integer_schema \\" in workflow
