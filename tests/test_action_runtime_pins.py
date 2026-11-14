"""Guard the reviewed cache, artifact and Python-setup runtime migration."""
from __future__ import annotations

import re
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ("ci.yml", "python-security-fuzzing.yml", "python315-compatibility.yml")
# Verified upstream action.yml declarations use node20 at these immutable refs.
RETIRED_PINS = {
    ("actions/cache", "0057852bfaa89a56745cba8c7296529d2fc39830"),
    ("actions/setup-python", "a26af69be951a213d495a4c3e4e4022e16d87065"),
    ("actions/upload-artifact", "ea165f8d65b6e75b540449e92b4886f43607fa02"),
}
ACTION_PATTERN = re.compile(
    r"^\s*(?:-\s+)?uses:\s*(actions/(?:cache|setup-python|upload-artifact))@(\S+)",
    re.MULTILINE,
)


def action_refs(workflow: str) -> list[tuple[str, str]]:
    text = (ROOT / ".github" / "workflows" / workflow).read_text(encoding="utf-8")
    refs = ACTION_PATTERN.findall(text)
    assert refs, f"No migration-scope actions found in {workflow}"
    return refs


@pytest.mark.parametrize("workflow", WORKFLOWS)
def test_migrated_actions_remain_immutable_pins(workflow: str) -> None:
    for action, ref in action_refs(workflow):
        assert re.fullmatch(r"[0-9a-f]{40}", ref), f"Unpinned action: {action}@{ref}"


@pytest.mark.parametrize("workflow", WORKFLOWS)
def test_known_retired_runtime_pins_are_not_reintroduced(workflow: str) -> None:
    retired = set(action_refs(workflow)) & RETIRED_PINS
    assert not retired, f"Retired runtime action pins in {workflow}: {sorted(retired)}"
