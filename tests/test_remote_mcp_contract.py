"""Offline contract drift and malformed fixture regressions for the MCP surface."""

from __future__ import annotations

import copy
import json
from typing import TYPE_CHECKING

import pytest

from scripts import verify_remote_mcp_contract as contract

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def inventory() -> dict:
    return json.loads(contract.FIXTURE.read_text(encoding="utf-8"))


@pytest.fixture
def snapshot() -> dict:
    return {
        "schema_version": 1,
        "tools": [
            {
                "name": "synthetic_echo",
                "description": "Return synthetic fixture text.",
                "inputSchema": {
                    "type": "object",
                    "properties": {"text": {"type": "string", "maxLength": 20}},
                    "required": ["text"],
                    "additionalProperties": False,
                },
                "outputSchema": {
                    "type": "object",
                    "properties": {"text": {"type": "string"}},
                    "required": ["text"],
                    "additionalProperties": False,
                },
                "annotations": {
                    "readOnlyHint": True,
                    "destructiveHint": False,
                    "idempotentHint": True,
                    "openWorldHint": False,
                },
            },
        ],
    }


def write_json(tmp_path: Path, value: object) -> Path:
    path = tmp_path / "contract.json"
    path.write_text(json.dumps(value), encoding="utf-8")
    return path


def test_remote_mcp_v1_contract_is_valid() -> None:
    assert contract.validate_contract() == []


@pytest.mark.parametrize("value", [None, [], "invalid", 1, True])
def test_inventory_rejects_non_object_roots(tmp_path: Path, value: object) -> None:
    assert contract.validate_contract(write_json(tmp_path, value))


@pytest.mark.parametrize("value", [None, [], "invalid", 1, True])
@pytest.mark.parametrize("field", ["tools", "required_input", "error_envelope", "invariants"])
def test_inventory_rejects_malformed_sections(
    tmp_path: Path,
    inventory: dict,
    field: str,
    value: object,
) -> None:
    inventory[field] = value
    assert contract.validate_contract(write_json(tmp_path, inventory))


@pytest.mark.parametrize("version", [None, 0, 2, True, "1", 1.5])
def test_inventory_rejects_unknown_or_ambiguous_version(
    tmp_path: Path,
    inventory: dict,
    version: object,
) -> None:
    inventory["schema_version"] = version
    assert contract.validate_contract(write_json(tmp_path, inventory))


@pytest.mark.parametrize("section", ["read_only", "write"])
def test_inventory_rejects_duplicate_tools(
    tmp_path: Path,
    inventory: dict,
    section: str,
) -> None:
    inventory["tools"][section].append(inventory["tools"][section][0])
    assert contract.validate_contract(write_json(tmp_path, inventory))


@pytest.mark.parametrize("value", [None, "instance_id", [{"instance_id": True}], [True]])
def test_inventory_rejects_invalid_required_parameters(
    tmp_path: Path,
    inventory: dict,
    value: object,
) -> None:
    inventory["required_input"]["remote_health"] = value
    assert contract.validate_contract(write_json(tmp_path, inventory))


def test_inventory_rejects_duplicate_required_parameters(tmp_path: Path, inventory: dict) -> None:
    inventory["required_input"]["remote_health"].append("instance_id")
    assert contract.validate_contract(write_json(tmp_path, inventory))


def test_inventory_checks_write_error_code(tmp_path: Path, inventory: dict) -> None:
    inventory["error_envelope"]["write_jsonrpc_code"] = -32603
    assert contract.validate_contract(write_json(tmp_path, inventory))


def test_inventory_checks_error_message(tmp_path: Path, inventory: dict) -> None:
    inventory["error_envelope"]["message"] = "synthetic-sensitive-value"
    errors = contract.validate_contract(write_json(tmp_path, inventory))
    assert errors
    assert "synthetic-sensitive-value" not in str(errors)


def test_inventory_requires_actual_boolean_privacy_flag(tmp_path: Path, inventory: dict) -> None:
    inventory["error_envelope"]["contains_sensitive_content"] = 0
    assert contract.validate_contract(write_json(tmp_path, inventory))


def test_inventory_rejects_duplicate_json_keys(tmp_path: Path) -> None:
    path = tmp_path / "duplicate.json"
    text = contract.FIXTURE.read_text(encoding="utf-8")
    path.write_text(text.replace('"schema_version": 1', '"schema_version": 0, "schema_version": 1'))
    assert contract.validate_contract(path)


