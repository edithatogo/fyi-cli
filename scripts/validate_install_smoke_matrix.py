#!/usr/bin/env python3
"""Validate package install-smoke contracts without installing packages."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_KEYS = {"id", "asset", "status", "smoke", "requires_release_asset", "remote_writes"}
ALLOWED_STATUSES = {"release-gated", "verified"}


def validate(repo_root: Path) -> list[str]:
    path = repo_root / "packaging" / "install-smoke-matrix.json"
    payload = json.loads(path.read_text(encoding="utf-8"))
    errors: list[str] = []
    if payload.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    targets = payload.get("targets")
    if not isinstance(targets, list) or not targets:
        return ["targets must be a non-empty list"]
    seen: set[str] = set()
    for index, target in enumerate(targets):
        prefix = f"targets[{index}]"
        if not isinstance(target, dict):
            errors.append(f"{prefix} must be an object")
            continue
        errors.extend(f"{prefix} missing {key}" for key in sorted(REQUIRED_KEYS - target.keys()))
        target_id = target.get("id")
        if not isinstance(target_id, str) or not target_id or target_id in seen:
            errors.append(f"{prefix}.id must be unique and non-empty")
        seen.add(target_id)
        if target.get("status") not in ALLOWED_STATUSES:
            errors.append(f"{prefix}.status is unsupported")
        if not isinstance(target.get("smoke"), str) or "--help" not in target["smoke"]:
            errors.append(f"{prefix}.smoke must be a help-only command")
        if target.get("remote_writes") is not False:
            errors.append(f"{prefix}.remote_writes must be false")
        asset = target.get("asset")
        if not isinstance(asset, str) or not (repo_root / asset).is_file():
            errors.append(f"{prefix}.asset is missing: {asset}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    errors = validate(args.repo_root)
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print("Install-smoke matrix valid: 12 release-gated targets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
