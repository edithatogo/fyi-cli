"""Verify development CI pins and asserts the selected repo-local Python runtime."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = (
    "ci.yml",
    "python-security-fuzzing.yml",
    "python315-compatibility.yml",
    "test-profile-isolation.yml",
    "profiling.yml",
    "mutation.yml",
    "frontier-conductor.yml",
    "external-registry-monitor.yml",
    "distribution-freshness.yml",
    "ci-learning-candidates.yml",
    "container-verification.yml",
    "project-sync.yml",
    "release.yml",
)


def test_repo_python_pin_is_exact() -> None:
    assert (ROOT / ".python-version").read_text(encoding="utf-8").strip() == "3.15.1"


def test_all_active_python_workflows_verify_repo_local_runtime() -> None:
    for name in WORKFLOWS:
        text = (ROOT / ".github" / "workflows" / name).read_text(encoding="utf-8")
        assert "3.15.1" in text, name
        assert "scripts/verify_python315_runtime.py" in text, name
        assert 'python-version: "3.11"' not in text, name
        assert 'python-version: "3.12"' not in text, name
        assert 'python-version: "3.15"' not in text, name


def test_runtime_verifier_is_fail_closed_and_virtualenv_scoped() -> None:
    text = (ROOT / "scripts" / "verify_python315_runtime.py").read_text(encoding="utf-8")
    assert "REQUIRED_VERSION = (3, 15, 1)" in text
    assert "sys.prefix == sys.base_prefix" in text
    assert 'repository / ".venv"' in text
