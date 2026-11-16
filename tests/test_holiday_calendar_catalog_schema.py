"""Validate the embedded holiday calendar catalog against its JSON Schema."""

from __future__ import annotations

import json
from datetime import date
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
CATALOG_DIR = ROOT / "crates/fyi-core/locales/calendars/v1"


def test_embedded_holiday_calendar_catalog_matches_schema_and_invariants() -> None:
    schema = json.loads((CATALOG_DIR / "catalog.schema.json").read_text(encoding="utf-8"))
    catalog = json.loads((CATALOG_DIR / "catalog.json").read_text(encoding="utf-8"))

    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    validator.validate(catalog)

    seen_jurisdictions: set[str] = set()
    for entry in catalog["calendars"]:
        jurisdiction = entry["jurisdiction"]
        assert jurisdiction not in seen_jurisdictions
        seen_jurisdictions.add(jurisdiction)
        assert entry["revision"].startswith("unvalidated-")

        valid_from = date.fromisoformat(entry["valid_from"])
        valid_to = date.fromisoformat(entry["valid_to"])
        assert valid_from <= valid_to
        for excluded in entry["excluded_dates"]:
            excluded_date = date.fromisoformat(excluded)
            assert valid_from <= excluded_date <= valid_to
