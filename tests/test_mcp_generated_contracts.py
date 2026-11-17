"""Generated offline MCP-to-source invariants using the existing TLS fixture.

The fixture never forwards connections. These are Linux synthetic contracts,
not live-platform certification, atomic upstream updates, or filesystem race
qualification. Every example uses a new process, database, journal and origin.
"""

from __future__ import annotations

import json
import sys
from email import policy
from email.parser import BytesParser
from typing import TYPE_CHECKING
from urllib.parse import urlencode

import anyio
import pytest
from hypothesis import example, given, settings
from hypothesis import strategies as st

from tests import test_mcp_client_interop
from tests.mcp_upstream_fixture import (
    SYNTHETIC_HEADER_VALUE,
    upstream_server,
    upstream_session,
)

if TYPE_CHECKING:
    from pathlib import Path

mcp_binary = test_mcp_client_interop.mcp_binary
pytestmark = [
    pytest.mark.property,
    pytest.mark.integration,
    pytest.mark.security,
    pytest.mark.skipif(
        sys.platform != "linux",
        reason="the child-only TLS source fixture is qualified on Linux",
    ),
]

PRIVATE = "synthetic-generated-private-"
STATES = ("waiting_response", "successful", "rejected", "partially_successful")


def _commit(prepared: dict, key: str) -> dict:
    return {
        "instance_id": "synthetic",
        "phase": "commit",
        "confirmation_token": prepared["confirmation_token"],
        "idempotency_key": key,
    }


def _assert_private_diagnostics(directory: Path, *private_values: str) -> None:
    diagnostics = (directory / "upstream-server.stderr").read_text(encoding="utf-8")
    for value in (SYNTHETIC_HEADER_VALUE, *private_values):
        assert value not in diagnostics, "diagnostics disclosed synthetic private input"


def _assert_omits(value: object, marker: str) -> None:
    """Inspect decoded strings so JSON escaping cannot conceal a disclosure."""
    if isinstance(value, dict):
        for key, item in value.items():
            _assert_omits(key, marker)
            _assert_omits(item, marker)
    elif isinstance(value, list):
        for item in value:
            _assert_omits(item, marker)
    elif isinstance(value, str):
        assert marker not in value, "result disclosed synthetic private input"


@settings(max_examples=16, deadline=None, derandomize=True)
@example(query=" Café 東京 &?/%+ ", limit=1, shape="results")
@example(query="next", limit=50, shape="requests")
@given(
    query=st.text(alphabet="Café東京 x&?/#%+", min_size=1, max_size=36).filter(str.strip),
    limit=st.integers(min_value=1, max_value=50),
    shape=st.sampled_from(("array", "requests", "results")),
)
def test_generated_search_variants_preserve_order_and_bound_pagination(
    mcp_binary: Path,
    tmp_path_factory: pytest.TempPathFactory,
    query: str,
    limit: int,
    shape: str,
) -> None:
    """One requested page cannot follow an untrusted next URL or overpublish."""
    directory = tmp_path_factory.mktemp("mcp-generated-page")
    records = [
        {"id": index + 1, "title": f"Synthetic {index}", "body": "Synthetic Café 東京"}
        for index in range(limit + 3)
    ]
    payload = (
        records
        if shape == "array"
        else {shape: records, "next": "https://never-forward.invalid/private-next-page"}
    )
    exchange = {
        "method": "GET",
        "path": "/search.json?" + urlencode({"q": query.strip(), "per_page": limit}),
        "status": 200,
        "json": payload,
    }

    async def scenario() -> None:
        with upstream_server(directory, [exchange]) as upstream:
            async with upstream_session(mcp_binary, directory, upstream) as session:
                result = await session.call_tool(
                    "remote_search_requests",
                    {"instance_id": "synthetic", "query": query, "limit": limit},
                )
                assert not result.isError
                assert result.structuredContent == {
                    "instance_id": "synthetic",
                    "query": query.strip(),
                    "requests": records[:limit],
                }
                assert (await session.call_tool("list_requests", {})).structuredContent == {
                    "requests": [],
                }
                await session.send_ping()
            upstream.assert_complete()
            assert len(upstream.observed) == 1

    anyio.run(scenario)
    _assert_private_diagnostics(directory)


