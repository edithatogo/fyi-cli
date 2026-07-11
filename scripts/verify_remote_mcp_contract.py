#!/usr/bin/env python3
"""Validate the versioned offline contract for remote MCP operations."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests/fixtures/remote_mcp/v1.json"
REQUIRED_READ_TOOLS = {
    "remote_health",
    "remote_version",
    "remote_search_requests",
    "remote_get_request",
    "remote_list_authorities",
}


def validate_contract(path: Path = FIXTURE) -> list[str]:
    errors: list[str] = []
    try:
        contract = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"fixture is not valid JSON: {error}"]
    if contract.get("schema_version") != 1:
        errors.append("unsupported or missing schema_version")
    tools = contract.get("tools", {})
    if set(tools.get("read_only", [])) != REQUIRED_READ_TOOLS:
        errors.append("read-only tool set does not match the v1 contract")
    if tools.get("write") != []:
        errors.append("write tools must remain absent from the read-only v1 contract")
    required = contract.get("required_input", {})
    for tool in REQUIRED_READ_TOOLS:
        if "instance_id" not in required.get(tool, []):
            errors.append(f"{tool} lacks required instance_id")
    envelope = contract.get("error_envelope", {})
    if envelope.get("jsonrpc_code") != -32003:
        errors.append("remote error code changed without a fixture version bump")
    if envelope.get("contains_sensitive_content") is not False:
        errors.append("remote error envelope allows sensitive content")
    invariants = contract.get("invariants", {})
    expected = {
        "default_remote_enabled": False,
        "read_does_not_enable_write": True,
        "wildcard_hosts_rejected": True,
        "credentials_rejected": True,
        "live_network_required": False,
    }
    for key, expected_value in expected.items():
        if invariants.get(key) is not expected_value:
            errors.append(f"security invariant {key} is not enforced")
    return errors


def main() -> int:
    errors = validate_contract()
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print(f"remote MCP contract valid: {FIXTURE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
