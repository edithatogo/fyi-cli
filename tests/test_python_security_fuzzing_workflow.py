"""Fail closed if the Python security and fuzzing workflow loses its bounds."""

import re
from pathlib import Path

WORKFLOW = Path(".github/workflows/python-security-fuzzing.yml")
FUZZ_REQUIREMENTS = Path("fuzzing/requirements.txt")


def _workflow() -> str:
    return WORKFLOW.read_text(encoding="utf-8")


def test_workflow_is_least_privilege_and_actions_are_commit_pinned() -> None:
    workflow = _workflow()
    assert re.search(r"(?m)^permissions:\n  contents: read$", workflow)
    action_uses = re.findall(r"uses: ([^\s#]+)", workflow)
    assert action_uses
    assert all(re.fullmatch(r"[^@]+@[0-9a-f]{40}", use) for use in action_uses)
    assert "pull_request_target:" not in workflow


def test_hypothesis_is_an_explicit_pull_request_gate() -> None:
    workflow = _workflow()
    assert "hypothesis-pr-gate:" in workflow
    assert "if: github.event_name == 'pull_request'" in workflow
    assert "tests/test_hypothesis.py tests/test_fuzz.py" in workflow


def test_quality_gate_covers_acquisition_boundaries_and_strict_types() -> None:
    workflow = _workflow()
    for path in (
        "src/fyi_system/acquisition_receipts.py",
        "src/fyi_system/internet_archive_cdx.py",
        "src/fyi_system/internet_archive_replay.py",
        "tests/test_atheris_targets.py",
        "tests/test_python_security_dependencies.py",
    ):
        assert path in workflow
    assert "uv run basedpyright" in workflow
    assert "uv run pip-audit" in workflow
    assert "requirements-fuzz-audit.txt" in workflow
    assert (
        "fuzzing/requirements.txt"
        not in workflow.split("requirements-audit.txt", 1)[1].split(
            "uv run pip-audit --requirement requirements-audit.txt",
            1,
        )[0]
    )


def test_atheris_matrix_is_bounded_and_retains_failures() -> None:
    workflow = _workflow()
    assert ".venv/bin/python -m fuzzing.run_atheris" in workflow
    assert "target: [receipt, cdx, wayback, redaction]" in workflow
    for bound in (
        "-max_total_time=",
        "-timeout=5",
        "-rss_limit_mb=2048",
        "-max_len=65536",
        '-artifact_prefix="artifacts/fuzz/$TARGET/"',
    ):
        assert bound in workflow
    assert "timeout-minutes: 35" in workflow
    assert "if: failure() || cancelled()" in workflow
    assert "retention-days: 14" in workflow


def test_long_runs_are_scheduled_and_manually_bounded() -> None:
    workflow = _workflow()
    assert "schedule:" in workflow
    assert "workflow_dispatch:" in workflow
    assert "REQUESTED_SECONDS >= 30 && REQUESTED_SECONDS <= 1800" in workflow


def test_fuzzer_installation_and_audit_use_the_same_immutable_source() -> None:
    requirements = [
        line
        for line in FUZZ_REQUIREMENTS.read_text(encoding="utf-8").splitlines()
        if line and not line.startswith("#")
    ]
    assert len(requirements) == 1
    assert re.fullmatch(
        r"atheris @ git\+https://github\.com/google/atheris\.git@[0-9a-f]{40}",
        requirements[0],
    )
    workflow = _workflow()
    install = "uv pip install --python .venv/bin/python --requirement fuzzing/requirements.txt"
    assert install in workflow
    assert "cp fuzzing/requirements.txt requirements-fuzz-audit.txt" in workflow
    assert "uv run pip-audit --requirement requirements-fuzz-audit.txt" in workflow


def test_fuzzer_qualification_executes_before_bounded_targets() -> None:
    workflow = _workflow()
    qualification = workflow.split(
        "- name: Qualify Atheris on the selected interpreter",
        1,
    )[1].split("- name: Select bounded duration", 1)[0]
    for suite in (
        "version_dependent_test.py",
        "instrument_bytecode_test.py",
        "fuzz_test.py",
    ):
        assert f'.venv/bin/python "$atheris_test_dir/{suite}"' in qualification
    for case in (
        "IntegrationTests.testBytesComparison",
        "IntegrationTests.testStringComparison",
        "IntegrationTests.testUtf8Comparison",
    ):
        assert case in qualification
    assert "set -o pipefail" in qualification
    assert "artifacts/logs/qualification.log" in qualification
    assert "continue-on-error:" not in workflow
    assert "|| true" not in qualification
