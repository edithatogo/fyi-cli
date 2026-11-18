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


def test_2026_source_corrections_and_exact_regional_scope() -> None:
    """Protect corrections against the linked 2026 government holiday lists."""
    catalog = json.loads((CATALOG_DIR / "catalog.json").read_text(encoding="utf-8"))
    calendars = {
        entry["jurisdiction"]: set(entry["excluded_dates"]) for entry in catalog["calendars"]
    }

    assert "2026-04-27" in calendars["AU-ACT"]
    assert "2026-04-27" in calendars["AU-NSW"]
    assert "2026-09-25" in calendars["AU-VIC"]
    assert "2026-06-15" in calendars["GB-SCT"]
    assert "GB" not in calendars
    assert calendars["GB-ENG"] == calendars["GB-WLS"]
    assert "2026-04-06" not in calendars["GB-SCT"]

    # Local dates are opt-in; replacements must remove the state's default date.
    assert "2026-07-17" not in calendars["AU-QLD"]
    assert "2026-07-17" in calendars["AU-QLD-CAIRNS"]
    assert "2026-08-12" in calendars["AU-QLD-BRISBANE"]
    assert "2026-08-12" not in calendars["AU-QLD-CAIRNS"]
    assert "2026-08-03" in calendars["AU-WA-KARRATHA"]
    assert "2026-09-28" not in calendars["AU-WA-KARRATHA"]
    assert "2026-09-28" in calendars["AU-WA"]


def test_bank_only_and_partial_holidays_are_not_whole_day_exclusions() -> None:
    """Employment, banking, local and partial-day applicability must not leak."""
    catalog = json.loads((CATALOG_DIR / "catalog.json").read_text(encoding="utf-8"))
    calendars = {
        entry["jurisdiction"]: set(entry["excluded_dates"]) for entry in catalog["calendars"]
    }
    for jurisdiction in ("AU-ACT", "AU-NSW"):
        assert "2026-08-03" not in calendars[jurisdiction]
    for jurisdiction in ("AU-QLD", "AU-SA", "AU-NT"):
        assert "2026-12-24" not in calendars[jurisdiction]
        assert "2026-12-31" not in calendars[jurisdiction]
    for jurisdiction in ("AU-QLD", "AU-VIC", "AU-SA", "AU-NT", "AU-TAS"):
        assert "2026-04-27" not in calendars[jurisdiction]
    assert "2026-04-07" not in calendars["AU-TAS"]
    assert "2026-12-26" not in calendars["AU-TAS"]
    assert "2026-04-04" not in calendars["AU-WA"]
