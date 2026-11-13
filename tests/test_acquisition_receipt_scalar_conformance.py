"""Pin shared Rust receipt scalar cases to the packaged v1 JSON Schema."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
CORPUS = json.loads(
    (ROOT / "conformance/acquisition/receipt-v1-scalar-cases.json").read_text(encoding="utf-8"),
)
SCHEMA = json.loads(
    (ROOT / "src/fyi_system/schemas/acquisition-receipt-v1.schema.json").read_text(
        encoding="utf-8",
    ),
)
VALIDATOR = Draft202012Validator(SCHEMA, format_checker=FormatChecker())


@pytest.mark.parametrize("case", CORPUS["cases"], ids=lambda case: case["name"])
def test_scalar_case_matches_packaged_schema(case: dict[str, Any]) -> None:
    assert CORPUS["schema_version"] == "1.0.0"
    receipt = json.loads(
        (ROOT / "conformance/acquisition/receipt-v1-python.json").read_text(encoding="utf-8"),
    )
    parent, key = case["pointer"].lstrip("/").split("/")
    receipt[parent][key] = case["value"]
    assert VALIDATOR.is_valid(receipt) == case["valid"], case["name"]
