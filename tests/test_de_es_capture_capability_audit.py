from __future__ import annotations

import json
from pathlib import Path

import tomllib
from jsonschema import Draft202012Validator

ROOT = Path(__file__).parent.parent
AUDIT = ROOT / "artifacts" / "foio" / "de_es_capture_capability_audit.json"
SCHEMA = ROOT / "schemas" / "de-es-capture-capability-audit.schema.json"


def test_de_es_capability_audit_is_schema_valid_and_non_executing() -> None:
    audit = json.loads(AUDIT.read_text(encoding="utf-8"))
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))

    Draft202012Validator(schema).validate(audit)
    assert audit["live_capture_performed"] is False
    assert {item["id"] for item in audit["instances"]} == {"de-fds", "es-tdas"}
    assert all(
        item["capture_status"] == "generic_alaveteli_capture_supported"
        for item in audit["instances"]
    )


def test_de_es_audit_is_grounded_in_registry_and_capture_implementation() -> None:
    registry = tomllib.loads(
        (ROOT / "crates" / "fyi-core" / "instances.toml").read_text(encoding="utf-8"),
    )
    ids = {item["id"] for item in registry["instances"]}
    capture_source = (ROOT / "src" / "fyi_system" / "archive_capture.py").read_text(
        encoding="utf-8",
    )

    assert {"de-fds", "es-tdas"}.issubset(ids)
    for capability_symbol in (
        "CaptureCaps",
        "get_with_retry",
        "write_warc_record",
        "package_wacz",
        "write_derived_store",
    ):
        assert capability_symbol in capture_source
