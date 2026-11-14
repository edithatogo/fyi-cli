#!/usr/bin/env python3
"""Fail-closed RustSec suppression inventory for CI and optional pre-commit.

Only existing historical exceptions are eligible for temporary grandfathering.
This checker does NOT substitute for cargo audit, cargo deny, or GitHub alerts.
"""
from __future__ import annotations

import argparse
from datetime import date
import json
from pathlib import Path
import re
import sys
import tomllib

# Immutable baseline: policy edits alone cannot authorise newly ignored IDs.
# Reduce this set as exceptions are remediated; never extend it.
GRANDFATHERED = frozenset({
    "RUSTSEC-2026-0190", "RUSTSEC-2024-0344", "RUSTSEC-2024-0388",
    "RUSTSEC-2022-0093", "RUSTSEC-2026-0174", "RUSTSEC-2024-0384",
    "RUSTSEC-2024-0436", "RUSTSEC-2025-0009", "RUSTSEC-2025-0010",
    "RUSTSEC-2023-0071", "RUSTSEC-2025-0134", "RUSTSEC-2026-0098",
    "RUSTSEC-2026-0099", "RUSTSEC-2026-0104", "RUSTSEC-2024-0363",
    "RUSTSEC-2026-0387", "RUSTSEC-2026-0459", "RUSTSEC-2026-0370",
    "RUSTSEC-2026-0371", "RUSTSEC-2026-0372",
})
ADVISORY = re.compile(r"^RUSTSEC-\d{4}-\d{4}$")
CLI_IGNORE = re.compile(r"--ignore\s+(RUSTSEC-\d{4}-\d{4})\b")


def evaluate(root: Path, today: date) -> list[str]:
    """Validate exception policy and all workflow cargo-audit ignore flags."""
    errors: list[str] = []
    try:
        with (root / "deny.toml").open("rb") as handle:
            deny = tomllib.load(handle)
        denied = deny["advisories"]["ignore"]
        if not isinstance(denied, list) or any(not isinstance(x, str) for x in denied):
            raise ValueError("advisories.ignore must be a list of identifiers")
        policy = json.loads(
            (root / "security/advisory-exceptions.json").read_text(encoding="utf-8")
        )
        if not isinstance(policy, dict):
            raise ValueError("security exception policy must be a JSON object")
        if policy.get("schema_version") != 1 or policy.get("allow_new_exceptions") is not False:
            errors.append("security policy must have schema_version=1 and allow_new_exceptions=false")
        exceptions = policy["exceptions"]
        if not isinstance(exceptions, dict):
            raise ValueError("exceptions must be a JSON object")
    except (OSError, ValueError, KeyError, TypeError, UnicodeError) as exc:
        return [f"cannot parse required security exception contract: {exc}"]

    def check_ids(ids: list[str] | set[str], scope: str) -> None:
        for item in ids:
            if not ADVISORY.fullmatch(item):
                errors.append(f"{scope}: malformed advisory ID {item!r}")
            elif item not in GRANDFATHERED:
                errors.append(f"{scope}: new suppression forbidden: {item}")

    check_ids(denied, "deny.toml")
    check_ids(set(exceptions), "policy")
    if len(denied) != len(set(denied)):
        errors.append("deny.toml: duplicate ignored advisory")
    if len(exceptions) != len(set(exceptions)):
        errors.append("policy: duplicate advisory ID")
    for advisory in denied:
        if advisory not in exceptions:
            errors.append(f"deny.toml: undocumented suppression: {advisory}")

    for advisory, record in exceptions.items():
        if not isinstance(record, dict):
            errors.append(f"{advisory}: exception record must be an object")
            continue
        for field in ("owner", "tracking_issue", "reason", "expires"):
            if not isinstance(record.get(field), str) or not record[field].strip():
                errors.append(f"{advisory}: missing {field}")
        if not str(record.get("tracking_issue", "")).startswith(
            "https://github.com/edithatogo/fyi-cli/issues/"
        ):
            errors.append(f"{advisory}: tracking issue must be a repo issue")
        try:
            expires = date.fromisoformat(record["expires"])
        except (ValueError, TypeError, KeyError):
            errors.append(f"{advisory}: invalid expires date")
        else:
            if expires <= today:
                errors.append(f"{advisory}: exception expired on {expires}")

    workflows = root / ".github/workflows"
    if not workflows.is_dir():
        errors.append("missing .github/workflows directory")
        return errors
    for path in sorted([*workflows.glob("*.yml"), *workflows.glob("*.yaml")]):
        try:
            content = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as exc:
            errors.append(f"cannot read {path}: {exc}")
            continue
        # An unrecognised --ignore in the audit invocation is not permitted.
        # Other tool flags (pytest/ruff) are not treated as RustSec suppressions.
        if "cargo audit" not in content:
            continue
        workflow_ids = CLI_IGNORE.findall(content)
        check_ids(workflow_ids, path.name)
        for advisory in workflow_ids:
            if advisory not in denied:
                errors.append(f"{path.name}: audit ignore absent from deny.toml: {advisory}")
            if advisory not in exceptions:
                errors.append(f"{path.name}: undocumented audit ignore: {advisory}")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--today", type=date.fromisoformat, default=date.today())
    parser.add_argument("--require-python", default=None)
    args = parser.parse_args(argv)
    if args.require_python and ".".join(map(str, sys.version_info[:3])) != args.require_python:
        print(
            f"security guard requires Python {args.require_python}; found {sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
            file=sys.stderr,
        )
        return 1
    errors = evaluate(args.root, args.today)
    if errors:
        for error in errors:
            print(f"SECURITY POLICY FAIL: {error}", file=sys.stderr)
        return 1
    print("Security exception policy PASS: no new/expired RustSec suppressions")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
