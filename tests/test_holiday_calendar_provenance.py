"""Offline source-provenance contracts for contributed holiday calendars."""

from __future__ import annotations

import json
from copy import deepcopy
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator

from scripts.verify_holiday_calendars import main, validate

ROOT = Path(__file__).resolve().parents[1]
CALENDAR_DIR = ROOT / "crates/fyi-core/locales/calendars/v1"


def _documents() -> tuple[dict[str, Any], dict[str, Any]]:
    calendar = {
        "jurisdiction": "NZ-AUK",
        "revision": "unvalidated-synthetic-2026-v1",
        "valid_from": "2026-01-01",
        "valid_to": "2026-12-31",
        "weekend_days": [5, 6],
        "excluded_dates": ["2026-01-01", "2026-01-26"],
    }
    evidence = {key: deepcopy(value) for key, value in calendar.items() if key != "excluded_dates"}
    evidence.update(
        {
            "coverage_kind": "anniversary_area",
            "scope": "Synthetic anniversary area; no live legal claim",
            "limitations": ["Synthetic fixture only"],
            "source_ids": ["synthetic-government"],
            "events": [
                {
                    "date": "2026-01-01",
                    "name": "New Year fixture",
                    "kind": "national",
                    "source_ids": ["synthetic-government"],
                },
                {
                    "date": "2026-01-26",
                    "name": "Anniversary fixture",
                    "kind": "regional",
                    "source_ids": ["synthetic-government"],
                },
            ],
        },
    )
    return {"schema_version": 1, "calendars": [calendar]}, {
        "schema_version": 1,
        "sources_checked_on": "2026-11-17",
        "legal_validation": "unvalidated",
        "sources": [
            {
                "id": "synthetic-government",
                "url": "https://calendar.example/2026",
                "title": "Synthetic 2026 calendar",
                "publisher": "Synthetic publisher",
                "accessed_on": "2026-11-17",
                "valid_from": "2026-01-01",
                "valid_to": "2026-12-31",
            },
        ],
        "calendars": [evidence],
        "unreviewed_calendars": [],
    }


def test_embedded_holiday_calendars_have_complete_source_provenance() -> None:
    catalog = json.loads((CALENDAR_DIR / "catalog.json").read_text(encoding="utf-8"))
    provenance = json.loads((CALENDAR_DIR / "provenance.json").read_text(encoding="utf-8"))
    assert validate(catalog, provenance) == []


def test_valid_source_fixture_and_provenance_schema() -> None:
    schema = json.loads((CALENDAR_DIR / "provenance.schema.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    assert validate(*_documents()) == []


@pytest.mark.parametrize("suffix", ["\n", "\r\n", "\t", " "])
def test_contribution_identifiers_reject_trailing_whitespace(suffix: str) -> None:
    """Regex dollar anchors must not admit JSON that Rust rejects."""
    catalog, provenance = _documents()
    for document in (catalog, provenance):
        document["calendars"][0]["jurisdiction"] += suffix
    assert validate(catalog, provenance)

    catalog, provenance = _documents()
    provenance["sources"][0]["id"] += suffix
    provenance["calendars"][0]["source_ids"][0] += suffix
    for event in provenance["calendars"][0]["events"]:
        event["source_ids"][0] += suffix
    assert validate(catalog, provenance)

    catalog, provenance = _documents()
    provenance["sources"][0]["url"] += suffix
    assert validate(catalog, provenance)


def test_two_distinct_holidays_on_one_date_are_valid() -> None:
    catalog, provenance = _documents()
    event = deepcopy(provenance["calendars"][0]["events"][0])
    event["name"] = "Coincident holiday fixture"
    provenance["calendars"][0]["events"].append(event)
    assert validate(catalog, provenance) == []


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("revision", "different-revision"),
        ("valid_from", "2026-01-02"),
        ("valid_to", "2026-12-30"),
        ("weekend_days", [4, 5]),
    ],
)
def test_calendar_identity_and_coverage_cannot_drift(field: str, value: object) -> None:
    catalog, provenance = _documents()
    provenance["calendars"][0][field] = value
    assert any(f"{field} mismatch" in error for error in validate(catalog, provenance))


