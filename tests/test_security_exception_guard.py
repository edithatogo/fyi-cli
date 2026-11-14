"""Fail-closed regression tests for the grandfathered Rust advisory policy.

All cases operate on synthetic repository roots and never fetch advisories.
"""
from __future__ import annotations

import json
import subprocess
import sys
from datetime import date
from pathlib import Path

GUARD = Path(__file__).resolve().parents[1] / "scripts" / "verify_security_exceptions.py"

LEGACY = "RUSTSEC-2026-0459"
NEW = "RUSTSEC-2026-9999"


def fixture(tmp_path: Path, *, denied: list[str], cli: list[str], expires: str) -> Path:
    (tmp_path / ".github" / "workflows").mkdir(parents=True)
    (tmp_path / "security").mkdir()
    ids = ", ".join(f'"{value}"' for value in denied)
    (tmp_path / "deny.toml").write_text(f"[advisories]\nversion = 2\nignore = [{ids}]\n")
    opts = " ".join(f"--ignore {value}" for value in cli)
    (tmp_path / ".github/workflows/ci.yml").write_text(
        f"name: CI\njobs:\n  audit:\n    steps:\n      - run: cargo audit {opts}\n"
    )
    (tmp_path / "security/advisory-exceptions.json").write_text(
        json.dumps({
            "schema_version": 1,
            "allow_new_exceptions": False,
            "exceptions": {
                LEGACY: {
                    "owner": "repository-maintainer",
                    "tracking_issue": "https://github.com/edithatogo/fyi-cli/issues/325",
                    "reason": "Version-resolution blocker, pending safe upgrade",
                    "expires": expires,
                }
            },
        })
    )
    return tmp_path


def run_guard(root: Path, today: date) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(GUARD), "--root", str(root), "--today", today.isoformat()],
        capture_output=True, text=True, check=False,
    )


def test_grandfathered_exception_is_valid_until_expiration(tmp_path: Path) -> None:
    today = date(2026, 11, 14)
    root = fixture(tmp_path, denied=[LEGACY], cli=[LEGACY], expires="2026-12-14")
    result = run_guard(root, today)
    assert result.returncode == 0, result.stdout + result.stderr


def test_new_suppression_is_rejected_even_with_unchanged_policy(tmp_path: Path) -> None:
    root = fixture(tmp_path, denied=[LEGACY, NEW], cli=[LEGACY, NEW], expires="2026-12-14")
    result = run_guard(root, date(2026, 11, 14))
    assert result.returncode != 0
    assert NEW in result.stdout + result.stderr


def test_exception_expiry_fails_closed(tmp_path: Path) -> None:
    root = fixture(tmp_path, denied=[LEGACY], cli=[LEGACY], expires="2026-11-13")
    result = run_guard(root, date(2026, 11, 14))
    assert result.returncode != 0
    assert "expired" in (result.stdout + result.stderr).lower()


def test_cargo_audit_must_not_ignore_unlisted_exceptions(tmp_path: Path) -> None:
    root = fixture(tmp_path, denied=[LEGACY], cli=[LEGACY, NEW], expires="2026-12-14")
    result = run_guard(root, date(2026, 11, 14))
    assert result.returncode != 0
    assert NEW in result.stdout + result.stderr


def test_policy_must_not_expand_frozen_baseline(tmp_path: Path) -> None:
    root = fixture(tmp_path, denied=[LEGACY], cli=[LEGACY], expires="2026-12-14")
    policy = root / "security/advisory-exceptions.json"
    data = json.loads(policy.read_text())
    data["exceptions"][NEW] = dict(data["exceptions"][LEGACY])
    policy.write_text(json.dumps(data))
    result = run_guard(root, date(2026, 11, 14))
    assert result.returncode != 0
    assert NEW in result.stdout + result.stderr


def test_policy_disallows_permanent_exceptions_and_approval_flag(tmp_path: Path) -> None:
    root = fixture(tmp_path, denied=[LEGACY], cli=[LEGACY], expires="2026-12-14")
    policy = root / "security/advisory-exceptions.json"
    data = json.loads(policy.read_text())
    data["allow_new_exceptions"] = True
    policy.write_text(json.dumps(data))
    assert run_guard(root, date(2026, 11, 14)).returncode != 0
    data["allow_new_exceptions"] = False
    del data["exceptions"][LEGACY]["expires"]
    policy.write_text(json.dumps(data))
    assert run_guard(root, date(2026, 11, 14)).returncode != 0


