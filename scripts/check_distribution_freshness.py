#!/usr/bin/env python3
"""Check public evidence URLs in the distribution ledger without credentials."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

TIMEOUT_SECONDS = 10
MAX_TARGETS = 30
EVIDENCE_STATUSES = {"live", "submitted", "blocked-external"}


def check_url(url: str) -> tuple[str, int | None]:
    request = Request(url, headers={"User-Agent": "fyi-cli-distribution-monitor/0.1"}, method="GET")
    try:
        with urlopen(request, timeout=TIMEOUT_SECONDS) as response:  # nosec B310: ledger-controlled HTTPS evidence URLs
            response.read(1)
            return "reachable", response.status
    except HTTPError as error:
        if 300 <= error.code < 500:
            return "reachable", error.code
        return "unreachable", error.code
    except (URLError, TimeoutError, OSError):
        return "unreachable", None


def build_report(ledger: dict) -> dict:
    targets = ledger.get("targets", [])
    if not isinstance(targets, list) or len(targets) > MAX_TARGETS:
        raise ValueError(f"targets must contain at most {MAX_TARGETS} entries")
    results = []
    for target in targets:
        status = target.get("status")
        evidence = target.get("evidence")
        result = {"id": target.get("id"), "status": status, "evidence": evidence, "check": "skipped"}
        if status in EVIDENCE_STATUSES:
            if not isinstance(evidence, str) or not evidence.startswith("https://"):
                result["check"] = "invalid-evidence"
            else:
                result["check"], result["http_status"] = check_url(evidence)
        results.append(result)
    return {"schema_version": 1, "source_release": ledger.get("source_release"), "results": results}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--path", type=Path, default=Path("packaging/registry-submissions.json"))
    parser.add_argument("--strict", action="store_true", help="fail when required evidence is unavailable")
    parser.add_argument("--output", type=Path, default=None)
    args = parser.parse_args()
    report = build_report(json.loads(args.path.read_text(encoding="utf-8")))
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    failures = [item for item in report["results"] if item["check"] in {"unreachable", "invalid-evidence"}]
    return 1 if args.strict and failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
