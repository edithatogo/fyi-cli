"""Shared semantic receipt cases, independently exercised in Rust and Python."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from fyi_system.acquisition_receipts import canonical_json_bytes, sha256_bytes, validate_receipt

ROOT = Path(__file__).resolve().parents[1]
CORPUS = json.loads(
    (ROOT / "conformance/acquisition/receipt-verifier-parity-v1.json").read_text(encoding="utf-8"),
)


def _case_receipt(case: dict[str, Any]) -> dict[str, Any]:
    receipt = json.loads((ROOT / "conformance/acquisition/receipt-v1-python.json").read_bytes())
    for pointer, replacement in case["patches"].items():
        tokens = pointer.removeprefix("/").split("/")
        target = receipt
        for token in tokens[:-1]:
            target = target[int(token)] if isinstance(target, list) else target[token]
        final = int(tokens[-1]) if isinstance(target, list) else tokens[-1]
        target[final] = replacement
    receipt.pop("receipt_sha256")
    receipt["receipt_sha256"] = sha256_bytes(canonical_json_bytes(receipt))
    return receipt


@pytest.mark.parametrize("case", CORPUS["cases"], ids=lambda case: case["id"])
def test_receipt_semantics_match_the_shared_contract(case: dict[str, Any]) -> None:
    assert CORPUS["contract"] == "urn:fyi-cli:receipt-verifier-parity:1"
    receipt = _case_receipt(case)
    if case["valid"]:
        validate_receipt(receipt)
    else:
        with pytest.raises(ValueError, match="invalid acquisition receipt"):
            validate_receipt(receipt)


def test_shared_corpus_is_in_cross_platform_receipt_gate() -> None:
    workflow = (ROOT / ".github/workflows/receipt-portability.yml").read_text(encoding="utf-8")
    assert "--test receipt_verifier_parity " in workflow
