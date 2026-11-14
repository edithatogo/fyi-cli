"""Versioned synthetic upstream contracts exercised by the real Rust MCP server."""

from __future__ import annotations

import json
import socket
import sys
import time
from email import policy
from email.parser import BytesParser
from pathlib import Path
from typing import TYPE_CHECKING
from urllib.parse import urlencode

import anyio
import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

from tests import test_mcp_client_interop
from tests.mcp_upstream_fixture import (
    PRIVATE_MARKER,
    SYNTHETIC_HEADER_VALUE,
    upstream_server,
    upstream_session,
)

if TYPE_CHECKING:
    from mcp import ClientSession
    from mcp.types import CallToolResult

# Register the existing fixture; Cargo remains the single executable builder.
mcp_binary = test_mcp_client_interop.mcp_binary

FIXTURE = Path(__file__).parent / "fixtures/remote_mcp/upstream-v1.json"
REMOTE_TOOLS = {
    "remote_health",
    "remote_version",
    "remote_get_request",
    "remote_search_requests",
    "remote_list_authorities",
    "remote_create_request",
    "remote_add_correspondence",
    "remote_update_state",
}
LINUX_TLS = pytest.mark.skipif(
    sys.platform != "linux",
    reason="the child-only SSL_CERT_FILE trust fixture is currently qualified on Linux",
)


def _fixture() -> dict:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


def _diagnostics_are_private(directory: Path) -> None:
    diagnostics = (directory / "upstream-server.stderr").read_text(encoding="utf-8")
    for private in [
        PRIVATE_MARKER,
        SYNTHETIC_HEADER_VALUE,
        "Synthetic submission body",
        "Synthetic correspondence",
        "Synthetic attachment",
    ]:
        assert private not in diagnostics


@pytest.mark.compatibility
def test_versioned_upstream_fixture_covers_every_current_remote_tool() -> None:
    assert FIXTURE.is_file(), "versioned actual MCP-to-source exchange fixtures are missing"
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    assert fixture["schema_version"] == 1
    assert fixture["origin"] == "https://mcp-fixture.invalid"
    assert fixture["provenance"] == "synthetic; not live-platform certification"
    cases = fixture["cases"]
    assert {case["tool"] for case in cases} == REMOTE_TOOLS
    assert len({case["id"] for case in cases}) == len(cases)
    for case in cases:
        assert case["arguments"]["instance_id"] == "synthetic"
        assert case["exchanges"], "a source contract must observe real wire traffic"
        for exchange in case["exchanges"]:
            assert exchange["method"] in {"GET", "POST", "PUT"}
            assert exchange["path"].startswith("/")
            assert exchange["status"] in {200, 201, 302}
    errors = fixture["errors"]
    assert len({case["id"] for case in errors}) == len(errors)
    assert {401, 403, 404, 429, 500, 503} <= {case["status"] for case in errors}


@pytest.mark.security
def test_local_fixture_refuses_other_destinations_without_forwarding(tmp_path: Path) -> None:
    with upstream_server(tmp_path, []) as upstream:
        with socket.create_connection(("127.0.0.1", upstream.port), timeout=2) as client:
            client.sendall(b"CONNECT never-forward.invalid:443 HTTP/1.1\r\n\r\n")
            response = client.recv(1024)
        assert response.startswith(b"HTTP/1.1 403 Forbidden")
        assert upstream.observed == []
        assert upstream.errors == ["proxy rejected an unexpected destination"]


