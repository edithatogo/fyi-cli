"""Regression tests for bounded Atheris target functions."""

import json

import pytest

from fuzzing.targets import MAX_INPUT_BYTES, TARGETS, _bounded_json


@pytest.mark.parametrize("target", sorted(TARGETS))
def test_targets_accept_arbitrary_and_oversized_bytes(target: str) -> None:
    TARGETS[target](b"\xff\x00not-json")
    TARGETS[target](b"x" * (MAX_INPUT_BYTES + 1))


def test_bounded_json_rejects_excessive_depth() -> None:
    value: object = "leaf"
    for _ in range(34):
        value = [value]
    with pytest.raises(ValueError, match="exceeds fuzz harness bounds"):
        _bounded_json(json.dumps(value).encode())


def test_all_high_risk_targets_are_registered() -> None:
    assert set(TARGETS) == {"receipt", "cdx", "wayback", "redaction"}