@pytest.mark.parametrize("contents", ["{", '{"secret": NaN}', '{"secret": Infinity}'])
def test_invalid_json_fails_closed_without_contents(tmp_path: Path, contents: str) -> None:
    path = tmp_path / "invalid.json"
    path.write_text(contents, encoding="utf-8")
    errors = contract.validate_contract(path)
    assert errors
    assert contents not in str(errors)


def test_missing_fixture_fails_closed_without_path(tmp_path: Path) -> None:
    errors = contract.validate_contract(tmp_path / "synthetic-sensitive-path")
    assert errors
    assert "synthetic-sensitive-path" not in str(errors)


def test_tool_snapshot_and_matching_live_catalogue_are_valid(snapshot: dict) -> None:
    assert contract.validate_tool_snapshot(snapshot) == []
    assert contract.compare_tool_catalogue(copy.deepcopy(snapshot["tools"]), snapshot) == []


@pytest.mark.parametrize("value", [None, [], "invalid", 1, True])
def test_tool_snapshot_rejects_non_object_roots(value: object) -> None:
    assert contract.validate_tool_snapshot(value)


@pytest.mark.parametrize("version", [None, 0, 2, True, "1", 1.5])
def test_tool_snapshot_rejects_unknown_or_ambiguous_version(
    snapshot: dict,
    version: object,
) -> None:
    snapshot["schema_version"] = version
    assert contract.validate_tool_snapshot(snapshot)


def test_tool_snapshot_rejects_duplicate_names(snapshot: dict) -> None:
    snapshot["tools"].append(copy.deepcopy(snapshot["tools"][0]))
    assert contract.validate_tool_snapshot(snapshot)


def test_tool_snapshot_rejects_invalid_nested_json_schema(snapshot: dict) -> None:
    snapshot["tools"][0]["inputSchema"]["properties"]["text"]["maxLength"] = "invalid"
    assert contract.validate_tool_snapshot(snapshot)


def test_tool_snapshot_rejects_schema_network_references(snapshot: dict) -> None:
    snapshot["tools"][0]["inputSchema"]["properties"]["text"] = {
        "$ref": "https://synthetic.invalid/schema.json",
    }
    assert contract.validate_tool_snapshot(snapshot)


@pytest.mark.parametrize("field", ["inputSchema", "outputSchema", "annotations"])
def test_tool_snapshot_rejects_missing_contract_fields(snapshot: dict, field: str) -> None:
    del snapshot["tools"][0][field]
    assert contract.validate_tool_snapshot(snapshot)


@pytest.mark.parametrize(
    "hint",
    ["readOnlyHint", "destructiveHint", "idempotentHint", "openWorldHint"],
)
def test_tool_snapshot_requires_boolean_hints(snapshot: dict, hint: str) -> None:
    snapshot["tools"][0]["annotations"][hint] = "false"
    assert contract.validate_tool_snapshot(snapshot)


def test_catalogue_ignores_tool_order(snapshot: dict) -> None:
    second = copy.deepcopy(snapshot["tools"][0])
    second["name"] = "synthetic_other"
    snapshot["tools"].append(second)
    assert contract.compare_tool_catalogue(list(reversed(snapshot["tools"])), snapshot) == []


@pytest.mark.parametrize("change", ["add", "remove", "duplicate", "input", "output", "hint"])
def test_catalogue_rejects_live_contract_drift(snapshot: dict, change: str) -> None:
    actual = copy.deepcopy(snapshot["tools"])
    if change == "add":
        extra = copy.deepcopy(actual[0])
        extra["name"] = "synthetic_extra"
        actual.append(extra)
    elif change == "remove":
        actual.clear()
    elif change == "duplicate":
        actual.append(copy.deepcopy(actual[0]))
    elif change == "input":
        actual[0]["inputSchema"]["properties"]["text"]["maxLength"] = 21
    elif change == "output":
        actual[0]["outputSchema"]["required"] = []
    else:
        actual[0]["annotations"]["readOnlyHint"] = False
    assert contract.compare_tool_catalogue(actual, snapshot)


@pytest.mark.parametrize("value", [None, {}, "invalid", True, [None], ["invalid"]])
def test_catalogue_rejects_malformed_live_tools(snapshot: dict, value: object) -> None:
    assert contract.compare_tool_catalogue(value, snapshot)


