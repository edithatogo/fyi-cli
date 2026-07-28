from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).parent.parent
AUDIT = ROOT / "artifacts" / "foio" / "us_federal_capture_capability_audit.json"
SCHEMA = ROOT / "schemas" / "us-federal-capture-capability-audit.schema.json"


def test_us_federal_capability_audit_is_schema_valid_and_non_executing() -> None:
    audit = json.loads(AUDIT.read_text(encoding="utf-8"))
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))

    Draft202012Validator(schema).validate(audit)
    assert audit["live_capture_performed"] is False
    assert audit["platform"]["id"] == "us-foia-gov"
    assert {
        entry["route"]: entry["capture_status"] for entry in audit["support_matrix"]
    } == {
        "agency_component_catalog": "discovery_only_authorized_api_key_required",
        "agency_request_submission": "explicitly_prohibited_write_surface",
        "public_request_response_capture": "unsupported_decentralized_agency_delivery",
        "public_attachment_timeline_capture": "unsupported_no_general_public_api",
    }


def test_us_federal_audit_is_grounded_in_documented_platform_constraints() -> None:
    evaluation = (ROOT / "docs" / "government-api-evaluation.md").read_text(
        encoding="utf-8",
    )
    landscape = (ROOT / "docs" / "foi-platform-landscape.md").read_text(
        encoding="utf-8",
    )
    capture_source = (ROOT / "src" / "fyi_system" / "archive_capture.py").read_text(
        encoding="utf-8",
    )

    assert "https://api.foia.gov/api/agency_components" in evaluation
    assert "API key" in evaluation
    assert "Agency API `POST` surface" in evaluation
    assert "FOIA.gov" in landscape
    for capability_symbol in (
        "CaptureCaps",
        "get_with_retry",
        "write_warc_record",
        "package_wacz",
        "write_derived_store",
    ):
        assert capability_symbol in capture_source