@pytest.mark.parametrize(
    ("path", "value"),
    [
        (("unexpected",), True),
        (("schema_version",), 2),
        (("legal_validation",), "approved"),
        (("sources_checked_on",), "2026-02-30"),
        (("sources", 0, "unexpected"), True),
        (("sources", 0, "url"), "http://calendar.example/2026"),
        (("sources", 0, "publisher"), " "),
        (("calendars", 0, "unexpected"), True),
        (("calendars", 0, "coverage_kind"), "complete_everywhere"),
        (("calendars", 0, "scope"), " "),
        (("calendars", 0, "weekend_days"), [5, 5, 6]),
        (("calendars", 0, "source_ids"), []),
        (("calendars", 0, "events", 0, "unexpected"), True),
        (("calendars", 0, "events", 0, "date"), "2026-01-32"),
        (("calendars", 0, "events", 0, "name"), " "),
        (("calendars", 0, "events", 0, "source_ids"), ["synthetic-government"] * 2),
        (("calendars", 0, "events", 0, "observed_for"), "2026-01-01"),
    ],
)
def test_malformed_provenance_fails_schema_validation(
    path: tuple[str | int, ...],
    value: object,
) -> None:
    catalog, provenance = _documents()
    target: Any = provenance
    for component in path[:-1]:
        target = target[component]
    target[path[-1]] = value
    assert any("provenance.schema.json" in error for error in validate(catalog, provenance))


@pytest.mark.parametrize("collection", ["sources", "calendars"])
def test_duplicate_source_and_calendar_ids_are_rejected(collection: str) -> None:
    catalog, provenance = _documents()
    provenance[collection].append(deepcopy(provenance[collection][0]))
    assert any("duplicate" in error for error in validate(catalog, provenance))


def test_duplicate_event_is_rejected_without_losing_coincident_holidays() -> None:
    catalog, provenance = _documents()
    events = provenance["calendars"][0]["events"]
    events.append(deepcopy(events[0]))
    assert any("duplicate event" in error for error in validate(catalog, provenance))


@pytest.mark.parametrize("change", ["missing", "extra"])
def test_calendar_dates_and_evidence_dates_are_conserved(change: str) -> None:
    catalog, provenance = _documents()
    events = provenance["calendars"][0]["events"]
    if change == "missing":
        events.pop()
    else:
        extra = deepcopy(events[0])
        extra["date"] = "2026-02-01"
        events.append(extra)
    assert any("event dates differ" in error for error in validate(catalog, provenance))


@pytest.mark.parametrize("change", ["missing", "extra"])
def test_calendar_scope_evidence_is_conserved(change: str) -> None:
    catalog, provenance = _documents()
    collection = catalog["calendars"] if change == "missing" else provenance["calendars"]
    extra = deepcopy(collection[0])
    extra["jurisdiction"] = "GB-ENG"
    collection.append(extra)
    assert any("calendar evidence differs" in error for error in validate(catalog, provenance))


@pytest.mark.parametrize("location", ["calendar", "event"])
def test_unknown_source_references_fail(location: str) -> None:
    catalog, provenance = _documents()
    entry = provenance["calendars"][0]
    target = entry if location == "calendar" else entry["events"][0]
    target["source_ids"] = ["missing"]
    assert any("unknown" in error for error in validate(catalog, provenance))


def test_event_sources_must_be_declared_by_the_calendar() -> None:
    catalog, provenance = _documents()
    other = deepcopy(provenance["sources"][0])
    other["id"] = "other-source"
    provenance["sources"].append(other)
    provenance["calendars"][0]["events"][0]["source_ids"] = ["other-source"]
    assert any("not declared" in error for error in validate(catalog, provenance))


@pytest.mark.parametrize(
    ("field", "value", "expected"),
    [
        ("accessed_on", "2026-11-18", "exceeds sources_checked_on"),
        ("valid_from", "2026-01-02", "outside synthetic-government coverage"),
        ("valid_to", "2026-01-25", "outside synthetic-government coverage"),
        ("valid_from", "2027-01-01", "reversed source coverage"),
        ("url", "https://user:password@calendar.example/2026", "no credentials"),
    ],
)
def test_source_chronology_and_url_contracts(field: str, value: str, expected: str) -> None:
    catalog, provenance = _documents()
    provenance["sources"][0][field] = value
    assert any(expected in error for error in validate(catalog, provenance))


