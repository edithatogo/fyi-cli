"""Fail-closed loading for synthetic Australian capture capability contracts."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, Never, cast

PLATFORM_RELATIVE_PATH = Path(
    "artifacts/foio/australian-capabilities/au-rtk-platform.v1.json",
)
LEGACY_RELATIVE_PATH = Path("artifacts/foio/australian_capture_capability_audit.json")
ADAPTER_RELATIVE_PATH = Path("src/fyi_system/archive_capture.py")

PLATFORM_SCHEMA_VERSION = "fyi-cli.australian-platform-capability-contract.v1.0.0"
RECORD_SCHEMA_VERSION = "fyi-cli.australian-jurisdiction-capability-record.v1.0.0"
EXPECTED_LEGACY_SHA256 = "d1af66697c173cfafd8de2dfd00a8d83f3be5a5401ece5fae2bac1e67c723d71"
EXPECTED_SCOPE_SHA256 = "4d59c0e6d8b6b84fb2268cf8ca8de9052ec554f5ab5f3ef9f4fa6eabcd3ca46d"

JURISDICTION_TO_PROFILE = {
    "AU-ACT": "foi-o-au-act",
    "AU-NT": "foi-o-au-nt",
    "AU-QLD": "foi-o-au-qld",
    "AU-SA": "foi-o-au-sa",
    "AU-TAS": "foi-o-au-tas",
    "AU-VIC": "foi-o-au-vic",
    "AU-WA": "foi-o-au-wa",
}

CAPABILITY_IDS = frozenset(
    {
        "attachment_discovery_html",
        "attachment_discovery_json",
        "attachment_download",
        "bounded_retry",
        "content_addressed_handoff",
        "raw_warc_wacz_preservation",
        "request_html_download",
        "request_json_discovery",
    },
)


class CapabilityContractError(ValueError):
    """Raised when a capability contract is unknown, unpinned, or inconsistent."""


def _fail(message: str) -> Never:
    raise CapabilityContractError(message)


def _require_object(value: object, message: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail(message)
    return cast("dict[str, Any]", value)


def _has_exact_capabilities(value: object) -> bool:
    return (
        isinstance(value, list)
        and all(isinstance(item, str) for item in value)
        and len(value) == len(CAPABILITY_IDS)
        and frozenset(value) == CAPABILITY_IDS
    )


def sha256_file(path: Path) -> str:
    """Return the SHA-256 digest of a file's exact bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_sha256(value: object) -> str:
    """Return a deterministic SHA-256 digest for a JSON value."""
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def require_explicit_jurisdiction(jurisdiction_id: str | None) -> str:
    """Accept only a registered explicit jurisdiction identifier."""
    if jurisdiction_id not in JURISDICTION_TO_PROFILE:
        msg = f"unknown or absent Australian jurisdiction: {jurisdiction_id!r}"
        raise CapabilityContractError(msg)
    return jurisdiction_id


def _read_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        msg = f"unable to read capability contract {path}"
        raise CapabilityContractError(msg) from exc
    if not isinstance(value, dict):
        msg = f"capability contract must be an object: {path}"
        raise CapabilityContractError(msg)
    return value


def _validate_platform_identity_and_scope(platform: dict[str, Any]) -> None:
    if platform.get("schema_version") != PLATFORM_SCHEMA_VERSION:
        _fail("unsupported Australian platform contract version")
    if platform.get("platform_id") != "au-rtk":
        _fail("unknown Australian platform")

    capability_ids = platform.get("declared_capability_ids")
    if not _has_exact_capabilities(capability_ids):
        _fail("unknown or missing platform capability")

    scope = platform.get("bounded_url_scope")
    if not isinstance(scope, dict) or canonical_sha256(scope) != EXPECTED_SCOPE_SHA256:
        _fail("bounded URL scope pin mismatch")
    if platform.get("bounded_url_scope_sha256") != EXPECTED_SCOPE_SHA256:
        _fail("bounded URL scope declaration mismatch")


def _validate_platform_adapter(platform: dict[str, Any], *, root: Path) -> None:
    adapter = _require_object(
        platform.get("adapter_revision"),
        "adapter revision must be pinned",
    )
    module_path = adapter.get("module_path")
    if module_path != ADAPTER_RELATIVE_PATH.as_posix():
        _fail("unknown adapter module")
    if sha256_file(root / ADAPTER_RELATIVE_PATH) != adapter.get("module_sha256"):
        _fail("adapter module pin mismatch")


