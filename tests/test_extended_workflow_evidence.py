"""Keep extended CI lanes bounded, executable, and useful after failures."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
import yaml

import mutation_test

ROOT = Path(__file__).resolve().parents[1]


def _workflow(name: str) -> dict:
    # BaseLoader only builds plain string containers, preserving GitHub's `on`.
    loader = yaml.BaseLoader(
        (ROOT / ".github" / "workflows" / name).read_text(encoding="utf-8"),
    )
    try:
        return loader.get_single_data()
    finally:
        loader.dispose()


def _step(job: dict, name: str) -> dict:
    return next(step for step in job["steps"] if step.get("name") == name)


def test_mutation_lanes_keep_existing_gates_and_use_the_locked_environment() -> None:
    workflow = _workflow("mutation.yml")
    assert set(workflow["on"]) == {"schedule", "workflow_dispatch"}
    assert workflow["permissions"] == {"contents": "read"}
    assert workflow["concurrency"]["cancel-in-progress"] == "false"
    jobs = workflow["jobs"]
    assert (
        "cargo mutants --workspace --all-features --timeout 60"
        in _step(
            jobs["rust-mutants"],
            "Run mutation analysis",
        )["run"]
    )
    assert (
        "uv run python mutation_test.py --threshold 70"
        in _step(
            jobs["python-mutants"],
            "Run Python mutation analysis",
        )["run"]
    )
    for job in jobs.values():
        assert job["timeout-minutes"] == "45"
        assert "continue-on-error" not in job
        assert all("continue-on-error" not in step for step in job["steps"])


@pytest.mark.parametrize(
    ("workflow_name", "job_name", "directory"),
    [
        ("mutation.yml", "rust-mutants", "artifacts/mutation"),
        ("mutation.yml", "python-mutants", "artifacts/mutation"),
        ("profiling.yml", "profile", "artifacts/profiling"),
    ],
)
def test_extended_evidence_survives_failure_with_run_and_runtime_provenance(
    workflow_name: str,
    job_name: str,
    directory: str,
) -> None:
    job = _workflow(workflow_name)["jobs"][job_name]
    capture = _step(job, "Record execution provenance")
    assert directory in capture["run"]
    for field in ("GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "git rev-parse HEAD"):
        assert field in capture["run"]
    uploads = [
        step for step in job["steps"] if step.get("uses", "").startswith("actions/upload-artifact@")
    ]
    assert len(uploads) == 1
    upload = uploads[0]
    assert upload["if"] == "always()"
    assert upload["with"]["retention-days"] == "14"
    assert upload["with"]["if-no-files-found"] == "error"
    assert directory in upload["with"]["path"]
    for field in ("github.sha", "github.run_id", "github.run_attempt"):
        assert field in upload["with"]["name"]


def test_profiling_uses_bounded_pr_smoke_and_scheduled_full_analysis() -> None:
    workflow = _workflow("profiling.yml")
    assert set(workflow["on"]) == {"pull_request", "schedule", "workflow_dispatch"}
    assert workflow["permissions"] == {"contents": "read"}
    assert (
        workflow["concurrency"]["cancel-in-progress"]
        == "${{ github.event_name == 'pull_request' }}"
    )
    job = workflow["jobs"]["profile"]
    assert int(job["timeout-minutes"]) <= 30
    smoke = _step(job, "Run bounded benchmark smoke")
    assert smoke["if"] == "github.event_name == 'pull_request'"
    assert "tests/test_benchmarks.py" in smoke["run"]
    assert "python_functions=benchmark_*" in smoke["run"]
    assert "--benchmark-min-rounds=1" in smoke["run"]
    assert "--benchmark-max-time=0.05" in smoke["run"]
    for name in ("Run Scalene profiler", "Run performance tests"):
        assert _step(job, name)["if"] == "github.event_name != 'pull_request'"
    assert "python_functions=benchmark_*" in _step(job, "Run performance tests")["run"]


def test_profile_selection_collects_all_eight_existing_benchmarks() -> None:
    completed = subprocess.run(
        [
            sys.executable,
            "-m",
            "pytest",
            "--collect-only",
            "-q",
            "tests/test_benchmarks.py",
            "-o",
            "python_functions=benchmark_*",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        timeout=30,
    )
    assert completed.returncode == 0, completed.stdout + completed.stderr
    assert "8 tests collected" in completed.stdout


@pytest.mark.parametrize(
    ("job_name", "step_name", "tool"),
    [
        ("rust-mutants", "Run mutation analysis", "cargo"),
        ("python-mutants", "Run Python mutation analysis", "uv"),
    ],
)
def test_mutation_log_pipeline_preserves_failing_exit_status(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    job_name: str,
    step_name: str,
    tool: str,
) -> None:
    job = _workflow("mutation.yml")["jobs"][job_name]
    script = _step(job, step_name)["run"]
    executable = tmp_path / tool
    executable.write_text(
        "#!/bin/sh\nprintf 'mutation stdout\\n'\nprintf 'mutation stderr\\n' >&2\nexit 23\n",
        encoding="utf-8",
    )
    executable.chmod(0o755)
    bash = shutil.which("bash")
    assert bash is not None
    monkeypatch.setenv("PATH", str(tmp_path) + os.pathsep + os.environ["PATH"])
    (tmp_path / "artifacts/mutation").mkdir(parents=True)
    # The checked-in workflow is exercised against local, failing test executables.
    completed = subprocess.run(  # noqa: S603
        [bash, "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", script],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
        timeout=10,
    )
    assert completed.returncode == 23, completed.stdout + completed.stderr
    logs = list((tmp_path / "artifacts/mutation").glob("*.log"))
    assert len(logs) == 1
    assert "mutation stdout" in logs[0].read_text(encoding="utf-8")
    assert "mutation stderr" in logs[0].read_text(encoding="utf-8")


def test_python_mutation_failures_keep_original_pytest_diagnostics(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    calls = []

    def run(command, **kwargs):
        calls.append((command, kwargs))
        return subprocess.CompletedProcess(command, 1, "assertion evidence\n", "failure details\n")

    monkeypatch.setattr(mutation_test.subprocess, "run", run)
    assert mutation_test.run_tests() is False
    assert calls[0][0][0] == sys.executable
    captured = capsys.readouterr()
    assert "assertion evidence" in captured.out
    assert "failure details" in captured.err
