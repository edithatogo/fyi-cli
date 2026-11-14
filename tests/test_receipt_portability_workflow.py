"""Keep the receipt portability gate bounded, offline and fail-closed."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/receipt-portability.yml"


def workflow() -> str:
    return WORKFLOW.read_text(encoding="utf-8")


def test_portability_covers_three_explicit_operating_systems() -> None:
    text = workflow()
    matrix = re.search(r"os: \[([^\]]+)\]", text)
    assert matrix is not None
    assert {value.strip() for value in matrix[1].split(",")} == {
        "ubuntu-24.04", "windows-2025", "macos-15"
    }
    assert "fail-fast: false" in text
    assert "timeout-minutes: 45" in text


def test_receipt_contracts_and_consumer_are_executed() -> None:
    text = workflow()
    assert "cargo test --locked --package fyi-core" in text
    assert "cargo test --locked --package fyi-cli" in text
    for target in (
        "acquisition_receipts", "acquisition_receipt_numeric_conformance",
        "acquisition_receipt_midpoint_conformance", "receipt_wire",
        "receipt_persistence_portability", "receipt_numeric_tests", "receipt_wire_tests",
    ):
        assert re.search(rf"--test {target}(?:\s|$)", text), target
    assert "continue-on-error" not in text
    assert "|| true" not in text


def test_workflow_has_read_only_permissions_and_immutable_actions() -> None:
    text = workflow()
    assert "permissions:\n  contents: read" in text
    assert "persist-credentials: false" in text
    assert not re.search(r"^\s+\w[\w-]*:\s+write\s*$", text, re.MULTILINE)
    uses = re.findall(r"^\s*(?:-\s+)?uses:\s*(\S+)", text, re.MULTILINE)
    assert uses
    assert all(re.fullmatch(r"[\w-]+/[\w-]+@[0-9a-f]{40}", action) for action in uses)


def test_artifacts_include_provenance_and_failures() -> None:
    text = workflow()
    assert "git rev-parse HEAD" in text
    assert "rustc --version --verbose" in text
    assert "cargo --version" in text
    assert "if: always()" in text
    assert "retention-days: 14" in text
    assert "if-no-files-found: error" in text
    assert "set -o pipefail" in text


def test_source_changes_and_workflow_changes_trigger_the_gate() -> None:
    text = workflow()
    assert "pull_request:" in text and "push:" in text
    assert "branches: [master]" in text
    for path in (
        "crates/fyi-core/**", "crates/fyi-cli/**", "conformance/acquisition/**",
        "Cargo.lock", "rust-toolchain.toml", ".github/workflows/receipt-portability.yml",
    ):
        assert f'"{path}"' in text
    assert "workflow_dispatch:" in text
    assert "schedule:" not in text


def test_runtime_acquisition_is_not_enabled_by_portability_checks() -> None:
    text = workflow()
    assert "FYI_RUN_LIVE" not in text
    assert "internet-archive-replay" not in text
    assert "fetch-request-page" not in text
    assert "CARGO_BUILD_JOBS:" in text
    assert "CARGO_INCREMENTAL:" in text
