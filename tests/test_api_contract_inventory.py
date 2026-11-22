"""Tests for the API contract inventory report."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.api_contract_inventory import (
    REQUIRED_SURFACES,
    UPSTREAM_CASES,
    build_contract_inventory,
    compare_upstream_case,
    compare_upstream_fixture,
    render_markdown,
)


def test_contract_inventory_covers_required_api_surfaces() -> None:
    """The inventory must cover every API-adjacent surface named by the track."""
    repo_root = Path(__file__).resolve().parents[1]

    inventory = build_contract_inventory(repo_root)
    surface_ids = {surface["id"] for surface in inventory["surfaces"]}

    assert surface_ids >= REQUIRED_SURFACES


def test_contract_inventory_maps_tests_and_resolved_risk() -> None:
    """The inventory should be actionable, not only a file list."""
    repo_root = Path(__file__).resolve().parents[1]

    inventory = build_contract_inventory(repo_root)
    surfaces = {surface["id"]: surface for surface in inventory["surfaces"]}

    assert "crates/fyi-core/src/api.rs" in surfaces["rust_api_payloads"]["files"]
    assert any(
        test_path.endswith("crates/fyi-core/src/sync.rs::tests")
        for test_path in surfaces["rust_sync_client"]["tests"]
    )
    assert surfaces["rust_sync_client"]["risk"] == "high"
    assert any(gap["risk"] == "closed" for gap in inventory["gaps"])


def test_contract_inventory_markdown_is_release_readable() -> None:
    """The generated report should include matrix and gap sections."""
    repo_root = Path(__file__).resolve().parents[1]

    report = render_markdown(build_contract_inventory(repo_root))

    assert "# API Contract Inventory" in report
    assert "| Surface | Contract | Coverage | Risk |" in report
    assert "## Contract Gaps And Residual Risk" in report
    assert "rust_sync_client" in report


def test_reference_comparison_checks_wire_parameters_and_response() -> None:
    """A matching path alone must not establish an upstream contract match."""
    reference = {
        "method": "POST",
        "path": "/api/v2/request/{request_id}.json",
        "parameters": ["k", "correspondence_json"],
        "json_string_parameters": ["correspondence_json"],
        "response_fields": ["url"],
    }
    exchange = {
        "method": "POST",
        "path": "/api/v2/request/7001.json",
        "request_parts": {"k": "synthetic", "correspondence_json": '{"body":"sample"}'},
        "json": {"url": "https://example.invalid/request/sample"},
    }
    assert compare_upstream_case(reference, exchange, request_id=7001) == []

    altered = {
        "method": "PUT",
        "path": "/api/v2/request/7001/state.json",
        "request_json": {"correspondence_json": {"body": "sample"}},
        "json": {"updated": True},
    }
    findings = compare_upstream_case(reference, altered, request_id=7001)
    assert {
        "method",
        "path",
        "missing parameter k",
        "parameter correspondence_json must be JSON text",
        "missing response field url",
    } <= set(findings)


def test_reference_comparison_fails_closed_for_missing_cases() -> None:
    """A missing fixture cannot silently disappear from a coverage report."""
    reference = [{"id": "create", "fixture_case": "create", "exchange_index": 0}]
    comparison = compare_upstream_fixture(reference, {"cases": []})
    assert comparison == [
        {"id": "create", "fixture_case": "create", "differences": ["fixture case missing"]},
    ]
    with pytest.raises(ValueError, match="reference cases must be present"):
        compare_upstream_fixture([], {"cases": []})


def test_upstream_contract_retains_every_selected_operation() -> None:
    assert {case["id"] for case in UPSTREAM_CASES} == {
        "authority_read_request",
        "authority_create_external_request",
        "authority_correspondence",
        "authority_response_attachment",
        "authority_update_state",
    }


def test_stock_reference_fixture_matches_pinned_controller_without_changing_legacy() -> None:
    root = Path(__file__).resolve().parents[1]
    fixture_dir = root / "tests/fixtures/remote_mcp"
    stock = json.loads((fixture_dir / "upstream-stock-v1.json").read_text(encoding="utf-8"))
    existing = json.loads((fixture_dir / "upstream-v2.json").read_text(encoding="utf-8"))
    assert stock["reference_revision"] == "3966415f0b2a9f445fa3bb25c06eaeb75ded6609"
    assert all(not case["differences"] for case in compare_upstream_fixture(UPSTREAM_CASES, stock))
    assert all(case["differences"] for case in compare_upstream_fixture(UPSTREAM_CASES, existing))
    assert stock["provenance"].startswith("synthetic")


def test_upstream_reference_comparison_rejects_wrong_success_status() -> None:
    reference = next(case for case in UPSTREAM_CASES if case["id"] == "authority_read_request")
    exchange = {
        "method": "GET",
        "path": "/api/v2/request/7001.json?k=placeholder",
        "status": 201,
        "json": {
            "id": 7001,
            "title": "Synthetic",
            "url": "https://example.invalid",
            "request_text": "Synthetic",
        },
    }
    assert "status" in compare_upstream_case(reference, exchange, request_id=7001)


def test_reference_comparison_rejects_empty_key_without_disclosing_values() -> None:
    reference = {
        "method": "POST",
        "path": "/submit",
        "parameters": ["k"],
        "response_fields": ["url"],
    }
    exchange = {"method": "POST", "path": "/submit?k=", "json": {"url": "synthetic"}}
    assert compare_upstream_case(reference, exchange, request_id=0) == [
        "parameter k is empty or invalid",
    ]
    exchange["path"] = "/wrong?k=synthetic-private-value"
    assert compare_upstream_case(reference, exchange, request_id=0) == ["path"]


def test_generated_inventory_matches_the_checked_in_contract() -> None:
    """The rendered inventory and its executable source must not drift apart."""
    repo_root = Path(__file__).resolve().parents[1]
    expected = render_markdown(build_contract_inventory(repo_root))
    assert (repo_root / "docs/api-contract-inventory.md").read_text(encoding="utf-8") == expected
