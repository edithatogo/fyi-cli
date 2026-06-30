"""Tests for the API contract inventory report."""

from __future__ import annotations

from pathlib import Path

from scripts.api_contract_inventory import (
    REQUIRED_SURFACES,
    build_contract_inventory,
    render_markdown,
)


def test_contract_inventory_covers_required_api_surfaces() -> None:
    """The inventory must cover every API-adjacent surface named by the track."""
    repo_root = Path(__file__).resolve().parents[1]

    inventory = build_contract_inventory(repo_root)
    surface_ids = {surface["id"] for surface in inventory["surfaces"]}

    assert surface_ids >= REQUIRED_SURFACES


def test_contract_inventory_maps_tests_and_high_risk_gaps() -> None:
    """The inventory should be actionable, not only a file list."""
    repo_root = Path(__file__).resolve().parents[1]

    inventory = build_contract_inventory(repo_root)
    surfaces = {surface["id"]: surface for surface in inventory["surfaces"]}

    assert "crates/fyi-core/src/api.rs" in surfaces["rust_api_payloads"]["files"]
    assert any(
        test_path.endswith("crates/fyi-core/src/sync.rs::tests")
        for test_path in surfaces["rust_sync_client"]["tests"]
    )
    assert any(gap["risk"] == "high" for gap in inventory["gaps"])


def test_contract_inventory_markdown_is_release_readable() -> None:
    """The generated report should include matrix and gap sections."""
    repo_root = Path(__file__).resolve().parents[1]

    report = render_markdown(build_contract_inventory(repo_root))

    assert "# API Contract Inventory" in report
    assert "| Surface | Contract | Coverage | Risk |" in report
    assert "## High-Risk Untested Paths" in report
    assert "rust_sync_client" in report
