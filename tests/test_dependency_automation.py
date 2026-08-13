"""Contracts for the repository's single dependency-update mechanism."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).parents[1]


def test_renovate_is_the_only_scheduled_dependency_updater() -> None:
    assert not (ROOT / ".github" / "dependabot.yml").exists()
    config = json.loads((ROOT / "renovate.json").read_text(encoding="utf-8"))
    presets = set(config["extends"])
    assert ":enableVulnerabilityAlerts" in presets
    assert ":dependencyDashboard" in presets
    assert ":maintainLockFilesWeekly" in presets
    assert config["vulnerabilityAlerts"]["labels"] == ["security"]
    assert "reviewers" not in config
    assert "assignees" not in config
