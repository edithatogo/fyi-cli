from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).parent.parent
AUDIT = ROOT / "artifacts" / "foio" / "south_africa_capture_capability_audit.json"
SCHEMA = ROOT / "schemas" / "south-africa-capture-capability-audit.schema.json"


def test_south_africa_capability_audit_is_schema_valid_and_non_executing() -> None:
    audit = json.loads(AUDIT.read_text(encoding="utf-8"))
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))

    Draft202012Validator(schema).validate(audit)
    assert audit["live_capture_performed"] is False
    assert audit["platform"]["id"] == "za-inforegulator-paia"
    assert {
        entry["route"]: entry["capture_status"] for entry in audit["support_matrix"]
    } == {
        "official_paia_guidance": "guidance_only_no_public_api",
        "automated_request_submission": "explicitly_prohibited_no_documented_public_submission_api",
        "public_request_response_capture": "unsupported_no_public_listing_api",
        "public_attachment_timeline_capture": "unsupported_no_public_listing_api",
    }


def test_south_africa_audit_is_grounded_in_documented_platform_constraints() -> None:
    evaluation = (ROOT / "docs" / "government-api-evaluation.md").read_text(
        encoding="utf-8",
    )
    landscape = (ROOT / "docs" / "foi-platform-landscape.md").read_text(
        encoding="utf-8",
    )
    capture_source = (ROOT / "src" / "fyi_system" / "archive_capture.py").read_text(
        encoding="utf-8",
    )

    assert "https://inforegulator.org.za/paia/" in evaluation
    assert "South Africa PAIA guidance" in evaluation
    assert "South Africa PAIA" in landscape
    for capability_symbol in (
        "CaptureCaps",
        "get_with_retry",
        "write_warc_record",
        "package_wacz",
        "write_derived_store",
    ):
        assert capability_symbol in capture_source
