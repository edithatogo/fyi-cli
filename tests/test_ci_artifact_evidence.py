"""Keep mandatory CI evidence useful when the command being recorded fails."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/ci.yml"
BASH = shutil.which("bash")
EVIDENCE_JOBS = {
    "python-tests": "python-tests",
    "audit": "security-audit",
    "coverage": "coverage",
}


def jobs() -> dict:
    return yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))["jobs"]


def step(job: str, name: str) -> dict:
    return next(item for item in jobs()[job]["steps"] if item.get("name") == name)


@pytest.mark.parametrize(("job", "directory"), EVIDENCE_JOBS.items())
def test_evidence_is_initialized_before_tool_setup_and_retained_on_failure(
    job: str,
    directory: str,
) -> None:
    steps = jobs()[job]["steps"]
    provenance = steps[1]
    assert provenance["name"] == "Record execution provenance"
    assert provenance["shell"] == "bash"
    assert provenance["env"]["SOURCE_HEAD_SHA"] == (
        "${{ github.event.pull_request.head.sha || github.sha }}"
    )
    for field in (
        "git rev-parse HEAD",
        "event_sha=",
        "source_head_sha=",
        "checkout_sha=",
        "run_id=",
        "run_attempt=",
        "run_url=",
    ):
        assert field in provenance["run"]
    assert f"artifacts/{directory}/environment.txt" in provenance["run"]

    upload = steps[-1]
    assert upload["if"] == "always()"
    assert upload["uses"] == ("actions/upload-artifact@b7c566a772e6b6bfb58ed0dc250532a479d7789f")
    assert upload["with"]["retention-days"] == 14
    assert upload["with"]["if-no-files-found"] == "error"
    assert f"artifacts/{directory}/" in upload["with"]["path"]
    for field in ("github.sha", "github.run_id", "github.run_attempt"):
        assert f"${{{{ {field} }}}}" in upload["with"]["name"]


def test_python_evidence_contains_junit_and_runtime_versions() -> None:
    test = step("python-tests", "Run Python test suite")
    assert "uv run pytest -q --junitxml=artifacts/python-tests/junit.xml" in test["run"]
    install = step("python-tests", "Install Python dependencies")["run"]
    for command in ("uv --version", "uv run python --version", "uv run pytest --version"):
        assert command in install


def test_audit_and_coverage_evidence_preserve_gate_commands_and_versions() -> None:
    audit = step("audit", "Run security audit")["run"]
    for advisory in (
        "RUSTSEC-2026-0387",
        "RUSTSEC-2026-0459",
        "RUSTSEC-2026-0370",
        "RUSTSEC-2026-0371",
        "RUSTSEC-2026-0372",
    ):
        assert f"--ignore {advisory}" in audit
    assert "cargo deny check" in step("audit", "Run cargo-deny")["run"]
    assert "cargo audit --version" in step("audit", "Install cargo-audit")["run"]
    assert "cargo deny --version" in step("audit", "Install cargo-deny")["run"]

    coverage = step("coverage", "Run tests with coverage")["run"]
    assert "cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info" in coverage
    assert "cargo llvm-cov report --summary-only --fail-under-lines 90" in coverage
    assert "test -f lcov.info" in coverage
    assert "lcov.info" in jobs()["coverage"]["steps"][-1]["with"]["path"]
    versions = step("coverage", "Record coverage tool versions")["run"]
    for command in ("rustc --version --verbose", "cargo --version", "cargo llvm-cov --version"):
        assert command in versions
    assert step("coverage", "Upload coverage reports to Codecov")["with"]["fail_ci_if_error"]


@pytest.mark.parametrize("job", EVIDENCE_JOBS)
def test_recording_commands_keep_bash_pipefail_and_mandatory_failure_behavior(job: str) -> None:
    definition = jobs()[job]
    assert not definition.get("continue-on-error", False)
    for item in definition["steps"]:
        if "| tee " in item.get("run", ""):
            assert item["shell"] == "bash"
            assert "set -o pipefail" in item["run"]
            assert not item.get("continue-on-error", False)


@pytest.mark.parametrize(
    ("job", "name", "executable", "log"),
    [
        ("python-tests", "Run Python test suite", "uv", "python-tests/pytest.log"),
        ("audit", "Run security audit", "cargo", "security-audit/audit.log"),
        ("audit", "Run cargo-deny", "cargo", "security-audit/deny.log"),
        ("coverage", "Run tests with coverage", "cargo", "coverage/tests.log"),
    ],
)
def test_real_workflow_pipeline_preserves_tool_failure_and_output(
    tmp_path: Path,
    job: str,
    name: str,
    executable: str,
    log: str,
) -> None:
    assert BASH is not None, "The Ubuntu CI workflow requires Bash"
    command = tmp_path / executable
    command.write_text("#!/bin/bash\nprintf 'failure evidence\\n' >&2\nexit 23\n", encoding="utf-8")
    command.chmod(0o700)
    destination = tmp_path / "artifacts" / log
    destination.parent.mkdir(parents=True)
    environment = dict(os.environ, PATH=f"{tmp_path}{os.pathsep}{os.environ['PATH']}")
    completed = subprocess.run(  # noqa: S603 - execute the repository workflow in an isolated fixture
        [BASH, "--noprofile", "--norc", "-e", "-c", step(job, name)["run"]],
        cwd=tmp_path,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
        timeout=10,
    )
    assert completed.returncode == 23, completed
    assert destination.read_text(encoding="utf-8") == "failure evidence\n"


def test_coverage_summary_failure_survives_successful_test_log_pipeline(tmp_path: Path) -> None:
    assert BASH is not None, "The Ubuntu CI workflow requires Bash"
    cargo = tmp_path / "cargo"
    cargo.write_text(
        '#!/bin/bash\nprintf "%s\\n" "$*" >> cargo-calls.log\n'
        'if [[ "$2" == "report" ]]; then\n'
        "  printf 'coverage below threshold\\n' >&2\n  exit 37\nfi\n"
        "printf 'coverage tests succeeded\\n'\nprintf 'TN:fixture\\n' > lcov.info\n",
        encoding="utf-8",
    )
    cargo.chmod(0o700)
    (tmp_path / "artifacts/coverage").mkdir(parents=True)
    environment = dict(os.environ, PATH=f"{tmp_path}{os.pathsep}{os.environ['PATH']}")
    completed = subprocess.run(  # noqa: S603 - run the actual coverage shell with a controlled stub
        [
            BASH,
            "--noprofile",
            "--norc",
            "-e",
            "-c",
            step("coverage", "Run tests with coverage")["run"],
        ],
        cwd=tmp_path,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
        timeout=10,
    )
    assert completed.returncode == 37, completed
    assert (tmp_path / "artifacts/coverage/summary.log").read_text(encoding="utf-8") == (
        "coverage below threshold\n"
    )
    assert "--fail-under-lines 90" in (tmp_path / "cargo-calls.log").read_text(encoding="utf-8")