def test_observed_dates_require_source_coverage_for_actual_and_observed_days() -> None:
    catalog, provenance = _documents()
    event = provenance["calendars"][0]["events"][1]
    event.update(kind="observed", observed_for="2026-01-24")
    assert validate(catalog, provenance) == []
    event["observed_for"] = "2025-12-31"
    assert any("outside synthetic-government coverage" in e for e in validate(catalog, provenance))


@pytest.mark.parametrize("jurisdiction", ["DE", "ES", "FR"])
def test_only_named_european_legacy_entries_can_remain_unreviewed(jurisdiction: str) -> None:
    catalog, provenance = _documents()
    legacy = deepcopy(catalog["calendars"][0])
    legacy["jurisdiction"] = jurisdiction
    catalog["calendars"].append(legacy)
    provenance["unreviewed_calendars"].append(
        {
            "jurisdiction": jurisdiction,
            "revision": legacy["revision"],
            "reason": "Outside this bounded source-verification increment",
        },
    )
    assert validate(catalog, provenance) == []
    provenance["unreviewed_calendars"][0]["revision"] = "drifted"
    assert any("unreviewed revision mismatch" in e for e in validate(catalog, provenance))


@pytest.mark.parametrize("jurisdiction", ["NZ-AUK", "AU", "GB-SCT", "US"])
def test_unreviewed_exception_cannot_expand(jurisdiction: str) -> None:
    catalog, provenance = _documents()
    provenance["unreviewed_calendars"].append(
        {
            "jurisdiction": jurisdiction,
            "revision": "r1",
            "reason": "Attempt to skip evidence",
        },
    )
    assert any("provenance.schema.json" in e for e in validate(catalog, provenance))


def test_duplicate_and_overlapping_legacy_exceptions_fail() -> None:
    catalog, provenance = _documents()
    catalog["calendars"][0]["jurisdiction"] = "DE"
    provenance["calendars"][0]["jurisdiction"] = "DE"
    legacy = {
        "jurisdiction": "DE",
        "revision": catalog["calendars"][0]["revision"],
        "reason": "Synthetic legacy entry",
    }
    provenance["unreviewed_calendars"] = [legacy, deepcopy(legacy)]
    errors = validate(catalog, provenance)
    assert any("duplicate legacy exception" in error for error in errors)
    assert any("both source evidence" in error for error in errors)


@pytest.mark.parametrize(
    ("field", "value", "expected"),
    [
        ("valid_from", "2027-01-01", "reversed calendar coverage"),
        ("excluded_dates", ["2027-01-01"], "outside calendar coverage"),
        ("weekend_days", list(range(7)), "catalog.schema.json"),
        ("excluded_dates", ["2026-01-01", "2026-01-01"], "catalog.schema.json"),
    ],
)
def test_invalid_calendar_inputs_fail(field: str, value: object, expected: str) -> None:
    catalog, provenance = _documents()
    catalog["calendars"][0][field] = value
    assert any(expected in error for error in validate(catalog, provenance))


def test_duplicate_catalog_jurisdiction_fails() -> None:
    catalog, provenance = _documents()
    catalog["calendars"].append(deepcopy(catalog["calendars"][0]))
    assert any("duplicate calendar" in error for error in validate(catalog, provenance))


def test_cli_accepts_explicit_offline_files_from_another_directory(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    catalog, provenance = _documents()
    calendar_path, provenance_path = tmp_path / "calendar.json", tmp_path / "provenance.json"
    calendar_path.write_text(json.dumps(catalog), encoding="utf-8")
    provenance_path.write_text(json.dumps(provenance), encoding="utf-8")
    monkeypatch.chdir(tmp_path)
    assert main(["--catalog", str(calendar_path), "--provenance", str(provenance_path)]) == 0
    assert "PASS" in capsys.readouterr().out


@pytest.mark.parametrize("payload", ["{", '{"schema_version":1,"schema_version":1}'])
def test_cli_rejects_malformed_json_and_duplicate_fields(
    tmp_path: Path,
    payload: str,
    capsys: pytest.CaptureFixture[str],
) -> None:
    calendar_path = tmp_path / "calendar.json"
    calendar_path.write_text(payload, encoding="utf-8")
    assert main(["--catalog", str(calendar_path)]) == 1
    captured = capsys.readouterr()
    assert "FAIL" in captured.out
    assert "cannot validate" in captured.err


def test_cli_rejects_missing_input(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--catalog", str(tmp_path / "missing.json")]) == 1
    assert "cannot validate" in capsys.readouterr().err