def test_catalogue_distinguishes_boolean_and_numeric_schema_constants(snapshot: dict) -> None:
    snapshot["tools"][0]["inputSchema"]["properties"]["text"] = {"enum": [True]}
    actual = copy.deepcopy(snapshot["tools"])
    actual[0]["inputSchema"]["properties"]["text"] = {"enum": [1]}
    assert contract.compare_tool_catalogue(actual, snapshot)


def test_snapshot_validation_never_echoes_untrusted_data(snapshot: dict) -> None:
    snapshot["tools"][0]["name"] = "synthetic-sensitive-value/invalid"
    errors = contract.validate_tool_snapshot(snapshot)
    assert errors
    assert "synthetic-sensitive-value" not in str(errors)


def test_tools_list_response_contract(snapshot: dict) -> None:
    response = {"jsonrpc": "2.0", "id": "catalogue", "result": {"tools": snapshot["tools"]}}
    assert contract.validate_tools_result(response, snapshot) == []


@pytest.mark.parametrize("request_id", [None, True, [], {}, 1.5])
def test_tools_list_rejects_invalid_response_ids(snapshot: dict, request_id: object) -> None:
    response = {"jsonrpc": "2.0", "id": request_id, "result": {"tools": snapshot["tools"]}}
    assert contract.validate_tools_result(response, snapshot)


@pytest.mark.parametrize(
    "response",
    [
        None,
        [],
        {"jsonrpc": "1.0", "id": 1, "result": {}},
        {"jsonrpc": "2.0", "id": 1, "result": None},
        {"jsonrpc": "2.0", "id": 1, "result": {}, "error": None},
        {"jsonrpc": "2.0", "id": 1, "result": {"tools": [], "nextCursor": "page-2"}},
    ],
)
def test_tools_list_rejects_malformed_error_or_incomplete_response(
    snapshot: dict,
    response: object,
) -> None:
    assert contract.validate_tools_result(response, snapshot)


def test_command_compares_captured_live_tools_result(tmp_path: Path) -> None:
    snapshot, errors = contract.load_fixture(contract.TOOLS_FIXTURE)
    assert errors == []
    snapshot_path = write_json(tmp_path, snapshot)
    response_path = tmp_path / "tools-list-response.json"
    response_path.write_text(
        json.dumps({"jsonrpc": "2.0", "id": 1, "result": {"tools": snapshot["tools"]}}),
        encoding="utf-8",
    )
    args = ["--snapshot", str(snapshot_path), "--tools-result", str(response_path)]
    assert contract.main(args) == 0
    snapshot["tools"][0]["annotations"]["readOnlyHint"] = False
    write_json(tmp_path, snapshot)
    assert contract.main(args) == 1


def test_command_fails_closed_when_snapshot_is_missing(tmp_path: Path) -> None:
    assert contract.main(["--snapshot", str(tmp_path / "missing.json")]) == 1


@pytest.fixture
def exchanges() -> dict:
    return json.loads((contract.FIXTURE_DIR / "v2.json").read_text(encoding="utf-8"))


def test_current_exchange_contract_is_valid() -> None:
    assert contract.validate_current_contract() == []


def test_current_examples_match_executable_tool_schema_snapshot(exchanges: dict) -> None:
    snapshot, errors = contract.load_fixture(contract.TOOLS_FIXTURE)
    assert errors == []
    assert contract.validate_case_schemas(exchanges, snapshot) == []


@pytest.mark.parametrize("value", [None, [], "invalid", 1, True])
def test_current_contract_rejects_non_object_roots(tmp_path: Path, value: object) -> None:
    assert contract.validate_current_contract(write_json(tmp_path, value))


@pytest.mark.parametrize("version", [None, 1, 3, True, "2", 2.0])
def test_current_contract_rejects_unsupported_versions(
    tmp_path: Path,
    exchanges: dict,
    version: object,
) -> None:
    exchanges["schema_version"] = version
    assert contract.validate_current_contract(write_json(tmp_path, exchanges))