def _validate_platform_legacy_oracle(platform: dict[str, Any], *, root: Path) -> None:
    legacy = platform.get("historical_regression_oracle")
    if not isinstance(legacy, dict) or legacy.get("sha256") != EXPECTED_LEGACY_SHA256:
        _fail("historical regression oracle pin mismatch")
    if sha256_file(root / LEGACY_RELATIVE_PATH) != EXPECTED_LEGACY_SHA256:
        _fail("historical CTH/NSW artifact changed")


def validate_platform_contract(platform: dict[str, Any], *, root: Path) -> None:
    """Validate content pins and closed capability identifiers."""
    _validate_platform_identity_and_scope(platform)
    _validate_platform_adapter(platform, root=root)
    _validate_platform_legacy_oracle(platform, root=root)


def load_platform_contract(root: Path) -> dict[str, Any]:
    """Load and semantically validate the shared platform contract."""
    platform = _read_object(root / PLATFORM_RELATIVE_PATH)
    validate_platform_contract(platform, root=root)
    return platform


def record_relative_path(jurisdiction_id: str | None) -> Path:
    """Return the registered record path without inferring jurisdiction."""
    explicit_id = require_explicit_jurisdiction(jurisdiction_id)
    slug = explicit_id.lower()
    return Path(f"artifacts/foio/australian-capabilities/{slug}.contract-only.json")


def _validate_record_identity(record: dict[str, Any]) -> None:
    if record.get("schema_version") != RECORD_SCHEMA_VERSION:
        _fail("unsupported jurisdiction capability record version")
    jurisdiction_id = require_explicit_jurisdiction(record.get("jurisdiction_id"))
    if record.get("profile_id") != JURISDICTION_TO_PROFILE[jurisdiction_id]:
        _fail("jurisdiction/profile mismatch")
    if record.get("record_status") != "contract_only_disabled":
        _fail("synthetic jurisdiction record must remain disabled")


def _validate_record_platform_pins(
    record: dict[str, Any],
    *,
    platform: dict[str, Any],
    platform_sha256: str,
) -> None:
    platform_ref = _require_object(
        record.get("platform_contract"),
        "platform contract reference is required",
    )
    if platform_ref.get("sha256") != platform_sha256:
        _fail("platform contract pin mismatch")
    if platform_ref.get("schema_version") != platform.get("schema_version"):
        _fail("platform contract version mismatch")

    if record.get("adapter_revision") != platform.get("adapter_revision"):
        _fail("adapter revision differs from shared platform contract")
    scope_ref = record.get("bounded_url_scope")
    if not isinstance(scope_ref, dict) or scope_ref.get("scope_sha256") != platform.get(
        "bounded_url_scope_sha256",
    ):
        _fail("jurisdiction URL scope pin mismatch")


def _validate_record_state(record: dict[str, Any]) -> None:
    capability_ids = record.get("capability_ids")
    if not _has_exact_capabilities(capability_ids):
        _fail("unknown or missing jurisdiction capability")

    classification = record.get("jurisdiction_classification")
    if not isinstance(classification, dict) or classification.get("mode") != "explicit_id_only":
        _fail("jurisdiction classification must fail closed")
    activation = record.get("activation")
    if not isinstance(activation, dict) or activation.get("enabled") is not False:
        _fail("contract-only jurisdiction cannot be activated")


def validate_jurisdiction_record(
    record: dict[str, Any],
    *,
    platform: dict[str, Any],
    platform_sha256: str,
) -> None:
    """Validate a disabled jurisdiction record against its exact platform contract."""
    _validate_record_identity(record)
    _validate_record_platform_pins(
        record,
        platform=platform,
        platform_sha256=platform_sha256,
    )
    _validate_record_state(record)


def load_jurisdiction_record(root: Path, jurisdiction_id: str | None) -> dict[str, Any]:
    """Load one exact disabled jurisdiction record and all referenced pins."""
    platform = load_platform_contract(root)
    platform_sha256 = sha256_file(root / PLATFORM_RELATIVE_PATH)
    record = _read_object(root / record_relative_path(jurisdiction_id))
    validate_jurisdiction_record(
        record,
        platform=platform,
        platform_sha256=platform_sha256,
    )
    return record
