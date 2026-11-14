"""Prevent regressions in action immutability and workflow token isolation."""

from __future__ import annotations

import re
from pathlib import Path

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github/workflows"
WORKFLOW_PATHS = sorted(path for path in WORKFLOWS.iterdir() if path.suffix in {".yml", ".yaml"})
# These lanes have dedicated contracts and separate implementation ownership.
SEPARATELY_MANAGED = {
    "ci.yml",
    "frontier-conductor.yml",
    "mutation.yml",
    "profiling.yml",
    "python-security-fuzzing.yml",
    "python315-compatibility.yml",
    "receipt-portability.yml",
}
MANAGED_PATHS = [path for path in WORKFLOW_PATHS if path.name not in SEPARATELY_MANAGED]
WRITE_PERMISSIONS = {
    ("codeql.yml", "analyze"): {"security-events"},
    ("container-publish.yml", "build-and-push"): {"packages"},
    ("docs-site.yml", "deploy"): {"pages", "id-token"},
    ("external-registry-monitor.yml", "monitor"): {"issues"},
    ("issue-maintenance.yml", "triage"): {"issues"},
    ("release-please.yml", "release-please"): {"contents", "pull-requests"},
    ("release.yml", "github-release"): {"contents", "id-token", "attestations"},
}


def workflow(path: Path) -> dict:
    """Load the repository's existing YAML format without executing workflow code."""
    return yaml.safe_load(path.read_text(encoding="utf-8"))


@pytest.mark.parametrize("path", WORKFLOW_PATHS, ids=lambda path: path.name)
def test_all_remote_actions_and_reusable_workflows_are_commit_pinned(path: Path) -> None:
    uses = []
    for job in workflow(path)["jobs"].values():
        if "uses" in job:
            uses.append(job["uses"])
        uses.extend(step["uses"] for step in job.get("steps", []) if "uses" in step)
    assert uses, path.name
    for reference in uses:
        if reference.startswith("./"):
            continue
        assert re.fullmatch(r"[^@]+@[0-9a-f]{40}", reference), (path.name, reference)


@pytest.mark.parametrize("path", MANAGED_PATHS, ids=lambda path: path.name)
def test_managed_workflows_serialize_runs_and_bound_jobs(path: Path) -> None:
    definition = workflow(path)
    concurrency = definition["concurrency"]
    assert concurrency["group"]
    assert concurrency["cancel-in-progress"] in (
        False,
        "${{ github.event_name == 'pull_request' }}",
    )
    for job in definition["jobs"].values():
        if "uses" in job:
            continue  # The pinned reusable workflow owns its execution timeouts.
        assert 0 < job["timeout-minutes"] <= 120


@pytest.mark.parametrize("path", WORKFLOW_PATHS, ids=lambda path: path.name)
def test_every_workflow_has_explicit_permissions_and_concurrency(path: Path) -> None:
    definition = workflow(path)
    assert isinstance(definition.get("permissions"), dict), path.name
    concurrency = definition.get("concurrency", {})
    assert concurrency.get("group"), path.name
    assert concurrency.get("cancel-in-progress") in (
        False,
        "${{ github.event_name == 'pull_request' }}",
    ), path.name


@pytest.mark.parametrize("path", MANAGED_PATHS, ids=lambda path: path.name)
def test_write_permissions_belong_only_to_jobs_that_need_them(path: Path) -> None:
    definition = workflow(path)
    assert isinstance(definition["permissions"], dict)
    for name, job in definition["jobs"].items():
        effective = job.get("permissions", definition["permissions"])
        assert isinstance(effective, dict)
        writes = {permission for permission, value in effective.items() if value == "write"}
        assert writes == WRITE_PERMISSIONS.get((path.name, name), set())


@pytest.mark.parametrize("path", MANAGED_PATHS, ids=lambda path: path.name)
def test_checkout_does_not_leave_credentials_for_repository_code(path: Path) -> None:
    for job in workflow(path)["jobs"].values():
        for step in job.get("steps", []):
            if step.get("uses", "").startswith("actions/checkout@"):
                assert step.get("with", {}).get("persist-credentials") is False


def test_actionlint_fails_on_all_findings_without_repository_write_access() -> None:
    definition = workflow(WORKFLOWS / "actionlint.yml")
    assert definition["permissions"] == {"contents": "read"}
    steps = definition["jobs"]["actionlint"]["steps"]
    lint = next(step for step in steps if "reviewdog/action-actionlint@" in step.get("uses", ""))
    assert lint["with"]["reporter"] == "local"
    assert lint["with"]["filter_mode"] == "nofilter"
    assert lint["with"]["fail_level"] == "any"
    assert not lint.get("continue-on-error", False)


def test_dependency_review_retains_its_mandatory_vulnerability_threshold() -> None:
    definition = workflow(WORKFLOWS / "dependency-review.yml")
    job = definition["jobs"]["dependency-review"]
    assert not job.get("continue-on-error", False)
    review = next(step for step in job["steps"] if step.get("name") == "Dependency Review")
    assert review["with"]["fail-on-severity"] == "high"
    assert not review.get("continue-on-error", False)


def test_release_and_pages_permissions_remain_available_only_for_publication() -> None:
    release = workflow(WORKFLOWS / "release.yml")
    assert release["permissions"] == {"contents": "read"}
    assert release["jobs"]["github-release"]["permissions"] == {
        "contents": "write",
        "id-token": "write",
        "attestations": "write",
    }
    docs = workflow(WORKFLOWS / "docs-site.yml")
    assert docs["permissions"] == {"contents": "read"}
    assert docs["jobs"]["build"]["permissions"] == {"contents": "read", "pages": "read"}
    assert docs["jobs"]["deploy"]["permissions"] == {"pages": "write", "id-token": "write"}


def test_optional_mirror_can_read_ghcr_without_write_access() -> None:
    container = workflow(WORKFLOWS / "container-publish.yml")
    assert container["permissions"] == {"contents": "read"}
    assert container["jobs"]["build-and-push"]["permissions"] == {
        "contents": "read",
        "packages": "write",
    }
    assert container["jobs"]["dockerhub-mirror"]["permissions"] == {"packages": "read"}
