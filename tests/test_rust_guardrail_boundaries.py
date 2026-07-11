"""Regression sensor for every production Rust outbound send boundary."""

from scripts.verify_rust_guardrail_boundaries import validate_boundaries


def test_all_rust_send_boundaries_use_guardrails() -> None:
    assert validate_boundaries() == []
