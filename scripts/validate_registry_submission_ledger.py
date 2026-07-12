#!/usr/bin/env python3
"""Validate the machine-readable registry submission ledger."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

STATUSES = {"planned", "assets-ready", "submitted", "live", "blocked-external", "not-applicable"}


def load_ledger(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def validate(ledger: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if ledger.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    targets = ledger.get("targets")
    if not isinstance(targets, list) or not targets:
        return ["targets must be a non-empty list"]

    seen: set[str] = set()
    for index, target in enumerate(targets):
        prefix = f"targets[{index}]"
        if not isinstance(target, dict):
            errors.append(f"{prefix} must be an object")
            continue
        target_id = target.get("id")
        status = target.get("status")
        if not isinstance(target_id, str) or not target_id:
            errors.append(f"{prefix}.id must be non-empty")
        elif target_id in seen:
            errors.append(f"duplicate target id: {target_id}")
        else:
            seen.add(target_id)
        if target.get("class") not in {"ai-plugin", "mcp-registry", "mcp-catalog", "community-catalog", "package", "container"}:
            errors.append(f"{prefix}.class is unsupported")
        if status not in STATUSES:
            errors.append(f"{prefix}.status is unsupported: {status}")
        next_action = target.get("next_action")
        if not isinstance(next_action, str) or not next_action.strip():
            errors.append(f"{prefix}.next_action must be non-empty")
        evidence = target.get("evidence")
        if status == "live" or status == "submitted" or status == "blocked-external":
            if not isinstance(evidence, str) or not evidence:
                errors.append(f"{prefix}.evidence is required for status {status}")
        if evidence:
            parsed = urlparse(evidence)
            if parsed.scheme not in {"http", "https"} or not parsed.netloc:
                errors.append(f"{prefix}.evidence must be an HTTP(S) URL")
        if status == "planned" and not target.get("submission_url") and target.get("class") == "ai-plugin":
            errors.append(f"{prefix}.submission_url is required for planned AI plugins")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--path", type=Path, default=Path("packaging/registry-submissions.json"))
    args = parser.parse_args()
    errors = validate(load_ledger(args.path))
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print(f"Registry ledger valid: {len(load_ledger(args.path)['targets'])} targets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