@pytest.mark.parametrize("change", ["missing", "duplicate", "unknown"])
def test_current_contract_requires_complete_distinct_cases(
    tmp_path: Path,
    exchanges: dict,
    change: str,
) -> None:
    if change == "missing":
        exchanges["cases"].pop()
    elif change == "duplicate":
        exchanges["cases"].append(copy.deepcopy(exchanges["cases"][0]))
    else:
        exchanges["cases"][0]["name"] = "unversioned_case"
    assert contract.validate_current_contract(write_json(tmp_path, exchanges))


@pytest.mark.parametrize("error_flag", [None, False, 1, "true"])
def test_runtime_tool_failure_requires_boolean_is_error(
    tmp_path: Path,
    exchanges: dict,
    error_flag: object,
) -> None:
    case = next(case for case in exchanges["cases"] if case["kind"] == "tool_error")
    case["response"]["result"]["isError"] = error_flag
    assert contract.validate_current_contract(write_json(tmp_path, exchanges))


def test_legacy_rpc_tool_error_is_rejected_in_current_contract(
    tmp_path: Path,
    exchanges: dict,
) -> None:
    case = next(case for case in exchanges["cases"] if case["kind"] == "tool_error")
    case["response"].pop("result")
    case["response"]["error"] = {"code": -32003, "message": "legacy tool failure"}
    assert contract.validate_current_contract(write_json(tmp_path, exchanges))


def test_protocol_error_cannot_be_relabelled_as_tool_failure(
    tmp_path: Path,
    exchanges: dict,
) -> None:
    case = next(case for case in exchanges["cases"] if case["name"] == "invalid_tool_parameters")
    case["response"].pop("error")
    case["response"]["result"] = {
        "isError": True,
        "content": [{"type": "text", "text": "invalid parameters"}],
    }
    assert contract.validate_current_contract(write_json(tmp_path, exchanges))


@pytest.mark.parametrize("text", ['{"changed":true}', "not JSON", '{"a":1,"a":2}'])
def test_success_content_must_match_structured_content(
    tmp_path: Path,
    exchanges: dict,
    text: str,
) -> None:
    exchanges["cases"][0]["response"]["result"]["content"][0]["text"] = text
    assert contract.validate_current_contract(write_json(tmp_path, exchanges))


def test_current_error_messages_cannot_drift_or_echo_private_input(
    tmp_path: Path,
    exchanges: dict,
) -> None:
    case = next(case for case in exchanges["cases"] if case["kind"] == "tool_error")
    case["response"]["result"]["content"][0]["text"] = "synthetic-sensitive-value"
    errors = contract.validate_current_contract(write_json(tmp_path, exchanges))
    assert errors
    assert "synthetic-sensitive-value" not in str(errors)


def test_current_success_response_id_must_match_request(tmp_path: Path, exchanges: dict) -> None:
    exchanges["cases"][0]["response"]["id"] = "different-request"
    assert contract.validate_current_contract(write_json(tmp_path, exchanges))


def test_current_contract_cannot_claim_synthetic_outputs_were_executed(
    tmp_path: Path,
    exchanges: dict,
) -> None:
    exchanges["cases"][0]["verification"] = "handler"
    assert contract.validate_current_contract(write_json(tmp_path, exchanges))


def test_current_contract_rejects_duplicate_json_keys(tmp_path: Path) -> None:
    path = tmp_path / "duplicate-current.json"
    text = (contract.FIXTURE_DIR / "v2.json").read_text(encoding="utf-8")
    path.write_text(
        text.replace('"schema_version": 2', '"schema_version": 1, "schema_version": 2'),
        encoding="utf-8",
    )
    assert contract.validate_current_contract(path)


def test_current_success_is_checked_against_output_schema(exchanges: dict) -> None:
    snapshot, errors = contract.load_fixture(contract.TOOLS_FIXTURE)
    assert errors == []
    case = exchanges["cases"][0]
    payload = case["response"]["result"]["structuredContent"]
    payload["network_reachable"] = "yes"
    case["response"]["result"]["content"][0]["text"] = json.dumps(payload)
    assert contract.validate_case_schemas(exchanges, snapshot)


def test_current_invalid_parameters_must_remain_invalid_under_schema(exchanges: dict) -> None:
    snapshot, errors = contract.load_fixture(contract.TOOLS_FIXTURE)
    assert errors == []
    case = next(case for case in exchanges["cases"] if case["name"] == "invalid_tool_parameters")
    case["request"]["params"]["arguments"]["request_id"] = 41
    assert contract.validate_case_schemas(exchanges, snapshot)
