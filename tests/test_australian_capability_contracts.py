from __future__ import annotations

import copy
import hashlib
import json
import shutil
from pathlib import Path
from typing import Any

import pytest
from jsonschema import Draft202012Validator, FormatChecker, ValidationError

from fyi_system.australian_capabilities import (
    CAPABILITY_IDS,
    EXPECTED_LEGACY_SHA256,
    JURISDICTION_TO_PROFILE,
    PLATFORM_RELATIVE_PATH,
    CapabilityContractError,
    canonical_sha256,
    load_jurisdiction_record,
    load_platform_contract,
    require_explicit_jurisdiction,
    sha256_file,
    validate_jurisdiction_record,
    validate_platform_contract,
)

ROOT = Path(__file__).parent.parent
PLATFORM_SCHEMA = ROOT / "schemas" / "australian-platform-capability-contract.schema.json"
RECORD_SCHEMA = ROOT / "schemas" / "australian-jurisdiction-capability-record.schema.json"
LEGACY_AUDIT = ROOT / "artifacts" / "foio" / "australian_capture_capability_audit.json"


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    assert isinstance(value, dict)
    return value


def validator(path: Path) -> Draft202012Validator:
    return Draft202012Validator(read_json(path), format_checker=FormatChecker())


def isolated_contract_root(tmp_path: Path) -> Path:
    root = tmp_path / "repository"
    for relative_path in (
        PLATFORM_RELATIVE_PATH,
        LEGACY_AUDIT.relative_to(ROOT),
        Path("src/fyi_system/archive_capture.py"),
        PLATFORM_SCHEMA.relative_to(ROOT),
        RECORD_SCHEMA.relative_to(ROOT),
    ):
        destination = root / relative_path
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / relative_path, destination)
    records = ROOT / "artifacts" / "foio" / "australian-capabilities"
    for source in records.glob("au-*.contract-only.json"):
        shutil.copy2(source, root / source.relative_to(ROOT))
    return root


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def test_shared_platform_contract_is_schema_valid_and_semantically_pinned() -> None:
    platform = load_platform_contract(ROOT)

    validator(PLATFORM_SCHEMA).validate(platform)
    assert platform["contract_status"] == "code_inspection_only"
    assert platform["live_access_performed"] is False
    assert set(platform["declared_capability_ids"]) == CAPABILITY_IDS
    assert canonical_sha256(platform["bounded_url_scope"]) == platform["bounded_url_scope_sha256"]


@pytest.mark.parametrize("jurisdiction_id", sorted(JURISDICTION_TO_PROFILE))
def test_contract_only_record_is_valid_disabled_and_pinned(jurisdiction_id: str) -> None:
    record = load_jurisdiction_record(ROOT, jurisdiction_id)

    validator(RECORD_SCHEMA).validate(record)
    assert record["profile_id"] == JURISDICTION_TO_PROFILE[jurisdiction_id]
    assert record["record_status"] == "contract_only_disabled"
    assert record["activation"]["enabled"] is False
    assert record["legal_context"]["source_evidence"] == []
    assert record["platform_contract"]["sha256"] == sha256_file(
        ROOT / PLATFORM_RELATIVE_PATH,
    )


def test_cth_nsw_historical_artifact_remains_exact_regression_oracle() -> None:
    assert hashlib.sha256(LEGACY_AUDIT.read_bytes()).hexdigest() == EXPECTED_LEGACY_SHA256
    historical = read_json(LEGACY_AUDIT)
    assert [item["id"] for item in historical["jurisdictions"]] == ["AU-CTH", "AU-NSW"]


@pytest.mark.parametrize("jurisdiction_id", [None, "", "AU", "AU-CTH", "AU-NSW", "../AU-VIC"])
def test_unknown_or_unregistered_jurisdiction_fails_closed(
    jurisdiction_id: str | None,
) -> None:
    with pytest.raises(CapabilityContractError, match="unknown or absent"):
        require_explicit_jurisdiction(jurisdiction_id)
    with pytest.raises(CapabilityContractError, match="unknown or absent"):
        load_jurisdiction_record(ROOT, jurisdiction_id)


def test_platform_schema_and_semantic_validator_reject_unknown_capability() -> None:
    platform = load_platform_contract(ROOT)
    invalid = copy.deepcopy(platform)
    invalid["declared_capability_ids"][-1] = "invented_live_capture"

    with pytest.raises(ValidationError):
        validator(PLATFORM_SCHEMA).validate(invalid)
    with pytest.raises(CapabilityContractError, match="platform contract schema"):
        validate_platform_contract(invalid, root=ROOT)


