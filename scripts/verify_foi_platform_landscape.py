#!/usr/bin/env python3
"""Validate reviewed platform research and compare supplied observations offline.

No HTTP requests, credentials, provider enablement or automatic baseline updates.
Fingerprints cover reviewed contract projections, not remote schema bytes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import date, timedelta
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INVENTORY = ROOT / "conformance/providers/foi-platform-landscape-v1.json"
PLATFORMS = {
    "muckrock": ("read_only_pilot", {"www.muckrock.com", "help.muckrock.com",
                                      "python-muckrock.readthedocs.io"}),
    "fragdenstaat": ("read_only_pilot", {"fragdenstaat.de", "froide.readthedocs.io"}),
    "foia_gov": ("discovery_only", {"www.foia.gov"}),
    "uscis": ("defer", {"developer.uscis.gov", "www.uscis.gov"}),
    "canada_atip": ("manual_only", {"www.canada.ca", "atip-aiprp.apps.gc.ca"}),
    "india_rti": ("manual_only", {"rtionline.gov.in", "www.rtionline.gov.in"}),
}
CONTRACT_FIELDS = {"public_api", "read_surface", "authentication", "write_semantics",
                   "rate_limit", "privacy", "availability"}
PLATFORM_FIELDS = {"id", "name", "rank", "decision", "reason", "jurisdiction", "lineage",
                   "next_experiment", "confidence", "reviewed_on", "review_interval_days",
                   "next_review_on", "sources", "contract", "contract_sha256"}


def require(condition: bool, message: str) -> None:
    """Fail with bounded metadata, never arbitrary input payloads."""
    if not condition:
        raise ValueError(message)


def _object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def _nonfinite(_value: str) -> None:
    raise ValueError("nonfinite JSON number")


def load_json(path: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_object,
                       parse_constant=_nonfinite)
    require(isinstance(value, dict), "JSON root must be an object")
    return value


def fingerprint(contract: dict) -> str:
    """Hash the documented semantic projection; this is not a signature."""
    payload = json.dumps(contract, sort_keys=True, ensure_ascii=False,
                         separators=(",", ":"), allow_nan=False).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _day(value: object) -> date:
    require(isinstance(value, str), "review date must be an ISO date")
    parsed = date.fromisoformat(value)
    require(parsed.isoformat() == value, "review date must use YYYY-MM-DD")
    return parsed


def _text(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _contract(value: object) -> None:
    require(isinstance(value, dict) and set(value) == CONTRACT_FIELDS,
            "contract fields are incomplete or unknown")
    require(all(_text(v) for v in value.values()), "contract values must be nonempty text")
    require(value["public_api"] in {"documented", "not_identified"}, "invalid API evidence state")
    require(value["availability"] == "documentation_review_only",
            "research cannot claim live API availability")


def validate_inventory(data: dict, *, as_of: date | None = None) -> dict:
    """Validate structure and return freshness separately from historical validity."""
    require(set(data) == {"schema_version", "reviewed_on", "scope", "excluded_lineages",
                          "network_access", "writes_enabled", "fingerprint_kind", "platforms"},
            "inventory fields are incomplete or unknown")
    require(data["schema_version"] == "1.0.0", "unsupported inventory version")
    require(data["scope"] == "research_only", "inventory is not runtime configuration")
    require(data["network_access"] is False and data["writes_enabled"] is False,
            "research must not enable network access or writes")
    require(data["excluded_lineages"] == ["alaveteli"], "Alaveteli exclusion is required")
    require(data["fingerprint_kind"] == "reviewed-contract-projection-v1",
            "invalid fingerprint meaning")
    reviewed = _day(data["reviewed_on"])
    effective = as_of if as_of is not None else reviewed
    require(effective >= reviewed, "research review is in the future")
    platforms = data["platforms"]
    require(isinstance(platforms, list) and len(platforms) == len(PLATFORMS),
            "platform inventory must contain all six candidates")
    seen, ranks, overdue = set(), set(), []
    for item in platforms:
        require(isinstance(item, dict) and set(item) == PLATFORM_FIELDS,
                "platform fields are incomplete or unknown")
        identity = item["id"]
        require(isinstance(identity, str) and identity in PLATFORMS and identity not in seen,
                "platform inventory has an unknown or duplicate candidate")
        seen.add(identity)
        decision, hosts = PLATFORMS[identity]
        require(item["decision"] == decision, "decision violates the reviewed provider boundary")
        rank = item["rank"]
        require(type(rank) is int and 1 <= rank <= len(PLATFORMS) and rank not in ranks,
                "platform ranks must be unique integers from one to six")
        ranks.add(rank)
        require(all(_text(item[key]) for key in
                    ("name", "reason", "jurisdiction", "lineage", "next_experiment")),
                "platform decision context is incomplete")
        require(item["lineage"].lower() != "alaveteli", "Alaveteli is out of scope")
        require(item["confidence"] in {"documented", "provisional", "limited_search"},
                "invalid confidence level")
        item_reviewed = _day(item["reviewed_on"])
        require(item_reviewed <= reviewed, "candidate review is in the future")
        interval = item["review_interval_days"]
        require(type(interval) is int and 1 <= interval <= 90, "invalid review cadence")
        due = _day(item["next_review_on"])
        require(due == item_reviewed + timedelta(days=interval), "next review disagrees with cadence")
        if effective >= due:
            overdue.append(identity)
        sources = item["sources"]
        require(isinstance(sources, list) and bool(sources), "primary evidence is required")
        urls = set()
        for source in sources:
            require(isinstance(source, dict) and set(source) == {"url", "reviewed_on", "access"},
                    "source fields are incomplete or unknown")
            require(isinstance(source["url"], str), "invalid source URL")
            parsed = urlsplit(source["url"])
            require(parsed.scheme == "https" and parsed.hostname in hosts
                    and parsed.port in {None, 443} and parsed.username is None
                    and parsed.password is None and not parsed.query and not parsed.fragment,
                    "source URL must be allowlisted and credential-free HTTPS")
            require(source["url"] not in urls, "duplicate source URL")
            urls.add(source["url"])
            require(source["access"] in {"retrieved_page", "indexed_page"},
                    "invalid evidence access")
            require(_day(source["reviewed_on"]) == item_reviewed, "source review date mismatch")
        _contract(item["contract"])
        require(item["contract_sha256"] == fingerprint(item["contract"]),
                "reviewed contract fingerprint mismatch")
    require(seen == set(PLATFORMS), "platform inventory is incomplete")
    return {"status": "valid", "platform_count": len(platforms),
            "reviewed_on": reviewed.isoformat(), "as_of": effective.isoformat(),
            "overdue": overdue}


def compare_observations(data: dict, observations: dict) -> list[str]:
    """Report changed, unavailable or deprecated supplied contracts; never refresh."""
    validate_inventory(data)
    require(set(observations) == {"schema_version", "observed_on", "platforms"},
            "observation fields are incomplete or unknown")
    require(observations["schema_version"] == "1.0.0", "unsupported observation version")
    require(_day(observations["observed_on"]) >= _day(data["reviewed_on"]),
            "observations predate the reviewed baseline")
    observed = observations["platforms"]
    require(isinstance(observed, list) and len(observed) == len(PLATFORMS),
            "observations must cover every platform")
    baseline = {p["id"]: p for p in data["platforms"]}
    seen, drift = set(), []
    for item in observed:
        require(isinstance(item, dict) and set(item) == {"id", "status", "contract"},
                "observation entry has missing or unknown fields")
        identity = item["id"]
        require(isinstance(identity, str) and identity in baseline and identity not in seen,
                "observations have an unknown or duplicate platform")
        seen.add(identity)
        require(item["status"] in {"available", "unavailable", "deprecated"},
                "invalid observed availability")
        _contract(item["contract"])
        changed = fingerprint(item["contract"]) != baseline[identity]["contract_sha256"]
        if item["status"] != "available" or changed:
            drift.append(identity)
    return sorted(drift, key=lambda identity: baseline[identity]["rank"])


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--observations", type=Path)
    parser.add_argument("--as-of", type=date.fromisoformat)
    parser.add_argument("--fail-on-overdue", action="store_true")
    args = parser.parse_args(argv)
    if args.fail_on_overdue and args.as_of is None:
        parser.error("--fail-on-overdue requires an explicit --as-of date")
    try:
        data = load_json(args.inventory)
        report = validate_inventory(data, as_of=args.as_of)
        report["drift"] = (compare_observations(data, load_json(args.observations))
                           if args.observations else [])
    except (OSError, ValueError, TypeError, KeyError, OverflowError):
        print(json.dumps({"status": "invalid",
                          "error": "invalid or unreadable landscape evidence"}))
        return 1
    print(json.dumps(report, sort_keys=True))
    return int(bool(report["drift"] or (args.fail_on_overdue and report["overdue"])))


if __name__ == "__main__":
    raise SystemExit(main())
