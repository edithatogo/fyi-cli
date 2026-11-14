"""Actual SDK contracts for bounded, private MCP dispatch telemetry."""

from __future__ import annotations

import json
import re
import tempfile
from copy import deepcopy
from pathlib import Path

import anyio
import pytest
from hypothesis import given, settings
from hypothesis import strategies as st
from jsonschema import Draft202012Validator
from mcp.shared.exceptions import McpError

from tests import test_mcp_client_interop
from tests.mcp_upstream_fixture import (
    PRIVATE_MARKER,
    SYNTHETIC_HEADER_VALUE,
    upstream_server,
    upstream_session,
)
from tests.test_mcp_upstream_contract import FIXTURE, LINUX_TLS, REMOTE_TOOLS

mcp_binary = test_mcp_client_interop.mcp_binary
STATUS_URI = "fyi://remote/status"
SCHEMA = Path(__file__).parent / "fixtures/remote_mcp/status-v1.schema.json"
CONTRACT = json.loads(FIXTURE.read_text(encoding="utf-8"))
CASES = list({case["tool"]: case for case in CONTRACT["cases"]}.values())
pytestmark = [pytest.mark.integration, pytest.mark.compatibility]


async def _status(session) -> dict:
    result = await session.read_resource(STATUS_URI)
    assert len(result.contents) == 1
    assert str(result.contents[0].uri) == STATUS_URI
    assert result.contents[0].mimeType == "application/json"
    status = json.loads(result.contents[0].text)
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(status)
    rows = status["metrics"]["operations"]
    assert {row["operation"] for row in rows} == REMOTE_TOOLS
    assert len(rows) == len(REMOTE_TOOLS)
    for row in rows:
        assert row["started"] == sum(row["outcomes"].values()) + row["in_flight"]
        assert row["duration_bucket_counts"] == sorted(row["duration_bucket_counts"])
        assert row["duration_bucket_counts"][-1] == sum(row["outcomes"].values())
    return status


def _row(status: dict, operation: str) -> dict:
    return next(row for row in status["metrics"]["operations"] if row["operation"] == operation)


def _events(directory: Path, operation: str, outcome: str) -> None:
    diagnostics = (directory / "upstream-server.stderr").read_text(encoding="utf-8")
    diagnostics = re.sub(r"\x1b\[[0-9;]*m", "", diagnostics)
    events = [line for line in diagnostics.splitlines() if "fyi_mcp::dispatch:" in line]
    assert len(events) == 2, diagnostics
    assert 'event="started"' in events[0]
    assert 'event="completed"' in events[1]
    assert f'outcome="{outcome}"' in events[1]
    if outcome != "success":
        assert "WARN" in events[1]
    correlations = []
    for event in events:
        assert "mcp_remote_dispatch" in event
        assert "schema_version=1" in event
        assert f'operation="{operation}"' in event
        correlation = re.findall(r'correlation_id="(mcp-\d+)"', event)
        assert correlation
        assert len(set(correlation)) == 1
        correlations.append(correlation[0])
    assert correlations[0] == correlations[1]
    for private in [
        PRIVATE_MARKER,
        SYNTHETIC_HEADER_VALUE,
        "mcp-fixture.invalid",
        "Synthetic submission body",
        "Synthetic correspondence",
        "Synthetic attachment",
        "confirmation_token",
        "idempotency_key",
        "instance_id",
        str(directory),
    ]:
        assert private not in diagnostics