def test_record_schema_and_semantic_validator_reject_unknown_capability() -> None:
    platform = load_platform_contract(ROOT)
    platform_sha256 = sha256_file(ROOT / PLATFORM_RELATIVE_PATH)
    record = load_jurisdiction_record(ROOT, "AU-VIC")
    invalid = copy.deepcopy(record)
    invalid["capability_ids"].append("unregistered_capability")

    with pytest.raises(ValidationError):
        validator(RECORD_SCHEMA).validate(invalid)
    with pytest.raises(CapabilityContractError, match="jurisdiction record schema"):
        validate_jurisdiction_record(
            invalid,
            platform=platform,
            platform_sha256=platform_sha256,
            root=ROOT,
        )


def test_record_rejects_cross_jurisdiction_profile_mapping() -> None:
    platform = load_platform_contract(ROOT)
    record = load_jurisdiction_record(ROOT, "AU-VIC")
    invalid = copy.deepcopy(record)
    invalid["profile_id"] = "foi-o-au-qld"

    with pytest.raises(ValidationError):
        validator(RECORD_SCHEMA).validate(invalid)
    with pytest.raises(CapabilityContractError, match="jurisdiction record schema"):
        validate_jurisdiction_record(
            invalid,
            platform=platform,
            platform_sha256=sha256_file(ROOT / PLATFORM_RELATIVE_PATH),
            root=ROOT,
        )


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("record_status", "enabled"),
        ("jurisdiction_classification", {"mode": "infer_from_request_text"}),
        ("activation", {"enabled": True}),
    ],
)
def test_record_rejects_activation_or_inferred_classification(
    field: str,
    value: object,
) -> None:
    platform = load_platform_contract(ROOT)
    record = load_jurisdiction_record(ROOT, "AU-SA")
    invalid = copy.deepcopy(record)
    invalid[field] = value

    with pytest.raises(ValidationError):
        validator(RECORD_SCHEMA).validate(invalid)
    with pytest.raises(CapabilityContractError):
        validate_jurisdiction_record(
            invalid,
            platform=platform,
            platform_sha256=sha256_file(ROOT / PLATFORM_RELATIVE_PATH),
            root=ROOT,
        )


def test_record_rejects_unpinned_platform_or_adapter_revision() -> None:
    platform = load_platform_contract(ROOT)
    record = load_jurisdiction_record(ROOT, "AU-NT")

    wrong_platform = copy.deepcopy(record)
    wrong_platform["platform_contract"]["sha256"] = "0" * 64
    with pytest.raises(CapabilityContractError, match="platform contract pin"):
        validate_jurisdiction_record(
            wrong_platform,
            platform=platform,
            platform_sha256=sha256_file(ROOT / PLATFORM_RELATIVE_PATH),
            root=ROOT,
        )

    wrong_adapter = copy.deepcopy(record)
    wrong_adapter["adapter_revision"]["module_sha256"] = "0" * 64
    with pytest.raises(CapabilityContractError, match="jurisdiction record schema"):
        validate_jurisdiction_record(
            wrong_adapter,
            platform=platform,
            platform_sha256=sha256_file(ROOT / PLATFORM_RELATIVE_PATH),
            root=ROOT,
        )


def test_schema_rejects_legal_claims_or_authentic_source_evidence() -> None:
    record = load_jurisdiction_record(ROOT, "AU-ACT")
    invalid = copy.deepcopy(record)
    invalid["legal_context"] = {
        "status": "verified",
        "regime_id": "invented",
        "authority_registry": "invented",
        "effective_date": "2026-07-31",
        "source_evidence": ["invented"],
    }

    with pytest.raises(ValidationError):
        validator(RECORD_SCHEMA).validate(invalid)


