"""Validate versioned MCP fixtures and compare a real tools/list result to its snapshot."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from jsonschema.exceptions import SchemaError
from referencing.exceptions import Unresolvable

ROOT = Path(__file__).resolve().parents[1]
FIXTURE_DIR = ROOT / "tests/fixtures/remote_mcp"
FIXTURE = FIXTURE_DIR / "v1.json"
CURRENT_FIXTURE = FIXTURE_DIR / "v2.json"
CURRENT_SCHEMA = FIXTURE_DIR / "v2.schema.json"
TOOLS_FIXTURE = FIXTURE_DIR / "tools-v1.json"
TOOLS_SCHEMA = FIXTURE_DIR / "tools-v1.schema.json"
REQUIRED_READ_TOOLS = {
    "remote_health",
    "remote_version",
    "remote_search_requests",
    "remote_get_request",
    "remote_list_authorities",
}
REQUIRED_WRITE_TOOLS = {
    "remote_create_request",
    "remote_add_correspondence",
    "remote_update_state",
}
REQUIRED_INPUT = {
    "remote_health": ["instance_id"],
    "remote_version": ["instance_id"],
    "remote_search_requests": ["instance_id", "query"],
    "remote_get_request": ["instance_id", "request_id"],
    "remote_list_authorities": ["instance_id"],
    "remote_create_request": ["instance_id", "idempotency_key"],
    "remote_add_correspondence": ["instance_id", "request_id", "idempotency_key"],
    "remote_update_state": ["instance_id", "request_id", "idempotency_key"],
}
ERROR_ENVELOPE = {
    "jsonrpc_code": -32003,
    "write_jsonrpc_code": -32004,
    "message": "remote operation failed; see operator audit telemetry",
    "contains_sensitive_content": False,
}
INVARIANTS = {
    "default_remote_enabled": False,
    "read_does_not_enable_write": True,
    "wildcard_hosts_rejected": True,
    "credentials_rejected": True,
    "live_network_required": False,
}
SUCCESS_CASE_TO_TOOL = {
    **{f"{tool}_success": tool for tool in REQUIRED_READ_TOOLS},
    **{
        f"{tool}_{phase}": tool
        for tool in REQUIRED_WRITE_TOOLS
        for phase in ("prepared", "replayed", "committed")
    },
}
TOOL_ERROR_CASES = {
    "remote_read_denied": (
        "remote_health",
        "remote read capability is disabled for instance",
        "handler",
    ),
    "remote_write_denied": (
        "remote_create_request",
        "remote write capability is disabled for instance",
        "handler",
    ),
    "remote_confirmation_rejected": (
        "remote_create_request",
        "confirmation_token is missing, expired, or already used",
        "handler",
    ),
    "remote_operation_failed": (
        "remote_version",
        "remote operation failed; see operator audit telemetry",
        "synthetic_output",
    ),
}
PROTOCOL_ERROR_CASES = {
    "malformed_json": (-32700, "Parse error"),
    "invalid_envelope": (-32600, "Invalid Request"),
    "invalid_tool_parameters": (-32602, "Tool arguments do not match the advertised schema"),
    "invalid_commit_parameters": (-32602, "Tool arguments do not match the advertised schema"),
    "unknown_tool": (-32602, "Unknown tool"),
}
INVALID_ARGUMENT_CASES = {"invalid_tool_parameters", "invalid_commit_parameters"}


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for name, value in pairs:
        if name in result:
            message = "duplicate JSON object key"
            raise ValueError(message)
        result[name] = value
    return result


def _reject_constant(_value: str) -> None:
    message = "non-finite JSON number"
    raise ValueError(message)


def load_fixture(path: Path) -> tuple[object, list[str]]:
    """Read strict JSON without printing paths, document fragments or decoder details."""
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (OSError, UnicodeError, ValueError, RecursionError):
        return None, ["fixture is missing or is not strict JSON"]
    return value, []


def _versioned_object(value: object, version: int = 1) -> bool:
    return (
        isinstance(value, dict)
        and isinstance(value.get("schema_version"), int)
        and not isinstance(value.get("schema_version"), bool)
        and value["schema_version"] == version
    )


def _exact_string_set(value: object, expected: set[str]) -> bool:
    return (
        isinstance(value, list)
        and all(isinstance(item, str) for item in value)
        and len(value) == len(expected)
        and set(value) == expected
    )


def _exact_mapping(value: dict, expected: dict) -> bool:
    return set(value) == set(expected) and all(
        type(value[key]) is type(item) and value[key] == item for key, item in expected.items()
    )


def _inventory_structure(contract: object) -> list[str]:
    if not _versioned_object(contract):
        return ["unsupported or missing schema_version"]
    sections = ("tools", "required_input", "error_envelope", "invariants")
    if set(contract) != {"schema_version", *sections}:
        return ["inventory sections do not match the v1 contract"]
    if any(not isinstance(contract.get(section), dict) for section in sections):
        return ["inventory sections must be objects"]
    return []


def validate_contract(path: Path = FIXTURE) -> list[str]:
    """Validate the historical remote-policy inventory without treating it as live evidence."""
    contract, errors = load_fixture(path)
    if errors:
        return errors
    errors = _inventory_structure(contract)
    if errors:
        return errors
    tools = contract["tools"]
    if set(tools) != {"read_only", "write"}:
        errors.append("tool groups do not match the v1 contract")
    if not _exact_string_set(tools.get("read_only"), REQUIRED_READ_TOOLS):
        errors.append("read-only tool set does not match the v1 contract")
    if not _exact_string_set(tools.get("write"), REQUIRED_WRITE_TOOLS):
        errors.append("governed write tool set does not match the v1 contract")
    required = contract["required_input"]
    if set(required) != set(REQUIRED_INPUT) or any(
        not _exact_string_set(required.get(tool), set(parameters))
        for tool, parameters in REQUIRED_INPUT.items()
    ):
        errors.append("required parameters do not match the v1 contract")
    if not _exact_mapping(contract["error_envelope"], ERROR_ENVELOPE):
        errors.append("error envelope changed without a fixture version bump")
    if not _exact_mapping(contract["invariants"], INVARIANTS):
        errors.append("security invariants do not match the v1 contract")
    return errors


def _has_external_reference(value: object) -> bool:
    if isinstance(value, dict):
        return any(
            (key in {"$ref", "$dynamicRef"} and isinstance(item, str) and not item.startswith("#"))
            or _has_external_reference(item)
            for key, item in value.items()
        )
    if isinstance(value, list):
        return any(_has_external_reference(item) for item in value)
    return False


def validate_tool_snapshot(snapshot: object) -> list[str]:
    """Check a complete catalogue fixture, including every nested input/output schema."""
    if not _versioned_object(snapshot):
        return ["unsupported or missing tool snapshot schema_version"]
    schema, errors = load_fixture(TOOLS_SCHEMA)
    if errors:
        return ["tool snapshot schema is missing or malformed"]
    try:
        Draft202012Validator.check_schema(schema)
        if not Draft202012Validator(schema).is_valid(snapshot):
            return ["tool snapshot does not satisfy the versioned catalogue schema"]
    except (SchemaError, RecursionError):
        return ["tool snapshot schema is invalid"]
    names: set[str] = set()
    for tool in snapshot["tools"]:
        if tool["name"] in names:
            errors.append("tool snapshot contains duplicate tool names")
        names.add(tool["name"])
        for key in ("inputSchema", "outputSchema"):
            if _has_external_reference(tool[key]):
                errors.append("tool schema has an external reference; fixtures must be offline")
                continue
            try:
                Draft202012Validator.check_schema(tool[key])
            except (SchemaError, RecursionError):
                errors.append("tool snapshot contains an invalid input or output JSON Schema")
    return errors


def compare_tool_catalogue(actual_tools: object, snapshot: object) -> list[str]:
    """Detect additions, removals and schema/annotation drift in an executable catalogue."""
    errors = validate_tool_snapshot(snapshot)
    if errors:
        return errors
    actual = {"schema_version": 1, "tools": actual_tools}
    errors = validate_tool_snapshot(actual)
    if errors:
        return ["live tool catalogue is malformed or has duplicate tool names"]
    expected_by_name = {tool["name"]: tool for tool in snapshot["tools"]}
    actual_by_name = {tool["name"]: tool for tool in actual["tools"]}
    if not _json_equal(expected_by_name, actual_by_name):
        return ["live tool catalogue differs from the versioned schema snapshot"]
    return []


def validate_tools_result(result: object, snapshot: object) -> list[str]:
    """Accept only a successful JSON-RPC tools/list response and compare every definition."""
    if (
        not isinstance(result, dict)
        or result.get("jsonrpc") != "2.0"
        or not isinstance(result.get("id"), (str, int))
        or isinstance(result.get("id"), bool)
        or "error" in result
        or not isinstance(result.get("result"), dict)
        or "nextCursor" in result["result"]
    ):
        return ["tools/list must be a complete successful JSON-RPC response"]
    return compare_tool_catalogue(result["result"].get("tools"), snapshot)


def _json_equal(left: object, right: object) -> bool:
    # Python equality treats True and 1 as equal; JSON contracts must distinguish them.
    return json.dumps(left, sort_keys=True) == json.dumps(right, sort_keys=True)


def _validate_current_shape(current: object) -> list[str]:
    if not _versioned_object(current, 2):
        return ["unsupported or missing current contract schema_version"]
    schema, errors = load_fixture(CURRENT_SCHEMA)
    if errors:
        return ["current exchange schema is missing or malformed"]
    try:
        Draft202012Validator.check_schema(schema)
        if not Draft202012Validator(schema).is_valid(current):
            return ["current exchanges do not satisfy the v2 schema"]
    except (SchemaError, RecursionError, Unresolvable):
        return ["current exchange schema is invalid"]
    names = [case["name"] for case in current["cases"]]
    expected = set(SUCCESS_CASE_TO_TOOL) | set(TOOL_ERROR_CASES) | set(PROTOCOL_ERROR_CASES)
    if len(names) != len(expected) or set(names) != expected:
        return ["current exchanges require every distinct versioned success and error case"]
    return []


def _validate_case_request(case: dict) -> list[str]:
    name = case["name"]
    if name == "malformed_json":
        try:
            json.loads(case.get("request_text", "null"))
        except (ValueError, RecursionError):
            return []
        return ["parse-error fixture must contain malformed JSON"]
    request = case.get("request", {})
    version = "1.0" if name == "invalid_envelope" else "2.0"
    if request.get("jsonrpc") != version or request.get("id") != name:
        return ["fixture request version or identifier differs from its contract"]
    expected_id = None if name == "invalid_envelope" else name
    if case["response"]["id"] != expected_id:
        return ["fixture response identifier does not match its request"]
    if name == "invalid_envelope":
        return []
    params = request.get("params")
    if (
        request.get("method") != "tools/call"
        or not isinstance(params, dict)
        or not isinstance(params.get("arguments"), dict)
        or not isinstance(params.get("name"), str)
    ):
        return ["tool fixture requires a structured tools/call request"]
    expected_tool = SUCCESS_CASE_TO_TOOL.get(name)
    if name in TOOL_ERROR_CASES:
        expected_tool = TOOL_ERROR_CASES[name][0]
    if expected_tool is not None and params["name"] != expected_tool:
        return ["fixture case does not exercise its specified tool"]
    return []


def _validate_case_response(case: dict) -> list[str]:
    name = case["name"]
    response = case["response"]
    if name in PROTOCOL_ERROR_CASES:
        code, message = PROTOCOL_ERROR_CASES[name]
        if (
            case["kind"] != "protocol_error"
            or case["verification"] != "handler"
            or response.get("error") != {"code": code, "message": message}
            or (name == "malformed_json" and response["id"] is not None)
        ):
            return ["protocol error fixture differs from the v2 contract"]
        return []
    if name in TOOL_ERROR_CASES:
        _, message, verification = TOOL_ERROR_CASES[name]
        if (
            case["kind"] != "tool_error"
            or case["verification"] != verification
            or response["result"]["content"][0]["text"] != message
        ):
            return ["runtime tool error fixture differs from the v2 contract"]
        return []
    if case["kind"] != "tool_success" or case["verification"] != "synthetic_output":
        return ["synthetic success outputs must not claim executed handler coverage"]
    result = response["result"]
    try:
        text_payload = json.loads(
            result["content"][0]["text"],
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (ValueError, RecursionError):
        return ["success text content is not strict JSON"]
    if not _json_equal(text_payload, result["structuredContent"]):
        return ["success text and structured content disagree"]
    return []


def validate_exchange_contract(current: object) -> list[str]:
    """Validate current synthetic examples without claiming their network paths executed."""
    errors = _validate_current_shape(current)
    if errors:
        return errors
    for case in current["cases"]:
        errors.extend(_validate_case_request(case))
        errors.extend(_validate_case_response(case))
    return errors


def validate_current_contract(path: Path = CURRENT_FIXTURE) -> list[str]:
    current, errors = load_fixture(path)
    return errors or validate_exchange_contract(current)


def _validate_case_schema(case: dict, by_name: dict) -> list[str]:
    if case["name"] in {"malformed_json", "invalid_envelope"}:
        return []
    request = case["request"]["params"]
    tool = by_name.get(request["name"])
    if case["name"] == "unknown_tool":
        return [] if tool is None else ["unknown-tool fixture names a registered tool"]
    if tool is None:
        return ["fixture tool is missing from the executable catalogue snapshot"]
    try:
        valid_input = Draft202012Validator(tool["inputSchema"]).is_valid(request["arguments"])
        if case["name"] in INVALID_ARGUMENT_CASES:
            return (
                [] if not valid_input else ["invalid-parameter fixture now passes input validation"]
            )
        if not valid_input:
            return ["success or runtime-error fixture has invalid tool arguments"]
        if case["kind"] == "tool_success" and not Draft202012Validator(
            tool["outputSchema"],
        ).is_valid(
            case["response"]["result"]["structuredContent"],
        ):
            return ["success fixture does not satisfy its executable output schema"]
    except (SchemaError, RecursionError, Unresolvable):
        return ["fixture tool schema could not be resolved offline"]
    return []


def validate_case_schemas(current: object, snapshot: object) -> list[str]:
    """Validate examples against captured schemas; negative parameters must remain invalid."""
    errors = validate_exchange_contract(current) + validate_tool_snapshot(snapshot)
    if errors:
        return errors
    by_name = {tool["name"]: tool for tool in snapshot["tools"]}
    for case in current["cases"]:
        # Names are checked against fixed versioned identifiers before appearing in diagnostics.
        errors.extend(f"{case['name']}: {error}" for error in _validate_case_schema(case, by_name))
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, default=TOOLS_FIXTURE)
    parser.add_argument("--tools-result", type=Path, help="Captured JSON-RPC tools/list response")
    arguments = parser.parse_args(argv)
    errors = validate_contract()
    current, current_errors = load_fixture(CURRENT_FIXTURE)
    errors.extend(current_errors or validate_exchange_contract(current))
    snapshot, snapshot_errors = load_fixture(arguments.snapshot)
    errors.extend(snapshot_errors or validate_tool_snapshot(snapshot))
    if not current_errors and not snapshot_errors:
        errors.extend(validate_case_schemas(current, snapshot))
    if arguments.tools_result:
        result, result_errors = load_fixture(arguments.tools_result)
        errors.extend(result_errors or validate_tools_result(result, snapshot))
    if errors:
        for error in errors:
            print(f"ERROR: {error}")  # noqa: T201 - command-line verification output
        return 1
    print("remote MCP v2 exchanges, historical v1 inventory and tool schemas valid")  # noqa: T201
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
