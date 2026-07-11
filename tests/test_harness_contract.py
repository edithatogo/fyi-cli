"""Regression tests for the repository-wide layered test harness."""

from scripts.verify_test_harness import LAYERS, validate_inventory


def test_every_required_test_layer_has_checked_in_evidence() -> None:
    assert not validate_inventory()
    assert {layer.name for layer in LAYERS} == {
        "unit",
        "integration",
        "end-to-end",
        "smoke/system",
        "mutation",
        "property-based",
        "edge",
        "performance",
        "security",
        "compatibility",
        "usability",
        "regression",
        "sanity",
        "remote-mcp-contract",
    }