def test_audit_ignore_must_be_in_deny_baseline(tmp_path: Path) -> None:
    root = fixture(tmp_path, denied=[], cli=[LEGACY], expires="2026-12-14")
    result = run_guard(root, date(2026, 11, 14))
    assert result.returncode != 0
    assert LEGACY in result.stdout + result.stderr

def test_protected_ci_and_precommit_both_run_guard() -> None:
    root = Path(__file__).resolve().parents[1]
    ci = (root / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    assert "Security Audit" in ci
    assert "scripts/verify_security_exceptions.py --require-python 3.15.1" in ci
    assert 'python-version: "3.15.1"' in ci

    hook = (root / ".pre-commit-config.yaml").read_text(encoding="utf-8")
    assert "security-exception-guard" in hook
    assert "scripts/verify_security_exceptions.py --require-python 3.15.1" in hook


def test_advisory_alias_and_equals_style_cannot_bypass_policy(tmp_path: Path) -> None:
    root = fixture(tmp_path, denied=[LEGACY], cli=[LEGACY], expires="2026-12-14")
    workflow = root / ".github/workflows/ci.yml"
    workflow.write_text(
        "name: CI\\njobs:\\n  audit:\\n    steps:\\n"
        "      - run: cargo audit --ignore=GHSA-aaaa-bbbb-cccc\\n"
    )
    result = run_guard(root, date(2026, 11, 14))
    assert result.returncode != 0
    assert "GHSA-aaaa-bbbb-cccc" in result.stdout + result.stderr


def test_duplicate_exception_metadata_keys_are_rejected(tmp_path: Path) -> None:
    root = fixture(tmp_path, denied=[LEGACY], cli=[LEGACY], expires="2026-12-14")
    policy = root / "security/advisory-exceptions.json"
    contents = policy.read_text(encoding="utf-8")
    contents = contents.replace(
        '"allow_new_exceptions": false,',
        '"allow_new_exceptions": false, "allow_new_exceptions": false,',
    )
    policy.write_text(contents, encoding="utf-8")
    result = run_guard(root, date(2026, 11, 14))
    assert result.returncode != 0
    assert "duplicate" in (result.stdout + result.stderr).lower()


def test_pre_push_runs_real_cargo_advisory_scan() -> None:
    root = Path(__file__).resolve().parents[1]
    hook = (root / ".pre-commit-config.yaml").read_text(encoding="utf-8")
    assert "cargo-deny-advisories" in hook
    assert "cargo deny check advisories" in hook
    assert "pre-push" in hook


RETIRED_NONMATCHING = frozenset({
    "RUSTSEC-2022-0093", "RUSTSEC-2024-0344", "RUSTSEC-2024-0363",
    "RUSTSEC-2024-0384", "RUSTSEC-2024-0388", "RUSTSEC-2025-0009",
    "RUSTSEC-2025-0010", "RUSTSEC-2025-0134", "RUSTSEC-2026-0098",
    "RUSTSEC-2026-0099", "RUSTSEC-2026-0104", "RUSTSEC-2026-0174",
    "RUSTSEC-2026-0190",
})


def test_confirmed_nonmatching_advisory_ignores_are_retired() -> None:
    import tomllib

    root = Path(__file__).resolve().parents[1]
    with (root / "deny.toml").open("rb") as stream:
        ignored = set(tomllib.load(stream)["advisories"]["ignore"])
    policy = json.loads(
        (root / "security/advisory-exceptions.json").read_text(encoding="utf-8")
    )
    assert RETIRED_NONMATCHING.isdisjoint(ignored)
    assert RETIRED_NONMATCHING.isdisjoint(policy["exceptions"])


def test_retired_ids_cannot_be_added_back_to_grandfathered_list() -> None:
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "security_exception_guard", Path(__file__).resolve().parents[1]
        / "scripts/verify_security_exceptions.py"
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert RETIRED_NONMATCHING.isdisjoint(module.GRANDFATHERED)