@settings(max_examples=16, deadline=None, derandomize=True)
@example(expected="waiting_response", matches=True, target="successful")
@example(expected="successful", matches=False, target="waiting_response")
@given(
    expected=st.sampled_from(STATES),
    matches=st.booleans(),
    target=st.sampled_from(STATES),
)
def test_generated_state_preconditions_bind_confirmation_and_prevent_replay(
    mcp_binary: Path,
    tmp_path_factory: pytest.TempPathFactory,
    expected: str,
    *,
    matches: bool,
    target: str,
) -> None:
    """A stale preflight performs GET only; either outcome consumes the token."""
    directory = tmp_path_factory.mktemp("mcp-generated-state")
    key = PRIVATE + expected
    actual = expected if matches else PRIVATE + "different-state"
    arguments = {
        "instance_id": "synthetic",
        "request_id": 7001,
        "expected_current_state": expected,
        "state": target,
        "idempotency_key": key,
    }
    exchanges = [
        {
            "method": "GET",
            "path": "/api/v2/request/7001.json",
            "status": 200,
            "json": {"id": 7001, "title": "Synthetic", "body": "Synthetic", "status": actual},
        },
    ]
    if matches:
        exchanges.append(
            {
                "method": "PUT",
                "path": "/api/v2/request/7001/state.json",
                "status": 200,
                "json": {"updated": True},
            },
        )

    async def scenario() -> None:
        with upstream_server(directory, exchanges) as upstream:
            async with upstream_session(mcp_binary, directory, upstream, writes=True) as session:
                prepared = await session.call_tool("remote_update_state", arguments)
                assert not prepared.isError
                repeated = await session.call_tool("remote_update_state", arguments)
                token = prepared.structuredContent["confirmation_token"]
                assert repeated.structuredContent["confirmation_token"] == token
                changed = await session.call_tool(
                    "remote_update_state",
                    {**arguments, "expected_current_state": "different-prepared-state"},
                )
                assert changed.isError
                assert changed.content[0].text == (
                    "idempotency_key is already bound to different write arguments"
                )
                commit = _commit(prepared.structuredContent, key)
                wrong_scope = await session.call_tool("remote_create_request", commit)
                assert wrong_scope.isError
                assert wrong_scope.content[0].text == (
                    "confirmation token does not match this write operation"
                )
                assert upstream.observed == [], "preparation or rejection performed source I/O"
                result = await session.call_tool("remote_update_state", commit)
                assert result.isError is not matches
                if matches:
                    assert result.structuredContent == {
                        "instance_id": "synthetic",
                        "state": {"updated": True},
                    }
                else:
                    assert result.content[0].text == (
                        "remote operation failed; see operator audit telemetry"
                    )
                    assert actual not in result.model_dump_json()
                replay = await session.call_tool("remote_update_state", commit)
                assert replay.isError
                assert replay.content[0].text == (
                    "confirmation_token is missing, expired, or already used"
                )
                await session.send_ping()
            upstream.assert_complete()
            assert [request.method for request in upstream.observed] == (
                ["GET", "PUT"] if matches else ["GET"]
            )
            if matches:
                assert json.loads(upstream.observed[1].body) == {"state": target}
        _assert_private_diagnostics(directory, key, token)

    anyio.run(scenario)


@settings(max_examples=10, deadline=None, derandomize=True)
@given(
    components=st.lists(
        st.text(alphabet="abcdef0123456789_-", min_size=1, max_size=8),
        min_size=1,
        max_size=3,
    ),
    separator=st.sampled_from(("/", "\\")),
)
def test_generated_attachment_paths_normalize_with_exact_multipart_bytes(
    mcp_binary: Path,
    tmp_path_factory: pytest.TempPathFactory,
    components: list[str],
    separator: str,
) -> None:
    """Both accepted separators name the same confined synthetic attachment."""
    directory = tmp_path_factory.mktemp("mcp-generated-attachment")
    attachment = directory.joinpath(*components, "synthetic.txt")
    attachment.parent.mkdir(parents=True)
    contents = "Synthetic generated attachment Café 東京"
    attachment.write_text(contents, encoding="utf-8")
    relative = separator.join([*components, "synthetic.txt"])
    arguments = {
        "instance_id": "synthetic",
        "request_id": 7001,
        "direction": "response",
        "body": "Synthetic body",
        "sent_at": "2026-11-17T00:00:00Z",
        "attachments": [relative],
        "idempotency_key": "generated-attachment",
    }
    exchange = {
        "method": "POST",
        "path": "/api/v2/request/7001/correspondence.json",
        "status": 201,
        "json": {"success": True},
    }

    async def scenario() -> None:
        with upstream_server(directory, [exchange]) as upstream:
            async with upstream_session(mcp_binary, directory, upstream, writes=True) as session:
                prepared = await session.call_tool("remote_add_correspondence", arguments)
                assert not prepared.isError
                assert upstream.observed == []
                result = await session.call_tool(
                    "remote_add_correspondence",
                    _commit(prepared.structuredContent, arguments["idempotency_key"]),
                )
                assert not result.isError
                assert result.structuredContent == {
                    "instance_id": "synthetic",
                    "correspondence": {"success": True},
                }
            upstream.assert_complete()
            assert len(upstream.observed) == 1
            request = upstream.observed[0]
            message = BytesParser(policy=policy.default).parsebytes(
                f"Content-Type: {request.headers['content-type']}\r\n\r\n".encode() + request.body,
            )
            attachments = [part for part in message.iter_parts() if part.get_filename()]
            assert len(attachments) == 1
            assert attachments[0].get_filename() == "synthetic.txt"
            assert attachments[0].get_payload(decode=True) == contents.encode()

    anyio.run(scenario)
    _assert_private_diagnostics(directory, str(directory))


