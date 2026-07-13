import json
from pathlib import Path

from scripts.check_distribution_freshness import build_report


def test_freshness_skips_targets_without_public_evidence():
    report = build_report({"source_release": "0.1.2", "targets": [{"id": "planned", "status": "planned", "evidence": None}]})
    assert report["results"][0]["check"] == "skipped"


def test_freshness_report_shape_matches_ledger(monkeypatch):
    monkeypatch.setattr("scripts.check_distribution_freshness.check_url", lambda url: ("reachable", 200))
    ledger = json.loads((Path(__file__).parents[1] / "packaging" / "registry-submissions.json").read_text())
    report = build_report(ledger)
    assert len(report["results"]) == len(ledger["targets"])
    assert all("id" in item and "check" in item for item in report["results"])
