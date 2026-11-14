"""Preserve mandatory CI gates while bounding their runner permissions and time."""

import shutil
import subprocess
import tomllib
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


def test_harness_uses_job_local_rustup_state(tmp_path: Path) -> None:
    workflow = yaml.safe_load((ROOT / ".github/workflows/ci.yml").read_text())
    job = workflow["jobs"]["harness-contract"]
    isolation = next(step for step in job["steps"] if step.get("name") == "Isolate Rustup state")
    assert isolation["shell"] == "bash"
    runner_temp = tmp_path / "runner temporary directory"
    runner_temp.mkdir()
    job_environment = tmp_path / "job environment file"
    job_environment.write_text("EXISTING_KEY=preserved\n", encoding="utf-8")
    existing_home = tmp_path / "existing rustup state"
    existing_home.mkdir()
    marker = existing_home / "toolchain-marker"
    marker.write_text("original toolchain state", encoding="utf-8")
    bash = shutil.which("bash")
    assert bash is not None

    # Execute the checked-in setup with isolated inputs; no real toolchain is touched.
    completed = subprocess.run(  # noqa: S603
        [bash, "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", isolation["run"]],
        cwd=tmp_path,
        env={
            "RUNNER_TEMP": str(runner_temp),
            "GITHUB_ENV": str(job_environment),
            "RUSTUP_HOME": str(existing_home),
        },
        capture_output=True,
        text=True,
        check=False,
        timeout=10,
    )
    assert completed.returncode == 0, completed.stdout + completed.stderr
    exports = dict(
        line.split("=", 1) for line in job_environment.read_text(encoding="utf-8").splitlines()
    )
    assert exports["RUSTUP_HOME"] == str(runner_temp / "fyi-harness-rustup")
    assert exports["EXISTING_KEY"] == "preserved"
    assert marker.read_text(encoding="utf-8") == "original toolchain state"
    assert "RUSTUP_HOME" not in workflow.get("env", {})


def test_harness_installs_canonical_toolchain_before_running_rust_layers() -> None:
    workflow = yaml.safe_load((ROOT / ".github/workflows/ci.yml").read_text())
    steps = workflow["jobs"]["harness-contract"]["steps"]
    installs = [
        (index, step)
        for index, step in enumerate(steps)
        if step.get("uses", "").startswith("dtolnay/rust-toolchain@")
    ]
    assert len(installs) == 1
    index, install = installs[0]
    workspace_setup = next(
        step["uses"]
        for step in workflow["jobs"]["test"]["steps"]
        if step.get("uses", "").startswith("dtolnay/rust-toolchain@")
    )
    assert install["uses"] == workspace_setup
    toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]
    # This pinned action installs with --profile minimal; it has no profile input.
    assert toolchain["profile"] == "minimal"
    assert install["with"]["toolchain"] == toolchain["channel"]
    assert set(install["with"]["components"].split(",")) == set(toolchain["components"])
    assert not install.get("continue-on-error", False)
    checkout = next(
        i for i, step in enumerate(steps) if step.get("uses", "").startswith("actions/checkout@")
    )
    isolation = next(
        i for i, step in enumerate(steps) if step.get("name") == "Isolate Rustup state"
    )
    harness = next(
        i for i, step in enumerate(steps) if "scripts/verify_test_harness.py" in step.get("run", "")
    )
    assert checkout < isolation < index < harness


def test_compatibility_sensor_cancels_only_superseded_pr_runs() -> None:
    path = ROOT / ".github/workflows/python315-compatibility.yml"
    workflow = yaml.safe_load(path.read_text())
    concurrency = workflow.get("concurrency", {})
    assert "github.workflow" in concurrency.get("group", "")
    assert "github.event.pull_request.number || github.ref" in concurrency.get("group", "")
    assert concurrency.get("cancel-in-progress") == "${{ github.event_name == 'pull_request' }}"