@LINUX_TLS
@pytest.mark.integration
@pytest.mark.e2e
@pytest.mark.compatibility
@pytest.mark.parametrize("case", _fixture()["cases"], ids=lambda case: case["id"])
def test_actual_sdk_to_source_success_contract(
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
                if writes:
                    prepared = await session.call_tool(case["tool"], case["arguments"])
                    assert not prepared.isError
                    assert prepared.structuredContent["status"] == "prepared"
                    assert upstream.observed == [], "prepare performed source I/O"
                    commit = {
                        "instance_id": "synthetic",
                        "phase": "commit",
                        "idempotency_key": case["arguments"]["idempotency_key"],
                        "confirmation_token": prepared.structuredContent["confirmation_token"],
                    }
                    invalid = await session.call_tool(
                        case["tool"],
                        {**commit, "confirmation_token": "synthetic-invalid-token"},
                    )
                    assert invalid.isError
                    assert upstream.observed == [], "invalid confirmation performed source I/O"
                    result = await session.call_tool(case["tool"], commit)
                else:
                    result = await session.call_tool(case["tool"], case["arguments"])
                assert not result.isError, result
                assert result.structuredContent == case["expected"]
                if writes:
                    cached = await session.call_tool(case["tool"], case["arguments"])
                    assert not cached.isError
                    assert cached.structuredContent["status"] == "already_committed"
                    assert cached.structuredContent["result"] == case["expected"]
                    replay = await session.call_tool(case["tool"], commit)
                    assert replay.isError
                await session.send_ping()
            upstream.assert_complete()
            assert len(upstream.observed) == len(case["exchanges"])
            for actual, expected in zip(upstream.observed, case["exchanges"], strict=True):
                assert actual.headers["host"] == "mcp-fixture.invalid"
                assert "fyi" in actual.headers["user-agent"].lower()
                assert actual.headers.get("x-fyi-bot-token") == (
                    SYNTHETIC_HEADER_VALUE if writes else None
                )
                if "request_json" in expected:
                    assert json.loads(actual.body) == expected["request_json"]
                elif "request_parts" in expected:
                    content_type = actual.headers["content-type"]
                    assert content_type.startswith("multipart/form-data;")
                    message = BytesParser(policy=policy.default).parsebytes(
                        f"Content-Type: {content_type}\r\n\r\n".encode() + actual.body,
                    )
                    parts = list(message.iter_parts())
                    values = {
                        part.get_param("name", header="content-disposition"): part.get_payload(
                            decode=True,
                        ).decode()
                        for part in parts
                    }
                    assert values == expected["request_parts"]
                    attachment = next(part for part in parts if part.get_filename())
                    assert attachment.get_filename() == "report.txt"
                    assert attachment.get_content_type() == "text/plain"
                else:
                    assert actual.body == b""

    anyio.run(scenario)
    _diagnostics_are_private(tmp_path)


@LINUX_TLS
@pytest.mark.integration
@pytest.mark.e2e
@pytest.mark.security
def test_shared_journal_allows_one_subprocess_write_and_blocks_restart_replay(
    mcp_binary: Path,
    tmp_path: Path,
) -> None:
    case = next(case for case in _fixture()["cases"] if case["id"] == "create")
    tokens: list[str] = []
    responses: list[str] = []
    replay_error = "a durable write attempt already exists; reconcile its outcome before retrying"

    async def scenario() -> None:
        with anyio.fail_after(20), upstream_server(tmp_path, case["exchanges"]) as upstream:
            # Both actual processes use the same private state directory. Start
            # them sequentially, then prepare both before either can dispatch.
            async with (
                upstream_session(mcp_binary, tmp_path, upstream, writes=True) as first,
                upstream_session(mcp_binary, tmp_path, upstream, writes=True) as second,
            ):
                sessions = (first, second)
                for session in sessions:
                    prepared = await session.call_tool(case["tool"], case["arguments"])
                    assert not prepared.isError
                    assert prepared.structuredContent["status"] == "prepared"
                    tokens.append(prepared.structuredContent["confirmation_token"])
                assert len(set(tokens)) == 2
                assert upstream.observed == [], "prepare performed source I/O"

                start = anyio.Event()
                results: list[CallToolResult] = []

                async def commit(session: ClientSession, token: str) -> None:
                    await start.wait()
                    results.append(
                        await session.call_tool(
                            case["tool"],
                            {
                                "instance_id": "synthetic",
                                "phase": "commit",
                                "idempotency_key": case["arguments"]["idempotency_key"],
                                "confirmation_token": token,
                            },
                        ),
                    )

                async with anyio.create_task_group() as group:
                    for session, token in zip(sessions, tokens, strict=True):
                        group.start_soon(commit, session, token)
                    start.set()
                succeeded = [result for result in results if not result.isError]
                rejected = [result for result in results if result.isError]
                assert len(succeeded) == len(rejected) == 1
                assert succeeded[0].structuredContent == case["expected"]
                assert rejected[0].content[0].text == replay_error
                responses.extend(result.model_dump_json() for result in results)
                for session in sessions:
                    await session.send_ping()

            # A successful result is process-local; a new process must retain
            # the reservation and require reconciliation without redispatch.
            async with upstream_session(mcp_binary, tmp_path, upstream, writes=True) as restarted:
                replay = await restarted.call_tool(case["tool"], case["arguments"])
                assert replay.isError
                assert replay.content[0].text == replay_error
                responses.append(replay.model_dump_json())
                await restarted.send_ping()
            upstream.assert_complete()
            assert len(upstream.observed) == 1, "competing processes dispatched duplicate writes"
            request = upstream.observed[0]
            assert request.headers["x-fyi-bot-token"] == SYNTHETIC_HEADER_VALUE
            assert json.loads(request.body) == case["exchanges"][0]["request_json"]

    anyio.run(scenario)
    _diagnostics_are_private(tmp_path)
    private_values = [
        SYNTHETIC_HEADER_VALUE,
        case["arguments"]["body"],
        case["arguments"]["idempotency_key"],
        *tokens,
    ]
    artifacts = [tmp_path / "upstream-server.stderr", *(tmp_path / "write-state").iterdir()]
    for artifact in artifacts:
        contents = artifact.read_bytes()
        for private in private_values:
            assert private.encode() not in contents
    for response in responses:
        for private in private_values:
            assert private not in response


@LINUX_TLS
@pytest.mark.integration
@pytest.mark.security
@pytest.mark.parametrize("exchange", _fixture()["errors"], ids=lambda exchange: exchange["id"])
def test_actual_source_failure_is_private_and_never_retried(
    mcp_binary: Path,
    tmp_path: Path,
    exchange: dict,
) -> None:
    async def scenario() -> None:
        with upstream_server(tmp_path, [exchange]) as upstream:
            async with upstream_session(mcp_binary, tmp_path, upstream) as session:
                result = await session.call_tool("remote_version", {"instance_id": "synthetic"})
                assert result.isError
                assert result.content[0].text == (
                    "remote operation failed; see operator audit telemetry"
                )
                assert PRIVATE_MARKER not in result.model_dump_json()
                await session.send_ping()
            upstream.assert_complete()
            assert len(upstream.observed) == 1, "failure silently retried a source request"

    anyio.run(scenario)
    _diagnostics_are_private(tmp_path)


@LINUX_TLS
@pytest.mark.integration
@pytest.mark.slow
@pytest.mark.parametrize("fault", ["stalled_headers", "stalled_body"])
def test_production_timeout_closes_a_stalled_source_connection(
    mcp_binary: Path,
    tmp_path: Path,
    fault: str,
) -> None:
    exchange = {
        "method": "GET",
        "path": "/api/v2/version.json",
        "status": 200,
        "json": {"version": "unexpected-fast-response"},
        "fault": fault,
    }

    async def scenario() -> None:
        with upstream_server(tmp_path, [exchange]) as upstream:
            async with upstream_session(mcp_binary, tmp_path, upstream) as session:
                started = time.monotonic()
                result = await session.call_tool("remote_version", {"instance_id": "synthetic"})
                elapsed = time.monotonic() - started
                assert result.isError, "the source fixture did not enforce its requested stall"
                assert result.content[0].text == (
                    "remote operation failed; see operator audit telemetry"
                )
                # Source client timeout is 60 seconds. A five-second fixture/SDK
                # timeout cannot be mistaken for production cancellation.
                assert 50 <= elapsed < 70
                assert upstream.closed_after_stall.wait(timeout=2)
                await session.send_ping()
            upstream.assert_complete()
            assert len(upstream.observed) == 1

    anyio.run(scenario)
    _diagnostics_are_private(tmp_path)


@LINUX_TLS
@pytest.mark.integration
@pytest.mark.security
def test_stale_preflight_and_read_only_write_denial_never_mutate_upstream(
    mcp_binary: Path,
    tmp_path: Path,
) -> None:
    case = next(case for case in _fixture()["cases"] if case["id"] == "state_preflight")
    exchange = case["exchanges"][0]
    exchange["json"]["status"] = "rejected"

    async def scenario() -> None:
        with upstream_server(tmp_path, [exchange]) as upstream:
            async with upstream_session(mcp_binary, tmp_path, upstream) as session:
                denied = await session.call_tool(case["tool"], case["arguments"])
                assert denied.isError
                assert upstream.observed == []
            async with upstream_session(mcp_binary, tmp_path, upstream, writes=True) as session:
                prepared = await session.call_tool(case["tool"], case["arguments"])
                assert not prepared.isError
                result = await session.call_tool(
                    case["tool"],
                    {
                        "instance_id": "synthetic",
                        "phase": "commit",
                        "confirmation_token": prepared.structuredContent["confirmation_token"],
                        "idempotency_key": case["arguments"]["idempotency_key"],
                    },
                )
                assert result.isError
            upstream.assert_complete()
            assert [(request.method, request.path) for request in upstream.observed] == [
                ("GET", "/api/v2/request/7001.json"),
            ]

    anyio.run(scenario)
    _diagnostics_are_private(tmp_path)


@LINUX_TLS
@pytest.mark.integration
@pytest.mark.property
@settings(max_examples=12, deadline=None, derandomize=True)
@given(
    query=st.text(alphabet="Café東京 x&?/#%+", min_size=1, max_size=24).filter(str.strip),
    limit=st.integers(min_value=1, max_value=50),
)
def test_generated_query_encoding_and_result_limit_match_the_wire(
    mcp_binary: Path,
    tmp_path_factory: pytest.TempPathFactory,
    query: str,
    limit: int,
) -> None:
    directory = tmp_path_factory.mktemp("mcp-upstream-query")
    requests = [
        {"id": index + 1, "title": "Synthetic result", "body": "Synthetic body"}
        for index in range(limit + 2)
    ]
    exchange = {
        "method": "GET",
        "path": "/search.json?" + urlencode({"q": query.strip(), "per_page": limit}),
        "status": 200,
        "json": {"results": requests},
    }

    async def scenario() -> None:
        with upstream_server(directory, [exchange]) as upstream:
            async with upstream_session(mcp_binary, directory, upstream) as session:
                result = await session.call_tool(
                    "remote_search_requests",
                    {
                        "instance_id": "synthetic",
                        "query": query,
                        "limit": limit,
                    },
                )
                assert not result.isError
                assert result.structuredContent["query"] == query.strip()
                assert result.structuredContent["requests"] == requests[:limit]
            upstream.assert_complete()
            assert len(upstream.observed) == 1

    anyio.run(scenario)
