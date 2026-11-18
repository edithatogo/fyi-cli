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


def test_docs_site_lockfile_has_no_vulnerable_picomatch() -> None:
    lockfile = json.loads((ROOT / "docs-site" / "package-lock.json").read_text(encoding="utf-8"))
    packages = lockfile["packages"]
    picomatch_entries = [
        (pkg_path, meta)
        for pkg_path, meta in packages.items()
        if pkg_path == "node_modules/picomatch"
        or pkg_path.endswith("/node_modules/picomatch")
        or meta.get("name") == "picomatch"
    ]
    assert picomatch_entries, "expected picomatch entries in docs-site/package-lock.json"

    vulnerable = []
    for pkg_path, meta in picomatch_entries:
        version_str = str(meta["version"]).split("-", 1)[0].split("+", 1)[0]
        version_tuple = tuple(int(part) for part in version_str.split("."))
        if version_tuple <= (4, 0, 7):
            vulnerable.append((pkg_path, meta["version"]))

    assert vulnerable == [], f"vulnerable picomatch entries found: {vulnerable}"
