import json
from pathlib import Path

from scripts.validate_registry_submission_ledger import load_ledger, validate


LEDGER = Path(__file__).parents[1] / "packaging" / "registry-submissions.json"


def test_registry_submission_ledger_is_valid():
    assert validate(load_ledger(LEDGER)) == []


def test_live_target_requires_public_evidence():
    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    ledger["targets"][0]["evidence"] = None
    assert any("evidence is required" in error for error in validate(ledger))


def test_planned_ai_plugin_requires_submission_route():
    ledger = json.loads(LEDGER.read_text(encoding="utf-8"))
    target = next(item for item in ledger["targets"] if item["id"] == "codex-plugins")
    target["submission_url"] = None
    assert any("submission_url is required" in error for error in validate(ledger))
