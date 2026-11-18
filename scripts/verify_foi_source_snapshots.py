#!/usr/bin/env python3
"""Validate documentation metadata or compare complete local captures offline.

The public baseline holds reviewed response digests, not redistributed source
pages. Metadata validation never claims byte verification. Supplied observations
must include all six raw bodies, canonical observations and acquisition receipts.
This module performs no network I/O and never changes the reviewed baseline.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import sys
from datetime import UTC, date, datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BASELINE = ROOT / "conformance/providers/foi-source-baseline-v1.json"
MAX_BODY_BYTES = 2_097_152
MAX_METADATA_BYTES = 65_536
SOURCES = {
    "muckrock_client": (
        "muckrock",
        "https://raw.githubusercontent.com/MuckRock/python-muckrock/main/docs/requests.rst",
        frozenset({"text/plain"}),
    ),
    "froide_api": (
        "fragdenstaat",
        "https://raw.githubusercontent.com/okfde/froide/main/docs/api.rst",
        frozenset({"text/plain"}),
    ),
    "foia_developer": (
        "foia_gov",
        "https://www.foia.gov/developer/",
        frozenset({"text/html"}),
    ),
    "uscis_openapi": (
        "uscis",
        "https://developer.uscis.gov/sites/default/files/2026-10/swagger.yaml",
        frozenset(
            {
                "application/yaml",
                "application/x-yaml",
                "text/yaml",
                "text/plain",
                "application/octet-stream",
            },
        ),
    ),
    "canada_guidance": (
        "canada_atip",
        (
            "https://www.canada.ca/en/treasury-board-secretariat/services/"
            "access-information-privacy/access-information/request-information/make-request.html"
        ),
        frozenset({"text/html"}),
    ),
    "india_faq": (
        "india_rti",
        "https://rtionline.gov.in/faq.php",
        frozenset({"text/html"}),
    ),
}
OBSERVATION_FIELDS = {
    "schema_version",
    "adapter",
    "adapter_version",
    "source_id",
    "platform_id",
    "source_url",
    "retrieved_at",
    "http_status",
    "media_type",
    "body_bytes",
    "body_sha256",
    "body_path",
    "receipt_path",
}
SOURCE_FIELDS = OBSERVATION_FIELDS - {
    "schema_version",
    "adapter",
    "adapter_version",
    "body_path",
    "receipt_path",
}


def require(condition: bool, message: str) -> None:  # noqa: FBT001 - predicate, not an option
    """Reject invalid evidence without echoing untrusted source content."""
    if not condition:
        raise ValueError(message)


def canonical(value: object) -> bytes:
    """Match acquisition receipt v1 for this string/integer-only subset."""
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        )
        + "\n"
    ).encode("utf-8")


def _unique(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def _nonfinite(_value: str) -> None:
    message = "nonfinite JSON number"
    raise ValueError(message)


def _read_file(path: Path, limit: int) -> bytes:
    require(not path.is_symlink(), "symlink evidence is not permitted")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    with os.fdopen(os.open(path, flags), "rb") as stream:
        info = os.fstat(stream.fileno())
        require(stat.S_ISREG(info.st_mode), "evidence must be a regular file")
        require(0 < info.st_size <= limit, "evidence file exceeds its bounds or is empty")
        raw = stream.read(limit + 1)
    require(len(raw) == info.st_size and len(raw) <= limit, "evidence size changed")
    return raw


def _json(raw: bytes) -> dict:
    value = json.loads(raw, object_pairs_hook=_unique, parse_constant=_nonfinite)
    require(isinstance(value, dict), "JSON root must be an object")
    return value


def load_json(path: Path) -> dict:
    """Read bounded JSON, rejecting symlinks, duplicates and nonfinite numbers."""
    return _json(_read_file(path, MAX_METADATA_BYTES))


def _day(value: object) -> date:
    require(isinstance(value, str), "date must be a string")
    parsed = date.fromisoformat(value)
    require(parsed.isoformat() == value, "date must use YYYY-MM-DD")
    return parsed


def _timestamp(value: object) -> datetime:
    require(isinstance(value, str), "timestamp must be a string")
    require(
        re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?Z", value) is not None,
        "timestamp must use UTC RFC3339",
    )
    return datetime.fromisoformat(value)


def _source_metadata(item: object, as_of: date) -> datetime:
    require(
        isinstance(item, dict) and set(item) == SOURCE_FIELDS,
        "source fields are incomplete or unknown",
    )
    source_id = item["source_id"]
    require(isinstance(source_id, str) and source_id in SOURCES, "unknown source")
    platform, url, media_types = SOURCES[source_id]
    require(
        item["platform_id"] == platform and item["source_url"] == url,
        "observation source identity mismatch",
    )
    require(
        type(item["http_status"]) is int and item["http_status"] == 200,
        "only successful documentation responses can be evidence",
    )
    require(
        isinstance(item["media_type"], str) and item["media_type"] in media_types,
        "unsupported documentation media type",
    )
    require(
        type(item["body_bytes"]) is int and 0 < item["body_bytes"] <= MAX_BODY_BYTES,
        "invalid response byte length",
    )
    require(
        isinstance(item["body_sha256"], str)
        and re.fullmatch(r"[0-9a-f]{64}", item["body_sha256"]) is not None,
        "invalid response byte fingerprint",
    )
    retrieved = _timestamp(item["retrieved_at"])
    require(retrieved.date() <= as_of, "observation is in the future")
    return retrieved


def _observation(item: object, as_of: date) -> datetime:
    require(
        isinstance(item, dict) and set(item) == OBSERVATION_FIELDS,
        "observation fields are incomplete or unknown",
    )
    require(
        item["schema_version"] == "1.0.0"
        and item["adapter"] == "platform-documentation"
        and item["adapter_version"] == "1.0.0",
        "unsupported observation contract",
    )
    require(
        item["body_path"] == "body.bin" and item["receipt_path"] == "receipt.json",
        "evidence paths must be fixed bundle-local names",
    )
    return _source_metadata({key: item[key] for key in SOURCE_FIELDS}, as_of)


def validate_baseline(data: dict) -> dict:
    """Validate the reviewed public manifest without claiming access to its bodies."""
    require(
        set(data)
        == {
            "schema_version",
            "scope",
            "fingerprint_kind",
            "reviewed_on",
            "network_access",
            "writes_enabled",
            "retention",
            "acquisition_method",
            "sources",
        },
        "baseline fields are incomplete or unknown",
    )
    require(
        data["schema_version"] == "1.0.0" and data["scope"] == "documentation_only",
        "unsupported source baseline",
    )
    require(
        data["fingerprint_kind"] == "raw-response-bytes-sha256",
        "baseline must describe response bytes",
    )
    require(
        data["network_access"] is False and data["writes_enabled"] is False,
        "research metadata must not enable runtime capabilities",
    )
    require(data["retention"] == "operator_local", "raw documentation is operator-local")
    require(
        data["acquisition_method"] in {"managed_proxy_https_review", "guarded_rust_observer"},
        "unsupported baseline acquisition method",
    )
    reviewed = _day(data["reviewed_on"])
    entries = data["sources"]
    require(
        isinstance(entries, list) and len(entries) == len(SOURCES),
        "baseline must cover all six documentation sources",
    )
    seen = set()
    for item in entries:
        _source_metadata(item, reviewed)
        require(item["source_id"] not in seen, "duplicate documentation source")
        seen.add(item["source_id"])
    require(seen == set(SOURCES), "baseline is incomplete")
    return {
        "status": "metadata_valid",
        "byte_verification": "not_performed",
        "source_count": len(entries),
        "reviewed_on": reviewed.isoformat(),
        "acquisition_method": data["acquisition_method"],
        "drift": [],
    }


def _verify_receipt(receipt: dict, entry: dict, observation_bytes: bytes) -> None:
    """Bind the exact documentation receipt subset to its observation and raw body."""
    started = _timestamp(receipt.get("started_at"))
    completed = _timestamp(entry["retrieved_at"])
    require(
        started <= completed <= started + timedelta(seconds=30),
        "receipt acquisition time is outside its bounds",
    )
    expected = {
        "schema": "urn:fyi-cli:acquisition-receipt",
        "schema_version": "1.0.0",
        "command": "observe-platform-docs",
        "adapter": {"id": "platform-documentation", "version": "1.0.0"},
        "source": {"url": entry["source_url"]},
        "request_bounds": {
            "source_id": entry["source_id"],
            "max_requests": 1,
            "max_response_bytes": MAX_BODY_BYTES,
            "timeout_seconds": 30,
            "redirects": 0,
            "retries": 0,
        },
        "started_at": receipt["started_at"],
        "completed_at": entry["retrieved_at"],
        "status": "succeeded",
        "responses": [
            {
                "url": entry["source_url"],
                "status": 200,
                "bytes": entry["body_bytes"],
                "sha256": entry["body_sha256"],
                "attempts": 1,
                "retry_delays_seconds": [],
            },
        ],
        "totals": {"requests": 1, "bytes": entry["body_bytes"], "retries": 0},
        "rate_limit": {"name": None, "minimum_interval_seconds": None},
        "checkpoint": {"before_sha256": None, "after_sha256": None},
        "result": {
            "representation": "canonical_result_projection",
            "media_type": "application/json",
            "bytes": len(observation_bytes),
            "sha256": hashlib.sha256(observation_bytes).hexdigest(),
        },
    }
    expected["receipt_sha256"] = hashlib.sha256(canonical(expected)).hexdigest()
    # Canonical bytes distinguish booleans/floats from integer counters and
    # reject any unrecognised fields rather than accepting Python's True == 1.
    require(canonical(receipt) == canonical(expected), "receipt does not bind this capture")


def _capture_root(root: Path) -> None:
    require(root.is_dir() and not root.is_symlink(), "capture root must be a real directory")
    require(
        not any(parent.is_symlink() for parent in root.absolute().parents),
        "capture root has a symlink ancestor",
    )
    require(
        {path.name for path in root.iterdir()} == set(SOURCES),
        "captures must cover exactly the six approved documentation sources",
    )


def _verify_body(folder: Path, entry: dict) -> None:
    require(folder.is_dir() and not folder.is_symlink(), "capture must be a real directory")
    body = _read_file(folder / "body.bin", MAX_BODY_BYTES)
    require(
        len(body) == entry["body_bytes"]
        and hashlib.sha256(body).hexdigest() == entry["body_sha256"],
        "raw response bytes do not match observation",
    )


def verify_baseline_bodies(data: dict, root: Path) -> dict:
    """Check retained research bytes without attributing them to the Rust observer."""
    report = validate_baseline(data)
    _capture_root(root)
    for entry in data["sources"]:
        _verify_body(root / entry["source_id"], entry)
    report.update(status="baseline_bytes_verified", byte_verification="baseline_verified")
    return report


def compare_snapshots(data: dict, root: Path, *, as_of: date | None = None) -> dict:
    """Verify every supplied capture and report raw-byte/media drift, never refresh."""
    report = validate_baseline(data)
    effective = as_of if as_of is not None else datetime.now(UTC).date()
    require(effective >= _day(data["reviewed_on"]), "baseline is in the future")
    _capture_root(root)
    baseline = {entry["source_id"]: entry for entry in data["sources"]}
    drift = []
    for source_id in SOURCES:
        folder = root / source_id
        require(folder.is_dir() and not folder.is_symlink(), "capture must be a real directory")
        require(
            {path.name for path in folder.iterdir()}
            == {"observation.json", "body.bin", "receipt.json"},
            "capture has incomplete or unknown evidence files",
        )
        raw = _read_file(folder / "observation.json", MAX_METADATA_BYTES)
        entry = _json(raw)
        observed_at = _observation(entry, effective)
        require(entry["source_id"] == source_id, "capture source directory mismatch")
        require(
            observed_at >= _timestamp(baseline[source_id]["retrieved_at"]),
            "observation predates reviewed baseline",
        )
        require(raw == canonical(entry), "observation must use canonical acquisition JSON")
        _verify_body(folder, entry)
        _verify_receipt(load_json(folder / "receipt.json"), entry, raw)
        if any(
            entry[key] != baseline[source_id][key]
            for key in ("body_sha256", "body_bytes", "media_type")
        ):
            drift.append(source_id)
    report.update(
        status="drift" if drift else "verified",
        byte_verification="verified",
        as_of=effective.isoformat(),
        drift=drift,
    )
    return report


def main(argv: list[str] | None = None) -> int:
    """Report bounded JSON and return nonzero for invalid or changed evidence."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    evidence = parser.add_mutually_exclusive_group()
    evidence.add_argument(
        "--observations",
        type=Path,
        help="directory containing the six complete source capture directories",
    )
    evidence.add_argument(
        "--baseline-bodies",
        type=Path,
        help="verify retained baseline body.bin files without claiming Rust receipt verification",
    )
    parser.add_argument(
        "--as-of",
        type=date.fromisoformat,
        help="UTC date for replayable observation freshness validation",
    )
    args = parser.parse_args(argv)
    try:
        data = load_json(args.baseline)
        if args.observations is not None:
            report = compare_snapshots(data, args.observations, as_of=args.as_of)
        elif args.baseline_bodies is not None:
            report = verify_baseline_bodies(data, args.baseline_bodies)
        else:
            report = validate_baseline(data)
    except (OSError, ValueError, TypeError, KeyError, OverflowError, RecursionError):
        sys.stdout.write(
            json.dumps(
                {
                    "status": "invalid",
                    "byte_verification": "not_verified",
                    "error": "invalid or incomplete documentation source evidence",
                },
            )
            + "\n",
        )
        return 1
    sys.stdout.write(json.dumps(report, sort_keys=True) + "\n")
    return int(bool(report["drift"]))


if __name__ == "__main__":
    raise SystemExit(main())
