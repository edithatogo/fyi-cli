"""Preserve mandatory CI gates while bounding their runner permissions and time."""

from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]


def test_ci_jobs_have_explicit_time_bounds() -> None:
    workflow = yaml.safe_load((ROOT / ".github/workflows/ci.yml").read_text())
    for name, job in workflow["jobs"].items():
        assert 1 <= job.get("timeout-minutes", 0) <= 45, name


def test_ci_write_permission_is_limited_to_coverage_oidc() -> None:
    workflow = yaml.safe_load((ROOT / ".github/workflows/ci.yml").read_text())
    assert workflow["permissions"] == {"contents": "read"}
    for name, job in workflow["jobs"].items():
        expected = {"contents": "read"}
        if name == "coverage":
            expected["id-token"] = "write"
        assert job.get("permissions", workflow["permissions"]) == expected, name


def test_mandatory_gate_commands_and_coverage_threshold_are_preserved() -> None:
    workflow = yaml.safe_load((ROOT / ".github/workflows/ci.yml").read_text())
    expected = {
        "python-tests": "uv run pytest -q",
        "property-tests": "uv run python scripts/run_property_tests.py",
        "fmt": "cargo fmt --all -- --check",
        "clippy": "cargo clippy --workspace --all-targets -- -D warnings",
        "test": "cargo test --workspace --all-features",
        "package-smoke": "./target/release/fyi-cli --help",
        "audit": "cargo deny check",
        "coverage": "--fail-under-lines 90",
        "harness-contract": "uv run python scripts/verify_test_harness.py --run",
        "packaging-assets": "python3 scripts/verify_packaging_assets.py",
    }
    assert set(workflow["jobs"]) == set(expected)
    for name, command in expected.items():
        job = workflow["jobs"][name]
        assert not job.get("continue-on-error", False), name
        steps = job["steps"]
        assert all(not step.get("continue-on-error", False) for step in steps), name
        assert command in "\n".join(step.get("run", "") for step in steps), name


def test_harness_runs_its_inventory_validation_once() -> None:
    workflow = yaml.safe_load((ROOT / ".github/workflows/ci.yml").read_text())
    steps = workflow["jobs"]["harness-contract"]["steps"]
    invocations = [
        step["run"] for step in steps if "scripts/verify_test_harness.py" in step.get("run", "")
    ]
    assert invocations == ["uv run python scripts/verify_test_harness.py --run"]


def test_compatibility_sensor_cancels_only_superseded_pr_runs() -> None:
    path = ROOT / ".github/workflows/python315-compatibility.yml"
    workflow = yaml.safe_load(path.read_text())
    concurrency = workflow.get("concurrency", {})
    assert "github.workflow" in concurrency.get("group", "")
    assert "github.event.pull_request.number || github.ref" in concurrency.get("group", "")
    assert concurrency.get("cancel-in-progress") == "${{ github.event_name == 'pull_request' }}"