@LINUX_TLS
@pytest.mark.e2e
@pytest.mark.parametrize("case", CASES, ids=lambda case: case["tool"])
def test_each_remote_tool_records_one_dispatch_not_preparation_or_replay(
    mcp_binary: Path,
    tmp_path: Path,
    case: dict,
) -> None:
    for name, contents in case.get("files", {}).items():
        assert Path(name).name == name
        (tmp_path / name).write_text(contents, encoding="utf-8")

    async def scenario() -> None:
        with upstream_server(tmp_path, case["exchanges"]) as upstream:
            writes = case.get("write", False)
            async with upstream_session(mcp_binary, tmp_path, upstream, writes=writes) as session:
                resources = await session.list_resources()
                assert any(str(resource.uri) == STATUS_URI for resource in resources.resources)
                before = await _status(session)
                assert before["policy"]["instances"][0]["requests_used"] == 0
                arguments = case["arguments"]
                if writes:
                    prepared = await session.call_tool(case["tool"], arguments)
                    assert not prepared.isError
                    assert await _status(session) == before
                    arguments = {
                        "instance_id": "synthetic",
                        "phase": "commit",
                        "idempotency_key": arguments["idempotency_key"],
                        "confirmation_token": prepared.structuredContent["confirmation_token"],
                    }
                    invalid = await session.call_tool(
                        case["tool"],
                        {**arguments, "confirmation_token": "synthetic-private-invalid"},
                    )
                    assert invalid.isError
                    assert await _status(session) == before
                result = await session.call_tool(case["tool"], arguments)
                assert not result.isError
                after = await _status(session)
                metric = _row(after, case["tool"])
                assert metric["started"] == 1
                assert metric["outcomes"]["success"] == 1
                assert metric["in_flight"] == 0
                encoded_result = json.dumps(
                    result.structuredContent,
                    ensure_ascii=False,
                    separators=(",", ":"),
                )
                expected_bytes = len(encoded_result.encode("utf-8"))
                assert metric["serialized_result_bytes"] == expected_bytes
                assert after["policy"]["instances"][0]["bytes_used"] == expected_bytes
                assert after["policy"]["instances"][0]["requests_used"] == 1
                assert sum(row["started"] for row in after["metrics"]["operations"]) == 1
                if writes:
                    cached = await session.call_tool(case["tool"], case["arguments"])
                    assert cached.structuredContent["status"] == "already_committed"
                    replay = await session.call_tool(case["tool"], arguments)
                    assert replay.isError
                    assert await _status(session) == after
                assert await _status(session) == after
            upstream.assert_complete()
            assert len(upstream.observed) == len(case["exchanges"])

    anyio.run(scenario)
    _events(tmp_path, case["tool"], "success")


@LINUX_TLS
@pytest.mark.security
def test_private_source_error_records_coarse_failure_without_status_guessing(
    mcp_binary: Path,
    tmp_path: Path,
) -> None:
    exchange = next(case for case in CONTRACT["errors"] if case["status"] == 403)

    async def scenario() -> None:
        with upstream_server(tmp_path, [exchange]) as upstream:
            async with upstream_session(mcp_binary, tmp_path, upstream) as session:
                result = await session.call_tool("remote_version", {"instance_id": "synthetic"})
                assert result.isError
                status = await _status(session)
                metric = _row(status, "remote_version")
                assert metric["outcomes"]["failed"] == 1
                assert metric["outcomes"]["timed_out"] == 0
                assert metric["serialized_result_bytes"] == 0
                assert PRIVATE_MARKER not in json.dumps(status)
            upstream.assert_complete()

    anyio.run(scenario)
    _events(tmp_path, "remote_version", "failed")


@LINUX_TLS
@pytest.mark.property
@pytest.mark.security
@settings(max_examples=12, derandomize=True, deadline=None)
@given(marker=st.text(min_size=1, max_size=80))
def test_generated_caller_values_cannot_create_metric_labels(mcp_binary: Path, marker: str) -> None:
    async def scenario(directory: Path) -> None:
        with upstream_server(directory, []) as upstream:
            async with upstream_session(mcp_binary, directory, upstream) as session:
                before = await _status(session)
                for arguments in [
                    {"instance_id": f"unconfigured-{marker}"},
                    {"instance_id": "synthetic", "unexpected": marker},
                ]:
                    code = None
                    try:
                        response = await session.call_tool("remote_version", arguments)
                    except McpError as error:
                        code = error.error.code
                    if code is None:
                        assert response.isError
                    else:
                        assert code == -32602
                assert await _status(session) == before
            upstream.assert_complete()
            assert upstream.observed == []

    with tempfile.TemporaryDirectory(prefix="mcp-metric-property-") as name:
        anyio.run(scenario, Path(name))


@LINUX_TLS
def test_metrics_reset_on_process_restart_and_schema_rejects_dynamic_labels(
    mcp_binary: Path,
    tmp_path: Path,
) -> None:
    exchange = next(case for case in CONTRACT["cases"] if case["tool"] == "remote_version")

    async def scenario() -> None:
        with upstream_server(tmp_path, exchange["exchanges"]) as upstream:
            async with upstream_session(mcp_binary, tmp_path, upstream) as session:
                await session.call_tool("remote_version", exchange["arguments"])
                assert _row(await _status(session), "remote_version")["started"] == 1
            async with upstream_session(mcp_binary, tmp_path, upstream) as restarted:
                empty = await _status(restarted)
                assert all(row["started"] == 0 for row in empty["metrics"]["operations"])
                mutated = deepcopy(empty)
                mutated["metrics"]["operations"][0]["instance_id"] = "synthetic-private"
                validator = Draft202012Validator(json.loads(SCHEMA.read_text(encoding="utf-8")))
                assert not validator.is_valid(mutated)
                mutated = deepcopy(empty)
                mutated["metrics"]["operations"][0]["operation"] = "caller-controlled-label"
                assert not validator.is_valid(mutated)
            upstream.assert_complete()

    anyio.run(scenario)
