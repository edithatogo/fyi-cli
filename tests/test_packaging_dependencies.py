"""Packaging dependency contracts for installed CLI entry points."""

from __future__ import annotations

from pathlib import Path

import tomllib


def test_runtime_dependencies_include_requests_for_alaveteli_client() -> None:
    """The installed CLI imports the established requests-based client stack."""
    pyproject = Path(__file__).resolve().parents[1] / "pyproject.toml"
    metadata = tomllib.loads(pyproject.read_text(encoding="utf-8"))
    dependencies = metadata["project"]["dependencies"]
    assert any(dependency.startswith("requests") for dependency in dependencies)
