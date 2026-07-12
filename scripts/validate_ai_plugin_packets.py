#!/usr/bin/env python3
"""Validate the repo-side AI plugin submission packets."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from urllib.parse import urlparse

PACKETS = {
    "codex": ("openai-codex-plugins", "codex-plugins"),
    "anthropic": ("anthropic-claude-connectors", "anthropic-connectors"),
}
REQUIRED_KEYS = {
    "schema_version",
    "target",
    "product",
    "version",
    "repository",
    "release_source",
    "license",
    "submission_route",
    "description",
    "runtime",
    "capabilities",
    "privacy",
    "review_checklist",
    "rollback",
}


def validate_packet(packet: dict, target_name: str, relative: Path) -> list[str]:
    errors: list[str] = []
    missing = REQUIRED_KEYS - packet.keys()
    errors.extend(f"{relative}: missing {key}" for key in sorted(missing))
    if packet.get("schema_version") != 1:
        errors.append(f"{relative}: schema_version must be 1")
    if packet.get("target") != target_name:
        errors.append(f"{relative}: target does not match {target_name}")
    if packet.get("product") != "fyi-mcp":
        errors.append(f"{relative}: product must be fyi-mcp")
    for key in ("repository", "release_source", "submission_route"):
        parsed = urlparse(packet.get(key, ""))
        if parsed.scheme not in {"http", "https"} or not parsed.netloc:
            errors.append(f"{relative}: {key} must be an HTTP(S) URL")
    capabilities = packet.get("capabilities", {})
    for key in ("remote_request_submission", "remote_authority_writes"):
        if capabilities.get(key) is not False:
            errors.append(f"{relative}: {key} must remain false")
    checklist = packet.get("review_checklist")
    if not isinstance(checklist, list) or len(checklist) < 4:
        errors.append(f"{relative}: review_checklist must contain at least 4 items")
    if not isinstance(packet.get("rollback"), str) or not packet["rollback"].strip():
        errors.append(f"{relative}: rollback must be non-empty")
    return errors


def validate(repo_root: Path) -> list[str]:
    errors: list[str] = []
    ledger = json.loads(
        (repo_root / "packaging" / "registry-submissions.json").read_text(
            encoding="utf-8"
        )
    )
    targets = {target["id"]: target for target in ledger["targets"]}
    for name, (target_name, ledger_id) in PACKETS.items():
        relative = Path("packaging") / "ai-plugins" / name / "submission.json"
        path = repo_root / relative
        if not path.is_file():
            errors.append(f"{relative} is missing")
            continue
        packet = json.loads(path.read_text(encoding="utf-8"))
        errors.extend(validate_packet(packet, target_name, relative))
        target = targets.get(ledger_id)
        if target is None:
            errors.append(f"ledger target missing: {ledger_id}")
        elif target.get("status") != "planned":
            errors.append(f"ledger target {ledger_id} must remain planned until external evidence")
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
    print("AI plugin submission packets valid: 2")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
