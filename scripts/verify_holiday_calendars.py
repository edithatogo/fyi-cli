"""Check holiday-calendar source provenance offline, without certifying legal use."""

from __future__ import annotations

import argparse
import json
import sys
from datetime import date
from pathlib import Path
from typing import Any, cast
from urllib.parse import urlsplit

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
CALENDAR_DIR = ROOT / "crates/fyi-core/locales/calendars/v1"


def _schema_errors(document: object, filename: str) -> list[str]:
    schema = json.loads((CALENDAR_DIR / filename).read_text(encoding="utf-8"))
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    return sorted(
        f"{filename} [{'.'.join(map(str, error.absolute_path)) or '$'}]: {error.message}"
        for error in validator.iter_errors(document)
    )


def _index(
    entries: list[dict[str, Any]],
    key: str,
    label: str,
    errors: list[str],
) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for entry in entries:
        identifier = entry[key]
        if identifier in result:
            errors.append(f"duplicate {label}: {identifier}")
        result[identifier] = entry
    return result


def _check_sources(
    sources: dict[str, dict[str, Any]],
    checked_on: str,
    errors: list[str],
) -> None:
    for source_id, source in sources.items():
        if date.fromisoformat(source["valid_from"]) > date.fromisoformat(source["valid_to"]):
            errors.append(f"{source_id}: reversed source coverage")
        if date.fromisoformat(source["accessed_on"]) > date.fromisoformat(checked_on):
            errors.append(f"{source_id}: accessed_on exceeds sources_checked_on")
        try:
            url = urlsplit(source["url"])
            valid_url = bool(url.hostname) and url.username is None and url.password is None
        except ValueError:
            valid_url = False
        if not valid_url:
            errors.append(f"{source_id}: source URL must have a host and no credentials")


def _check_events(
    entry: dict[str, Any],
    calendar: dict[str, Any],
    sources: dict[str, dict[str, Any]],
    errors: list[str],
) -> None:
    jurisdiction = entry["jurisdiction"]
    declared_sources = set(entry["source_ids"])
    unknown = declared_sources - sources.keys()
    if unknown:
        errors.append(f"{jurisdiction}: unknown source IDs: {sorted(unknown)}")
    seen: set[tuple[str, str]] = set()
    for event in entry["events"]:
        identity = (event["date"], event["name"])
        if identity in seen:
            errors.append(f"{jurisdiction}: duplicate event: {identity}")
        seen.add(identity)
        for source_id in event["source_ids"]:
            if source_id not in declared_sources:
                errors.append(f"{jurisdiction}: event source {source_id} is not declared")
            source = sources.get(source_id)
            if source is None:
                errors.append(f"{jurisdiction}: unknown event source: {source_id}")
                continue
            errors.extend(
                f"{jurisdiction}: {event[field]} outside {source_id} coverage"
                for field in ("date", "observed_for")
                if field in event and not source["valid_from"] <= event[field] <= source["valid_to"]
            )
    dates = {event["date"] for event in entry["events"]}
    exclusions = set(calendar["excluded_dates"])
    if dates != exclusions:
        errors.append(
            f"{jurisdiction}: event dates differ from excluded_dates; "
            f"missing={sorted(exclusions - dates)}, extra={sorted(dates - exclusions)}",
        )


def _check_calendar(
    calendar: dict[str, Any],
    evidence: dict[str, Any] | None,
    sources: dict[str, dict[str, Any]],
    errors: list[str],
) -> None:
    jurisdiction = calendar["jurisdiction"]
    if calendar["valid_from"] > calendar["valid_to"]:
        errors.append(f"{jurisdiction}: reversed calendar coverage")
    if any(
        not calendar["valid_from"] <= day <= calendar["valid_to"]
        for day in calendar["excluded_dates"]
    ):
        errors.append(f"{jurisdiction}: excluded date outside calendar coverage")
    if evidence is None:
        return
    errors.extend(
        f"{jurisdiction}: {field} mismatch"
        for field in ("revision", "valid_from", "valid_to")
        if evidence[field] != calendar[field]
    )
    if set(evidence["weekend_days"]) != set(calendar["weekend_days"]):
        errors.append(f"{jurisdiction}: weekend_days mismatch")
    _check_events(evidence, calendar, sources, errors)


def validate(catalog: object, provenance: object) -> list[str]:
    """Return deterministic errors for schema, coverage and provenance mismatches.

    Both JSON Schemas and all reference checks run locally. A clean result verifies
    the contribution contract; it does not fetch sources or approve legal reliance.
    """
    errors = _schema_errors(catalog, "catalog.schema.json")
    errors.extend(_schema_errors(provenance, "provenance.schema.json"))
    if errors:
        return errors
    catalog = cast("dict[str, Any]", catalog)
    provenance = cast("dict[str, Any]", provenance)
    calendars = _index(catalog["calendars"], "jurisdiction", "calendar", errors)
    reviewed = _index(provenance["calendars"], "jurisdiction", "calendar evidence", errors)
    legacy = _index(provenance["unreviewed_calendars"], "jurisdiction", "legacy exception", errors)
    sources = _index(provenance["sources"], "id", "source ID", errors)
    _check_sources(sources, provenance["sources_checked_on"], errors)
    evidence_keys = reviewed.keys() | legacy.keys()
    if evidence_keys != calendars.keys():
        errors.append(
            f"calendar evidence differs from catalog; "
            f"missing={sorted(calendars.keys() - evidence_keys)}, "
            f"extra={sorted(evidence_keys - calendars.keys())}",
        )
    if reviewed.keys() & legacy.keys():
        errors.append("calendar cannot have both source evidence and an unreviewed exception")
    for jurisdiction, calendar in calendars.items():
        if jurisdiction in legacy and legacy[jurisdiction]["revision"] != calendar["revision"]:
            errors.append(f"{jurisdiction}: unreviewed revision mismatch")
        _check_calendar(calendar, reviewed.get(jurisdiction), sources, errors)
    return sorted(errors)


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            message = f"duplicate JSON field: {key}"
            raise ValueError(message)
        result[key] = value
    return result


def main(argv: list[str] | None = None) -> int:
    """Validate repository defaults or explicit local files; return a CLI exit code."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=CALENDAR_DIR / "catalog.json")
    parser.add_argument("--provenance", type=Path, default=CALENDAR_DIR / "provenance.json")
    args = parser.parse_args(argv)
    try:
        catalog = json.loads(
            args.catalog.read_text(encoding="utf-8"),
            object_pairs_hook=_unique_object,
        )
        provenance = json.loads(
            args.provenance.read_text(encoding="utf-8"),
            object_pairs_hook=_unique_object,
        )
        errors = validate(catalog, provenance)
    except (OSError, UnicodeError, ValueError) as exc:
        errors = [f"cannot validate calendar files: {exc}"]
    sys.stdout.write(f"Holiday calendar provenance: {'FAIL' if errors else 'PASS'}\n")
    for error in errors:
        sys.stderr.write(f"  - {error}\n")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
