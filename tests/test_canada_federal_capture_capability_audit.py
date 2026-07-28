from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).parent.parent
AUDIT = ROOT / "artifacts" / "foio" / "canada_federal_capture_capability_audit.json"
SCHEMA = ROOT / "schemas" / "canada-federal-capture-capability-audit.schema.json"


def test_canada_federal_capability_audit_is_schema_valid_and_non_executing() -> None:
    audit = json.loads(AUDIT.read_text(encoding="utf-8"))
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))

    Draft202012Validator(schema).validate(audit)
    assert audit["live_capture_performed"] is False
    assert audit["platform"]["id"] == "ca-atip-online-request"
    assert {
        entry["route"]: entry["capture_status"] for entry in audit["support_matrix"]
    } == {
        "manual_online_request_portal": "manual_route_only_no_public_api",
        "automated_request_submission": "explicitly_prohibited_identity_consent_surface",
        "public_request_response_capture": "unsupported_no_public_listing_api",
        "public_attachment_timeline_capture": "unsupported_no_public_listing_api",
    }


def test_canada_federal_audit_is_grounded_in_documented_platform_constraints() -> None:
    landscape = (ROOT / "docs" / "foi-platform-landscape.md").read_text(
        encoding="utf-8",
    )
    evaluation = (ROOT / "docs" / "government-api-evaluation.md").read_text(
        encoding="utf-8",
    )
    capture_source = (ROOT / "src" / "fyi_system" / "archive_capture.py").read_text(
        encoding="utf-8",
    )

    assert "https://atip-aiprp.apps.gc.ca/atip/" in landscape
    assert "Canada ATIP" in landscape
    assert "## Canada ATIP Online Request portal" in evaluation
    assert "stable public API for listing requests" in evaluation
    for capability_symbol in (
        "CaptureCaps",
        "get_with_retry",
        "write_warc_record",
        "package_wacz",
        "write_derived_store",
    ):
        assert capability_symbol in capture_source