@settings(max_examples=12, deadline=None, derandomize=True)
@given(segment=st.text(alphabet="abcdef0123456789_-", min_size=1, max_size=20))
def test_generated_unsafe_paths_are_rejected_before_source_io(
    mcp_binary: Path,
    tmp_path_factory: pytest.TempPathFactory,
    segment: str,
) -> None:
    """Explicit traversal/absolute/empty-component classes never reach a source."""
    directory = tmp_path_factory.mktemp("mcp-generated-path-denial")
    paths = (
        f"../{segment}",
        f"{segment}/../synthetic.txt",
        f"{segment}\\..\\synthetic.txt",
        f"/{segment}",
        f"C:\\{segment}",
        f"{segment}//synthetic.txt",
        f"{segment}/./synthetic.txt",
        f"{segment}\0synthetic.txt",
    )

    async def scenario() -> None:
        with upstream_server(directory, []) as upstream:
            async with upstream_session(mcp_binary, directory, upstream, writes=True) as session:
                for index, path in enumerate(paths):
                    result = await session.call_tool(
                        "remote_add_correspondence",
                        {
                            "instance_id": "synthetic",
                            "request_id": 7001,
                            "direction": "response",
                            "body": PRIVATE + segment,
                            "sent_at": "2026-11-17T00:00:00Z",
                            "attachments": [path],
                            "idempotency_key": f"path-{index}",
                        },
                    )
                    assert result.isError
                    assert result.content[0].text == "attachment path is invalid"
                    _assert_omits(result.model_dump(mode="json"), path)
                await session.send_ping()
            upstream.assert_complete()
            assert upstream.observed == []

    anyio.run(scenario)
    _assert_private_diagnostics(directory, PRIVATE + segment)


@settings(max_examples=12, deadline=None, derandomize=True)
@given(
    status=st.sampled_from((401, 403, 404, 429, 500, 503)),
    private=st.text(alphabet='Café東京\n\r\\" 401 503', min_size=1, max_size=100),
)
def test_generated_source_errors_keep_bodies_and_header_values_private(
    mcp_binary: Path,
    tmp_path_factory: pytest.TempPathFactory,
    status: int,
    private: str,
) -> None:
    """Untrusted error text cannot become an MCP error or an unbounded retry."""
    directory = tmp_path_factory.mktemp("mcp-generated-redaction")
    marker = PRIVATE + private
    exchange = {
        "method": "GET",
        "path": "/api/v2/version.json",
        "status": status,
        "body": marker,
        "headers": {"X-Synthetic-Private": PRIVATE + "header"},
    }

    async def scenario() -> None:
        with upstream_server(directory, [exchange]) as upstream:
            async with upstream_session(mcp_binary, directory, upstream) as session:
                result = await session.call_tool("remote_version", {"instance_id": "synthetic"})
                assert result.isError
                assert result.structuredContent is None
                assert result.content[0].text == (
                    "remote operation failed; see operator audit telemetry"
                )
                assert PRIVATE not in result.model_dump_json()
                await session.send_ping()
            upstream.assert_complete()
            assert len(upstream.observed) == 1

    anyio.run(scenario)
    # A fixed ASCII prefix also detects escaped Unicode/newline/quote leaks.
    _assert_private_diagnostics(directory, PRIVATE, marker, PRIVATE + "header")
