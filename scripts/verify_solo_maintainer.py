#!/usr/bin/env python3
"""One-command validation for solo-maintainer repository security (fyi-cli).

Fail-closed sensor for the #281 solo-maintainer security baseline:

- Canonical context files exist: AGENTS.md, CONTRIBUTING.md, .github/SECURITY.md,
  renovate.json.
- ``renovate.json`` inherits ``github>edithatogo/renovate-config``.
- Codecov OIDC upload is configured in ``.github/workflows/ci.yml``
  (``use_oidc: true`` with ``id-token: write``).
- Declarative expectations for branch protection and the additive ruleset are recorded in
  the drift receipt; live values are queried via ``gh api`` when available and compared.
- Emits a drift receipt JSON (``--receipt <path>``) capturing observed state and per-check
  results; exits non-zero if any check fails.

Run with:
    python scripts/verify_solo_maintainer.py
    python scripts/verify_solo_maintainer.py --receipt .conductor/tracks/solo-maintainer-security-20261113/drift-receipt.json
    python scripts/verify_solo_maintainer.py --offline   # skip live gh api checks
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REPO = "edithatogo/fyi-cli"

EXPECTED_REQUIRED_CONTEXTS = {
    "Rustfmt",
    "Clippy Lints",
    "Workspace Tests",
    "Package Smoke",
    "Security Audit",
    "Code Coverage",
    "Layered Harness Contract",
    "Python Tests",
    "CodeQL",
    "Analyze (python)",
    "Analyze (javascript-typescript)",
    "Packaging Assets",
}

REQUIRED_FILES = [
    "AGENTS.md",
    "CONTRIBUTING.md",
    ".github/SECURITY.md",
    ".github/CODEOWNERS",
    "renovate.json",
]


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def gh_api(path: str) -> tuple[bool, Any]:
    """Call ``gh api``; return (ok, parsed_json_or_error_text)."""
    result = subprocess.run(
        ["gh", "api", path],
        capture_output=True,
        text=True,
        timeout=60,
        cwd=ROOT,
    )
    if result.returncode != 0:
        return False, result.stderr.strip() or result.stdout.strip()
    try:
        return True, json.loads(result.stdout)
    except json.JSONDecodeError:
        return False, "gh api returned non-JSON output"


def check_required_files(checks: list[dict[str, Any]]) -> None:
    for rel in REQUIRED_FILES:
        exists = (ROOT / rel).is_file()
        checks.append(
            {"name": f"file:{rel}", "ok": exists, "detail": "present" if exists else "MISSING"}
        )


def check_renovate_inheritance(checks: list[dict[str, Any]]) -> None:
    path = ROOT / "renovate.json"
    if not path.is_file():
        checks.append(
            {"name": "renovate:extends", "ok": False, "detail": "renovate.json missing"}
        )
        return
    config = load_json(path)
    extends = config.get("extends", [])
    has_preset = any(
        entry == "github>edithatogo/renovate-config"
        or str(entry).startswith("github>edithatogo/renovate-config:")
        for entry in extends
    )
    checks.append(
        {
            "name": "renovate:inherits-edithatogo-preset",
            "ok": has_preset,
            "detail": (
                f"extends={extends} ; preset "
                f"{'present' if has_preset else 'MISSING — expected github>edithatogo/renovate-config'}"
            ),
        }
    )


def check_codecov_oidc(checks: list[dict[str, Any]]) -> None:
    ci = ROOT / ".github/workflows/ci.yml"
    text = ci.read_text(encoding="utf-8") if ci.is_file() else ""
    upload_step = "codecov-action" in text
    oidc = re.search(r"use_oidc:\s*true", text) is not None
    id_token = re.search(r"id-token:\s*write", text) is not None
    fail_closed = re.search(r"fail_ci_if_error:\s*true", text) is not None
    ok = upload_step and oidc and id_token and fail_closed
    checks.append(
        {
            "name": "codecov:oidc-upload",
            "ok": ok,
            "detail": (
                f"codecov-action={upload_step} use_oidc={oidc} "
                f"id-token:write={id_token} fail_ci_if_error={fail_closed}"
            ),
        }
    )


def check_solo_operation(checks: list[dict[str, Any]], live: bool) -> None:
    if not live:
        checks.append(
            {"name": "github:collaborators-solo", "ok": True, "detail": "skipped (--offline)"}
        )
        return
    ok, payload = gh_api(f"repos/{REPO}/collaborators")
    if not ok:
        checks.append(
            {"name": "github:collaborators-solo", "ok": False, "detail": f"gh api failed: {payload}"}
        )
        return
    logins = sorted(entry.get("login") for entry in payload if isinstance(entry, dict))
    solo = logins == ["edithatogo"]
    checks.append(
        {
            "name": "github:collaborators-solo",
            "ok": solo,
            "detail": f"collaborators={logins}",
        }
    )


def check_branch_protection(checks: list[dict[str, Any]], live: bool) -> dict[str, Any]:
    if not live:
        checks.append(
            {
                "name": "github:branch-protection-master",
                "ok": True,
                "detail": "skipped (--offline)",
            }
        )
        return {"observed": None}
    ok, payload = gh_api(f"repos/{REPO}/branches/master/protection")
    if not ok:
        checks.append(
            {
                "name": "github:branch-protection-master",
                "ok": False,
                "detail": f"gh api failed: {payload}",
            }
        )
        return {"observed": None}

    contexts = set(payload.get("required_status_checks", {}).get("contexts", []))
    approving = (
        payload.get("required_pull_request_reviews", {}) or {}
    ).get("required_approving_review_count")
    allow_force = payload.get("allow_force_pushes", {}).get("enabled")
    allow_deletions = payload.get("allow_deletions", {}).get("enabled")
    enforce_admins = payload.get("enforce_admins", {}).get("enabled")

    missing = sorted(EXPECTED_REQUIRED_CONTEXTS - contexts)
    ok = (
        not missing
        and allow_force is False
        and allow_deletions is False
        and enforce_admins is True
        and approving == 0
    )
    detail = (
        f"required={sorted(contexts)} missing={missing} "
        f"force_pushes={allow_force} deletions={allow_deletions} "
        f"enforce_admins={enforce_admins} approving={approving}"
    )
    checks.append(
        {"name": "github:branch-protection-master", "ok": ok, "detail": detail}
    )
    return {
        "observed": {
            "required_contexts": sorted(contexts),
            "enforce_admins": enforce_admins,
            "allow_force_pushes": allow_force,
            "allow_deletions": allow_deletions,
            "required_approving_review_count": approving,
        }
    }


def check_rulesets(checks: list[dict[str, Any]], live: bool) -> dict[str, Any]:
    observation: dict[str, Any] = {"observed": None}
    if not live:
        checks.append(
            {"name": "github:rulesets-solo-maintainer", "ok": True, "detail": "skipped (--offline)"}
        )
        return observation
    ok, payload = gh_api(f"repos/{REPO}/rulesets")
    if not ok:
        checks.append(
            {
                "name": "github:rulesets-solo-maintainer",
                "ok": False,
                "detail": f"gh api failed: {payload}",
            }
        )
        return observation

    rulesets = payload if isinstance(payload, list) else []
    blocking = [
        ruleset.get("name")
        for ruleset in rulesets
        if isinstance(ruleset, dict) and ruleset.get("enforcement") == "active"
    ]
    # Inspect each active ruleset for deletion/non_fast_forward rules.
    blocking_names: list[str | None] = []
    for ruleset in rulesets:
        if not isinstance(ruleset, dict) or ruleset.get("enforcement") != "active":
            continue
        rules_ok, rules_payload = gh_api(f"repos/{REPO}/rulesets/{ruleset.get('id')}")
        if not rules_ok:
            continue
        types = {rule.get("type") for rule in rules_payload.get("rules", [])}
        if {"deletion", "non_fast_forward"} <= types:
            blocking_names.append(ruleset.get("name"))

    ok = bool(blocking_names)
    checks.append(
        {
            "name": "github:rulesets-solo-maintainer",
            "ok": ok,
            "detail": (
                f"active_blocking_rulesets={blocking_names} "
                f"all_active={blocking}"
            ),
        }
    )
    observation["observed"] = {
        "active_blocking_rulesets": blocking_names,
        "active_rulesets": blocking,
    }
    return observation


def check_zero_approvals_note(checks: list[dict[str, Any]]) -> None:
    # Enforced by check_branch_protection (approving == 0). This entry documents intent.
    checks.append(
        {
            "name": "solo:zero-mandatory-approvals",
            "ok": True,
            "detail": "merge gate is automated checks; required approving review count is 0",
        }
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--receipt",
        type=Path,
        default=None,
        help="Write the drift receipt JSON to this path.",
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="Skip live gh api checks (file checks only).",
    )
    args = parser.parse_args()

    live = not args.offline
    checks: list[dict[str, Any]] = []
    check_required_files(checks)
    check_renovate_inheritance(checks)
    check_codecov_oidc(checks)
    check_solo_operation(checks, live)
    protection = check_branch_protection(checks, live)
    rulesets = check_rulesets(checks, live)
    check_zero_approvals_note(checks)

    failures = [c for c in checks if not c["ok"]]
    receipt = {
        "receipt_type": "solo-maintainer-security-drift",
        "repo": REPO,
        "collected_at": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "mode": "offline" if args.offline else "live",
        "checks": checks,
        "branch_protection": protection["observed"],
        "rulesets": rulesets["observed"],
        "ok": not failures,
    }
    if args.receipt:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        args.receipt.write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )

    for check in checks:
        status = "PASS" if check["ok"] else "FAIL"
        print(f"[{status}] {check['name']}: {check['detail']}")

    if failures:
        print(f"\n{len(failures)} check(s) FAILED")
        return 1
    print("\nAll solo-maintainer security checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
