from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).parent.parent
MANIFEST = ROOT / "artifacts" / "foio" / "foundation_capture_handoff_manifest.json"
SCHEMA = ROOT / "schemas" / "foundation-capture-handoff-manifest.schema.json"
EVIDENCE = (
    ROOT
    / ".conductor"
    / "tracks"
    / "jurisdiction-capture-completion-20260721"
    / "evidence.jsonl"
)
METADATA = (
    ROOT
    / ".conductor"
    / "tracks"
    / "jurisdiction-capture-completion-20260721"
    / "metadata.json"
)


def test_foundation_handoff_manifest_is_schema_valid_and_non_executing() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))

    Draft202012Validator(schema).validate(manifest)
    assert manifest["live_capture_performed"] is False
    assert manifest["summary"] == {
        "bounded_targets_total": 7,
        "implementation_ready_with_authorized_nonempty_capture": 4,
        "blocked_pending_external_source_or_policy": 3,
    }


def test_foundation_handoff_manifest_matches_track_evidence_and_blockers() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    metadata = json.loads(METADATA.read_text(encoding="utf-8"))
    evidence_rows = [
        json.loads(line)
        for line in EVIDENCE.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    evidence_by_artifact = {
        row["artifact"]: row["artifact_sha256"]
        for row in evidence_rows
        if "artifact" in row and "artifact_sha256" in row
    }

    assert {
        entry["blocking_gate"] for entry in manifest["jurisdiction_audits"]
    } == {gate["id"] for gate in metadata["gates"]}
    assert {blocker["id"] for blocker in manifest["external_blockers"]} == {
        gate["id"] for gate in metadata["gates"]
    }

    for entry in manifest["jurisdiction_audits"]:
        assert (ROOT / entry["artifact"]).exists()
        assert evidence_by_artifact[entry["artifact"]] == entry["artifact_sha256"]


def test_foundation_handoff_manifest_is_grounded_in_spec_and_capture_runtime() -> None:
    spec = (
        ROOT
        / ".conductor"
        / "tracks"
        / "jurisdiction-capture-completion-20260721"
        / "spec.md"
    ).read_text(encoding="utf-8")
    capture_source = (ROOT / "src" / "fyi_system" / "archive_capture.py").read_text(
        encoding="utf-8",
    )

    assert "archive handoff manifest" in spec
    assert "Every target has a tested supported/unsupported/blocked status" in spec
    for capability_symbol in (
        "CaptureCaps",
        "get_with_retry",
        "write_warc_record",
        "package_wacz",
        "write_derived_store",
    ):
        assert capability_symbol in capture_source