@pytest.mark.parametrize(
    ("path", "value"),
    [
        (("rate_and_terms_policy", "live_access_allowed"), True),
        (("live_access_performed",), True),
        (("recorded_at",), "not-a-date-time"),
    ],
)
def test_platform_loader_enforces_schema_before_semantic_pins(
    tmp_path: Path,
    path: tuple[str, ...],
    value: object,
) -> None:
    root = isolated_contract_root(tmp_path)
    contract_path = root / PLATFORM_RELATIVE_PATH
    contract = read_json(contract_path)
    target: dict[str, Any] = contract
    for key in path[:-1]:
        child = target[key]
        assert isinstance(child, dict)
        target = child
    target[path[-1]] = value
    write_json(contract_path, contract)

    with pytest.raises(CapabilityContractError, match="platform contract schema"):
        load_platform_contract(root)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("prohibitions", []),
        ("activation", {"enabled": False, "prerequisites": []}),
        (
            "legal_context",
            {
                "status": "unverified_contract_only",
                "regime_id": None,
                "authority_registry": None,
                "effective_date": None,
                "source_evidence": ["invented-source"],
            },
        ),
        ("recorded_at", "31 July 2026"),
    ],
)
def test_jurisdiction_loader_enforces_schema_before_semantic_pins(
    tmp_path: Path,
    field: str,
    value: object,
) -> None:
    root = isolated_contract_root(tmp_path)
    record_path = root / "artifacts/foio/australian-capabilities/au-vic.contract-only.json"
    record = read_json(record_path)
    record[field] = value
    write_json(record_path, record)

    with pytest.raises(CapabilityContractError, match="jurisdiction record schema"):
        load_jurisdiction_record(root, "AU-VIC")


def test_loaders_reject_non_object_contracts_via_schema_boundary(tmp_path: Path) -> None:
    root = isolated_contract_root(tmp_path)
    write_json(root / PLATFORM_RELATIVE_PATH, [])

    with pytest.raises(CapabilityContractError, match="platform contract schema"):
        load_platform_contract(root)


def test_platform_loader_rejects_resealed_schema_and_live_access(tmp_path: Path) -> None:
    root = isolated_contract_root(tmp_path)
    schema_path = root / PLATFORM_SCHEMA.relative_to(ROOT)
    schema = read_json(schema_path)
    schema["properties"]["live_access_performed"] = {"type": "boolean"}
    schema["properties"]["rate_and_terms_policy"]["properties"]["live_access_allowed"] = {
        "type": "boolean",
    }
    write_json(schema_path, schema)

    contract_path = root / PLATFORM_RELATIVE_PATH
    contract = read_json(contract_path)
    contract["live_access_performed"] = True
    contract["rate_and_terms_policy"]["live_access_allowed"] = True
    write_json(contract_path, contract)

    with pytest.raises(CapabilityContractError, match="trusted platform schema pin"):
        load_platform_contract(root)


def test_record_loader_rejects_resealed_schema_and_policy_evidence_bypass(
    tmp_path: Path,
) -> None:
    root = isolated_contract_root(tmp_path)
    schema_path = root / RECORD_SCHEMA.relative_to(ROOT)
    schema = read_json(schema_path)
    schema["properties"]["prohibitions"] = {"type": "array"}
    schema["properties"]["activation"] = {"type": "object"}
    schema["properties"]["legal_context"] = {"type": "object"}
    write_json(schema_path, schema)

    record_path = root / "artifacts/foio/australian-capabilities/au-vic.contract-only.json"
    record = read_json(record_path)
    record["prohibitions"] = []
    record["activation"] = {"enabled": False, "prerequisites": []}
    record["legal_context"]["source_evidence"] = ["fabricated-source"]
    write_json(record_path, record)

    with pytest.raises(CapabilityContractError, match="trusted jurisdiction schema pin"):
        load_jurisdiction_record(root, "AU-VIC")


def test_public_platform_validator_rejects_live_access_without_loader() -> None:
    platform = load_platform_contract(ROOT)
    invalid = copy.deepcopy(platform)
    invalid["live_access_performed"] = True
    invalid["rate_and_terms_policy"]["live_access_allowed"] = True

    with pytest.raises(CapabilityContractError, match="platform contract schema"):
        validate_platform_contract(invalid, root=ROOT)


def test_public_record_validator_rejects_policy_and_evidence_without_loader() -> None:
    platform = load_platform_contract(ROOT)
    record = load_jurisdiction_record(ROOT, "AU-VIC")
    invalid = copy.deepcopy(record)
    invalid["prohibitions"] = []
    invalid["activation"]["prerequisites"] = []
    invalid["legal_context"]["source_evidence"] = ["fabricated-source"]

    with pytest.raises(CapabilityContractError, match="jurisdiction record schema"):
        validate_jurisdiction_record(
            invalid,
            platform=platform,
            platform_sha256=sha256_file(ROOT / PLATFORM_RELATIVE_PATH),
            root=ROOT,
        )


def test_platform_loader_rejects_fabricated_repository_revision(tmp_path: Path) -> None:
    root = isolated_contract_root(tmp_path)
    contract_path = root / PLATFORM_RELATIVE_PATH
    contract = read_json(contract_path)
    contract["repository_revision"] = "f" * 40
    write_json(contract_path, contract)

    with pytest.raises(CapabilityContractError, match="registered repository revision"):
        load_platform_contract(root)
