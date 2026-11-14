"""Exercise the Rust server through the official MCP SDK, using local processes only.

The SDK is supplied by the project's existing fastmcp dependency. Set
FYI_MCP_TEST_BINARY to test a specific built executable. Otherwise build once
with Cargo, or reuse the workspace debug binary only when Cargo is unavailable.
Only the absence of both Cargo and a binary can skip these tests.

Run the bounded local performance assessment with::

    pytest -q -s tests/test_mcp_client_interop.py -m performance

It uses one persistent SDK HTTP session against an empty ephemeral database,
warms ten check_status calls, then times 100 sequential calls including client
serialization, loopback transport, database work and SDK output validation.
Reported p50 is the median; p95 is the nearest-rank percentile. Linux memory is
the server process's /proc VmRSS and lifetime VmHWM, excluding the Python client.
The JSON output includes raw samples, the executable SHA-256 and runtime details
in pytest's temporary directory. Conservative local fixture regression budgets
are 250 ms p95, 128 MiB peak RSS and 16 MiB RSS growth after warmup; these do not
establish remote-service, populated-database or deployment-wide memory bounds.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import platform
import shutil
import socket
import statistics
import subprocess
import tempfile
import time
from contextlib import asynccontextmanager, contextmanager
from dataclasses import dataclass
from datetime import timedelta
from importlib.metadata import version
from pathlib import Path
from typing import TYPE_CHECKING

import anyio
import httpx
import pytest
from hypothesis import given, settings
from hypothesis import strategies as st
from jsonschema import Draft202012Validator
from mcp import ClientSession, StdioServerParameters, types
from mcp.client.stdio import stdio_client
from mcp.client.streamable_http import streamable_http_client
from mcp.shared.exceptions import McpError

from scripts import verify_remote_mcp_contract as contract

if TYPE_CHECKING:
    from collections.abc import AsyncIterator, Iterator


ROOT = Path(__file__).resolve().parents[1]
REQUEST_TIMEOUT = timedelta(seconds=5)
SESSION_TIMEOUT_SECONDS = 30
SUPPORTED_REVISIONS = {"2025-06-18", "2025-03-26"}
SYNTHETIC_BEARER = "synthetic-mcp-client-interop-bearer"
PRIVATE_MARKER = "synthetic-sdk-private-value"
INVALID_ARGUMENTS_MESSAGE = "Tool arguments do not match the advertised schema"
PERFORMANCE_WARMUP_CALLS = 10
PERFORMANCE_MEASURED_CALLS = 100
PERFORMANCE_P95_BUDGET_MS = 250
PERFORMANCE_RSS_BUDGET_BYTES = 128 * 1024 * 1024
PERFORMANCE_RSS_GROWTH_BUDGET_BYTES = 16 * 1024 * 1024
REMOTE_WRITE_TOOLS = {
    "remote_create_request",
    "remote_add_correspondence",
    "remote_update_state",
}

pytestmark = [pytest.mark.integration, pytest.mark.e2e, pytest.mark.compatibility]


def _cargo_executable() -> str | None:
    cargo = shutil.which("cargo")
    if cargo is not None:
        return cargo
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    installed = cargo_home / "bin" / ("cargo.exe" if os.name == "nt" else "cargo")
    return str(installed) if installed.is_file() else None


@pytest.fixture(scope="session")
def mcp_binary() -> Path:
    """Fail visibly if an available build or an explicitly selected binary fails."""
    configured = os.environ.get("FYI_MCP_TEST_BINARY")
    if configured is not None:
        binary = Path(configured).expanduser().resolve()
        if not configured or not binary.is_file():
            pytest.fail("FYI_MCP_TEST_BINARY must name an existing executable")
        return binary

    binary_name = "fyi-mcp.exe" if os.name == "nt" else "fyi-mcp"
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    if not target_dir.is_absolute():
        target_dir = ROOT / target_dir
    binary = target_dir / "debug" / binary_name
    cargo = _cargo_executable()
    if cargo is None:
        if binary.is_file():
            return binary.resolve()
        pytest.skip("MCP SDK interoperability requires Cargo or FYI_MCP_TEST_BINARY")
    build = subprocess.run(  # noqa: S603 - fixed Cargo arguments, no shell or caller input.
        [cargo, "build", "-p", "fyi-mcp", "--locked", "--message-format=json"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=900,
        check=False,
    )
    if build.returncode != 0:
        pytest.fail(f"fyi-mcp build failed:\n{build.stderr[-12000:]}")
    built_binary = None
    for line in build.stdout.splitlines():
        artifact = json.loads(line)
        if artifact.get("target", {}).get("name") == "fyi-mcp" and artifact.get("executable"):
            built_binary = Path(artifact["executable"])
    if built_binary is None or not built_binary.is_file():
        pytest.fail("Cargo succeeded without producing the fyi-mcp executable")
    return built_binary.resolve()


def _server_environment(**settings: str) -> dict[str, str]:
    """Allow only process essentials and synthetic settings into the server."""
    environment = {
        name: os.environ[name]
        for name in ("PATH", "SYSTEMROOT", "WINDIR", "TEMP", "TMP", "PATHEXT")
        if name in os.environ
    }
    environment.update(
        {
            "FYI_MCP_EPHEMERAL": "1",
            "FYI_MCP_TRANSPORT": "stdio",
            "FYI_MCP_REMOTE_INSTANCES": "",
            "FYI_MCP_REMOTE_READ": "",
            "FYI_MCP_REMOTE_WRITE": "",
            "RUST_LOG": "warn",
        },
    )
    environment.update(settings)
    return environment


@dataclass(frozen=True)
class _HttpServer:
    endpoint: str
    pid: int


@contextmanager
def _http_server(binary: Path, directory: Path) -> Iterator[_HttpServer]:
    """Start a loopback-only fixture and reap it even if a client assertion fails."""
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    address = f"127.0.0.1:{port}"
    base_url = f"http://{address}"
    environment = _server_environment(
        FYI_MCP_TRANSPORT="http",
        FYI_MCP_HTTP_ADDR=address,
        FYI_MCP_HTTP_BEARER_TOKEN=SYNTHETIC_BEARER,
    )
    with (directory / "http-server.stderr").open("w+", encoding="utf-8") as diagnostics:
        process = subprocess.Popen(  # noqa: S603 - the validated Rust test binary, no shell.
            [str(binary)],
            cwd=directory,
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=diagnostics,
        )
        try:
            deadline = time.monotonic() + 10
            with httpx.Client(timeout=0.25, trust_env=False) as readiness:
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        diagnostics.seek(0)
                        pytest.fail(f"HTTP MCP server exited during startup:\n{diagnostics.read()}")
                    try:
                        if readiness.get(f"{base_url}/healthz").status_code == 200:
                            break
                    except httpx.TransportError:
                        pass
                    time.sleep(0.025)
                else:
                    pytest.fail("HTTP MCP server did not become ready within 10 seconds")
            yield _HttpServer(endpoint=f"{base_url}/mcp", pid=process.pid)
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
            assert process.poll() is not None, "HTTP MCP subprocess was not reaped"


@asynccontextmanager
async def _http_client_session(endpoint: str) -> AsyncIterator[ClientSession]:
    """Share the same authenticated SDK connection for contracts and measurement."""
    async with (
        httpx.AsyncClient(
            headers={"Authorization": f"Bearer {SYNTHETIC_BEARER}"},
            timeout=5,
            trust_env=False,
        ) as http_client,
        streamable_http_client(endpoint, http_client=http_client) as streams,
        ClientSession(
            streams[0],
            streams[1],
            read_timeout_seconds=REQUEST_TIMEOUT,
        ) as session,
    ):
        yield session


@asynccontextmanager
async def _sdk_session(
    binary: Path,
    directory: Path,
    transport: str,
    *,
    remote_write: bool = False,
    remote_read: bool = False,
) -> AsyncIterator[ClientSession]:
    """Use unmodified SDK transport/session lifecycle with bounded request waits."""
    with anyio.fail_after(SESSION_TIMEOUT_SECONDS):
        if transport == "http":
            with _http_server(binary, directory) as server:
                async with _http_client_session(server.endpoint) as session:
                    yield session
        else:
            environment = _server_environment()
            if remote_write or remote_read:
                environment.update(
                    {
                        "FYI_MCP_REMOTE_INSTANCES": "synthetic=https://mcp-fixture.invalid",
                        "FYI_MCP_REMOTE_READ": "synthetic",
                        "FYI_MCP_REMOTE_WRITE": "synthetic" if remote_write else "",
                    },
                )
            server = StdioServerParameters(
                command=str(binary),
                cwd=directory,
                env=environment,
            )
            with (directory / "stdio-server.stderr").open("w", encoding="utf-8") as diagnostics:
                async with (
                    stdio_client(server, errlog=diagnostics) as streams,
                    ClientSession(
                        *streams,
                        read_timeout_seconds=REQUEST_TIMEOUT,
                    ) as session,
                ):
                    yield session


async def _exercise_local_contract(session: ClientSession, expected_revision: str) -> None:
    initialized = await session.initialize()
    assert initialized.protocolVersion == expected_revision
    assert initialized.capabilities.tools is not None
    assert (await session.send_ping()).model_dump(exclude_none=True) == {}

    discovered = await session.list_tools()
    names = [tool.name for tool in discovered.tools]
    assert len(names) == len(set(names)), "Tool registration contains duplicates"
    assert {"check_status", "list_requests", "retrieve_request"} <= set(names)
    assert not any(name.startswith("remote_") for name in names)
    for tool in discovered.tools:
        Draft202012Validator.check_schema(tool.inputSchema)
        assert tool.inputSchema["type"] == "object"
        if tool.outputSchema is not None:
            Draft202012Validator.check_schema(tool.outputSchema)

    # ClientSession.call_tool also checks successful output against outputSchema.
    health = await session.call_tool("check_status", {})
    assert not health.isError
    assert health.structuredContent is not None
    assert health.structuredContent["status"] == "healthy"
    requests = await session.call_tool("list_requests", {"limit": 1})
    assert not requests.isError
    assert requests.structuredContent == {"requests": []}

    missing = await session.call_tool("retrieve_request", {"id": 999999})
    assert missing.isError, "Missing local data must be a tool error, not a protocol error"
    assert missing.content

    denied = await session.call_tool(
        "remote_create_request",
        {
            "instance_id": "unconfigured",
            "phase": "prepare",
            "title": "Synthetic contract request",
            "body": "Synthetic fixture only; must never be transmitted.",
            "external_user_name": "Synthetic Client",
            "external_url": "https://mcp-fixture.invalid/request/1",
            "idempotency_key": "unconfigured-write",
        },
    )
    assert denied.isError, "Hidden remote tools must still enforce the authorization boundary"
    assert (await session.call_tool("list_requests", {})).structuredContent == {"requests": []}
    await _exercise_local_sync_contract(session)


async def _exercise_local_sync_contract(session: ClientSession) -> None:
    """Validate both success shapes against the SDK's discovered output schema."""
    created = await session.call_tool(
        "create_request",
        {
            "title": "Synthetic SDK contract request",
            "body": "Ephemeral local fixture; no remote submission.",
            "status": "draft",
        },
    )
    assert not created.isError
    assert created.structuredContent is not None
    request_id = created.structuredContent["request"]["id"]
    aggregate = await session.call_tool("sync_status", {})
    assert not aggregate.isError
    assert aggregate.structuredContent is not None
    assert aggregate.structuredContent["total"] == 1
    individual = await session.call_tool("sync_status", {"request_id": request_id})
    assert not individual.isError
    assert individual.structuredContent is not None
    assert individual.structuredContent["request_id"] == request_id
    retrieved = await session.call_tool("retrieve_request", {"id": request_id})
    assert not retrieved.isError
    assert retrieved.structuredContent is not None
    assert retrieved.structuredContent["request"]["id"] == request_id


@pytest.mark.parametrize("transport", ["stdio", "http"])
@pytest.mark.parametrize("requested_revision", [None, "2025-06-18", "2025-03-26"])
def test_official_sdk_local_contract(
    mcp_binary: Path,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    transport: str,
    requested_revision: str | None,
) -> None:
    """Discover and call the executable without inheriting operator credentials."""
    if requested_revision is not None:
        # Select a legacy profile without replacing any SDK session/transport code.
        monkeypatch.setattr(types, "LATEST_PROTOCOL_VERSION", requested_revision)
    monkeypatch.setenv("FYI_MCP_REMOTE_INSTANCES", "inherited=https://inherited.invalid")
    monkeypatch.setenv("FYI_MCP_REMOTE_WRITE", "inherited")
    monkeypatch.setenv("FYI_MCP_REMOTE_BOT_TOKEN_INHERITED", "synthetic-inherited-secret")
    monkeypatch.setenv("FYI_MCP_HTTP_BEARER_TOKEN", "synthetic-inherited-http-secret")
    monkeypatch.setenv("DATABASE_URL", "sqlite:///must-not-open-operator-database")

    async def scenario() -> None:
        async with _sdk_session(mcp_binary, tmp_path, transport) as session:
            await _exercise_local_contract(session, requested_revision or "2025-06-18")

    anyio.run(scenario)
    diagnostics = (tmp_path / f"{transport}-server.stderr").read_text(encoding="utf-8")
    assert "synthetic-inherited-secret" not in diagnostics
    assert "synthetic-inherited-http-secret" not in diagnostics
    assert "must-not-open-operator-database" not in diagnostics


async def _assert_invalid_tool_call(
    session: ClientSession,
    name: str,
    arguments: dict[str, object],
    message: str = INVALID_ARGUMENTS_MESSAGE,
) -> None:
    with pytest.raises(McpError) as rejected:
        await session.call_tool(name, arguments)
    assert rejected.value.error.model_dump(exclude_none=True) == {
        "code": -32602,
        "message": message,
    }


@pytest.mark.security
@pytest.mark.edge
@pytest.mark.parametrize("transport", ["stdio", "http"])
def test_official_sdk_invalid_arguments_and_private_errors(
    mcp_binary: Path,
    tmp_path: Path,
    transport: str,
) -> None:
    """Reject malformed parameters before dispatch, without echoing caller data."""
    invalid_calls: list[tuple[str, dict[str, object]]] = [
        ("list_requests", {"limit": False}),
        ("list_requests", {"limit": PRIVATE_MARKER}),
        ("list_requests", {"limit": 0}),
        ("list_requests", {"limit": 501}),
        ("retrieve_request", {}),
        ("retrieve_request", {"id": 1.5}),
        ("retrieve_request", {"id": 2**63}),
        ("check_status", {PRIVATE_MARKER: PRIVATE_MARKER}),
        ("create_request", {"title": "", "body": PRIVATE_MARKER}),
        ("create_request", {"title": PRIVATE_MARKER, "body": "Fixture", "status": PRIVATE_MARKER}),
        ("create_request", {"title": "Fixture", "body": PRIVATE_MARKER, "tags": [1]}),
        ("sync_resolve_conflict", {"request_id": 1, "mark_clean": "true"}),
        ("compute_deadline", {"start_date": "2026-01-01", "days": 36501}),
    ]

    async def scenario() -> None:
        async with _sdk_session(mcp_binary, tmp_path, transport) as session:
            await session.initialize()
            for name, arguments in invalid_calls:
                await _assert_invalid_tool_call(session, name, arguments)
            await _assert_invalid_tool_call(
                session,
                f"unknown_{PRIVATE_MARKER}",
                {PRIVATE_MARKER: PRIVATE_MARKER},
                "Unknown tool",
            )
            # A schema-valid but unparseable date is a private tool runtime error.
            invalid_date = await session.call_tool(
                "compute_deadline",
                {"start_date": "PRIVATE-DT", "days": 1},
            )
            assert invalid_date.isError
            assert len(invalid_date.content) == 1
            assert invalid_date.content[0].model_dump(exclude_none=True) == {
                "type": "text",
                "text": "Invalid start_date (expected YYYY-MM-DD)",
            }
            # JSON Schema integers include losslessly representable 1.0.
            requests = await session.call_tool("list_requests", {"limit": 1.0})
            assert not requests.isError
            assert requests.structuredContent == {"requests": []}
            assert (await session.send_ping()).model_dump(exclude_none=True) == {}

    anyio.run(scenario)
    diagnostics = (tmp_path / f"{transport}-server.stderr").read_text(encoding="utf-8")
    assert PRIVATE_MARKER not in diagnostics
    assert "PRIVATE-DT" not in diagnostics


@st.composite
def _invalid_tool_calls(draw: st.DrawFn) -> tuple[str, dict[str, object]]:
    """Vary wrong types, bounds and unregistered keys independently of the server."""
    kind = draw(st.sampled_from(["create_type", "list_bound", "extra_property"]))
    if kind == "create_type":
        field = draw(st.sampled_from(["title", "body"]))
        integer = st.integers(min_value=-(2**63), max_value=2**63 - 1)
        wrong_type = draw(
            st.one_of(
                st.none(),
                st.booleans(),
                integer,
                st.lists(integer, max_size=3),
                st.dictionaries(st.text(max_size=8), st.booleans(), max_size=3),
            ),
        )
        arguments: dict[str, object] = {"title": "Synthetic fixture", "body": PRIVATE_MARKER}
        arguments[field] = wrong_type
        return "create_request", arguments
    if kind == "list_bound":
        limit = draw(
            st.one_of(
                st.integers(min_value=-(2**63), max_value=0),
                st.integers(min_value=501, max_value=2**63 - 1),
            ),
        )
        return "list_requests", {"limit": limit}
    extra = draw(st.text(alphabet="abcdefghijklmnopqrstuvwxyz_", min_size=1, max_size=24))
    return "create_request", {
        "title": "Synthetic fixture",
        "body": PRIVATE_MARKER,
        f"unknown_{extra}": PRIVATE_MARKER,
    }


@pytest.mark.property
@pytest.mark.security
@settings(max_examples=20, deadline=None, derandomize=True)
@given(case=_invalid_tool_calls())
def test_official_sdk_invalid_parameter_property(
    mcp_binary: Path,
    case: tuple[str, dict[str, object]],
) -> None:
    """Generated invalid calls leave an independently started server usable and empty."""
    name, arguments = case
    with tempfile.TemporaryDirectory(prefix="fyi-mcp-property-") as directory:

        async def scenario() -> None:
            async with _sdk_session(mcp_binary, Path(directory), "stdio") as session:
                await session.initialize()
                await _assert_invalid_tool_call(session, name, arguments)
                assert (await session.call_tool("list_requests", {})).structuredContent == {
                    "requests": [],
                }
                assert (await session.send_ping()).model_dump(exclude_none=True) == {}

        anyio.run(scenario)


@pytest.mark.security
def test_official_sdk_read_only_registration(mcp_binary: Path, tmp_path: Path) -> None:
    """Read enablement advertises reads while every synthetic write stays denied."""

    async def scenario() -> None:
        async with _sdk_session(mcp_binary, tmp_path, "stdio", remote_read=True) as session:
            await session.initialize()
            names = {tool.name for tool in (await session.list_tools()).tools}
            assert {name for name in names if name.startswith("remote_")} == {
                "remote_health",
                "remote_version",
                "remote_search_requests",
                "remote_get_request",
                "remote_list_authorities",
            }
            fixtures = json.loads(contract.CURRENT_FIXTURE.read_text(encoding="utf-8"))
            prepared_cases = [
                case for case in fixtures["cases"] if case["name"].endswith("_prepared")
            ]
            assert len(prepared_cases) == 3
            for case in prepared_cases:
                params = case["request"]["params"]
                denied = await session.call_tool(params["name"], params["arguments"])
                assert denied.isError
                assert denied.structuredContent is None
            assert (await session.call_tool("list_requests", {})).structuredContent == {
                "requests": [],
            }
            assert (await session.send_ping()).model_dump(exclude_none=True) == {}

    anyio.run(scenario)


@pytest.mark.security
def test_official_sdk_synthetic_write_preparation(mcp_binary: Path, tmp_path: Path) -> None:
    """Prepare synthetic data through the SDK; never send a valid remote commit."""

    async def scenario() -> None:
        async with _sdk_session(mcp_binary, tmp_path, "stdio", remote_write=True) as session:
            initialized = await session.initialize()
            assert initialized.protocolVersion in SUPPORTED_REVISIONS
            discovered = await session.list_tools()
            snapshot = json.loads(contract.TOOLS_FIXTURE.read_text(encoding="utf-8"))
            assert (
                contract.compare_tool_catalogue(
                    [tool.model_dump(mode="json", exclude_none=True) for tool in discovered.tools],
                    snapshot,
                )
                == []
            )
            tools = {tool.name: tool for tool in discovered.tools}
            assert tools.keys() >= REMOTE_WRITE_TOOLS
            # Semantic input rejection must not trip the upstream circuit or
            # consume remote attempts before these valid preparations.
            for _ in range(3):
                invalid_read = await session.call_tool(
                    "remote_search_requests",
                    {"instance_id": "synthetic", "query": "   "},
                )
                assert invalid_read.isError
                assert invalid_read.content[0].text == "query must be between 1 and 200 characters"
            for name in REMOTE_WRITE_TOOLS:
                assert tools[name].annotations is not None
                assert tools[name].annotations.readOnlyHint is False
                assert tools[name].outputSchema is not None

            fixtures = json.loads(contract.CURRENT_FIXTURE.read_text(encoding="utf-8"))
            prepared_cases = [
                case for case in fixtures["cases"] if case["name"].endswith("_prepared")
            ]
            assert len(prepared_cases) == 3
            for case in prepared_cases:
                params = case["request"]["params"]
                prepared_fixture = await session.call_tool(params["name"], params["arguments"])
                assert not prepared_fixture.isError
                assert prepared_fixture.structuredContent is not None
                assert prepared_fixture.structuredContent["status"] == "prepared"
                assert prepared_fixture.structuredContent["requires_commit"] is True

            arguments = {
                "instance_id": "synthetic",
                "phase": "prepare",
                "title": "Synthetic contract request",
                "body": "Synthetic fixture only; must never be transmitted.",
                "external_user_name": "Synthetic Client",
                "external_url": "https://mcp-fixture.invalid/request/1",
                "idempotency_key": "sdk-prepare-only",
            }
            Draft202012Validator(tools["remote_create_request"].inputSchema).validate(arguments)
            prepared = await session.call_tool("remote_create_request", arguments)
            assert not prepared.isError
            assert prepared.structuredContent is not None
            payload = prepared.structuredContent
            assert payload["status"] == "prepared"
            assert payload["requires_commit"] is True
            assert isinstance(payload["confirmation_token"], str)
            assert payload["confirmation_token"]
            assert 0 < payload["expires_in_seconds"] <= 300

            # A token from one operation cannot authorize another operation.
            denied = await session.call_tool(
                "remote_update_state",
                {
                    "instance_id": "synthetic",
                    "phase": "commit",
                    "confirmation_token": payload["confirmation_token"],
                    "idempotency_key": arguments["idempotency_key"],
                },
            )
            assert denied.isError
            mismatched_key = await session.call_tool(
                "remote_create_request",
                {
                    "instance_id": "synthetic",
                    "phase": "commit",
                    "confirmation_token": payload["confirmation_token"],
                    "idempotency_key": "different-synthetic-key",
                },
            )
            assert mismatched_key.isError
            await _assert_invalid_tool_call(
                session,
                "remote_create_request",
                {
                    "instance_id": "synthetic",
                    "phase": "commit",
                    "confirmation_token": payload["confirmation_token"],
                    "idempotency_key": arguments["idempotency_key"],
                    "body": PRIVATE_MARKER,
                },
            )
            repeated = await session.call_tool("remote_create_request", arguments)
            assert not repeated.isError
            assert repeated.structuredContent is not None
            assert repeated.structuredContent["confirmation_token"] == payload["confirmation_token"]
            assert (await session.call_tool("list_requests", {})).structuredContent == {
                "requests": [],
            }

    anyio.run(scenario)


@pytest.mark.security
def test_http_authentication_origin_and_version_boundaries(
    mcp_binary: Path,
    tmp_path: Path,
) -> None:
    """Reject invalid transport credentials before allowing a synthetic request."""
    with (
        _http_server(mcp_binary, tmp_path) as server,
        httpx.Client(
            timeout=5,
            trust_env=False,
        ) as client,
    ):
        base_headers = {
            "Accept": "application/json, text/event-stream",
            "MCP-Protocol-Version": "2025-06-18",
        }
        message = {"jsonrpc": "2.0", "id": "auth-probe", "method": "ping"}
        for authorization in (None, "Bearer synthetic-wrong-bearer"):
            headers = dict(base_headers)
            if authorization is not None:
                headers["Authorization"] = authorization
            response = client.post(server.endpoint, headers=headers, json=message)
            assert response.status_code == 401
            assert response.headers["www-authenticate"].startswith("Bearer")
            assert SYNTHETIC_BEARER not in response.text

        valid_headers = {**base_headers, "Authorization": f"Bearer {SYNTHETIC_BEARER}"}
        invalid_origin = client.post(
            server.endpoint,
            headers={**valid_headers, "Origin": "https://untrusted-origin.invalid"},
            json=message,
        )
        assert invalid_origin.status_code == 403
        unsupported = client.post(
            server.endpoint,
            headers={**valid_headers, "MCP-Protocol-Version": "1900-01-01"},
            json=message,
        )
        assert unsupported.status_code == 400
        assert client.get(server.endpoint, headers=valid_headers).status_code == 405
        valid = client.post(server.endpoint, headers=valid_headers, json=message)
        assert valid.status_code == 200
        assert valid.json() == {"jsonrpc": "2.0", "id": "auth-probe", "result": {}}


def _linux_process_memory(pid: int) -> dict[str, int] | None:
    """Read actual resident pages; unsupported platforms still assess latency."""
    if platform.system() != "Linux":
        return None
    status = Path(f"/proc/{pid}/status").read_text(encoding="utf-8")
    memory = {}
    for line in status.splitlines():
        fields = line.split()
        if fields and fields[0] in {"VmRSS:", "VmHWM:"}:
            assert fields[2] == "kB"
            memory[fields[0].removesuffix(":")] = int(fields[1]) * 1024
    assert memory.keys() == {"VmRSS", "VmHWM"}, "Linux server RSS measurement is unavailable"
    assert all(value > 0 for value in memory.values())
    return memory


async def _measure_http_workload(session: ClientSession, pid: int) -> dict[str, object]:
    initialized = await session.initialize()
    assert initialized.protocolVersion in SUPPORTED_REVISIONS
    await session.list_tools()
    for _ in range(PERFORMANCE_WARMUP_CALLS):
        assert not (await session.call_tool("check_status", {})).isError
    memory_before = _linux_process_memory(pid)
    elapsed_ms = []
    for _ in range(PERFORMANCE_MEASURED_CALLS):
        started = time.perf_counter_ns()
        result = await session.call_tool("check_status", {})
        elapsed_ms.append((time.perf_counter_ns() - started) / 1_000_000)
        assert not result.isError
        assert result.structuredContent is not None
        assert result.structuredContent["status"] == "healthy"
    memory_after = _linux_process_memory(pid)
    return {
        "workload": "official SDK check_status, sequential persistent HTTP, empty ephemeral SQLite",
        "protocol_revision": initialized.protocolVersion,
        "warmup_calls": PERFORMANCE_WARMUP_CALLS,
        "measured_calls": PERFORMANCE_MEASURED_CALLS,
        "latency_p50_ms": statistics.median(elapsed_ms),
        "latency_p95_ms": sorted(elapsed_ms)[math.ceil(0.95 * len(elapsed_ms)) - 1],
        "latency_max_ms": max(elapsed_ms),
        "latency_samples_ms": elapsed_ms,
        "memory_source": "/proc/PID/status"
        if memory_after is not None
        else "unavailable on this OS",
        "memory_before_bytes": memory_before,
        "memory_after_bytes": memory_after,
        "latency_p95_budget_ms": PERFORMANCE_P95_BUDGET_MS,
        "peak_rss_budget_bytes": PERFORMANCE_RSS_BUDGET_BYTES,
        "rss_growth_budget_bytes": PERFORMANCE_RSS_GROWTH_BUDGET_BYTES,
    }


@pytest.mark.performance
def test_official_sdk_http_latency_and_rss(mcp_binary: Path, tmp_path: Path) -> None:
    """Bound and record a warmed SDK path; no remote I/O or separate load harness."""
    with _http_server(mcp_binary, tmp_path) as server:
        # /proc identifies the running image even if another build replaces its path.
        image = Path(f"/proc/{server.pid}/exe") if platform.system() == "Linux" else mcp_binary
        with image.open("rb") as executable:
            binary_sha256 = hashlib.file_digest(executable, "sha256").hexdigest()

        async def scenario() -> dict[str, object]:
            with anyio.fail_after(SESSION_TIMEOUT_SECONDS):
                async with _http_client_session(server.endpoint) as session:
                    return await _measure_http_workload(session, server.pid)

        metrics = anyio.run(scenario)
    metrics.update(
        {
            "binary_path": str(mcp_binary),
            "binary_sha256": binary_sha256,
            "sdk_version": version("mcp"),
            "python_version": platform.python_version(),
            "platform": platform.platform(),
            "architecture": platform.machine(),
        },
    )
    report = tmp_path / "mcp-performance.json"
    report.write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")
    summary = {key: value for key, value in metrics.items() if key != "latency_samples_ms"}
    summary["samples_artifact"] = str(report)
    print("MCP_PERFORMANCE " + json.dumps(summary, sort_keys=True))  # noqa: T201
    assert metrics["latency_p95_ms"] <= PERFORMANCE_P95_BUDGET_MS
    memory_after = metrics["memory_after_bytes"]
    if memory_after is not None:
        assert memory_after["VmHWM"] <= PERFORMANCE_RSS_BUDGET_BYTES
        assert (
            memory_after["VmRSS"] - metrics["memory_before_bytes"]["VmRSS"]
            <= PERFORMANCE_RSS_GROWTH_BUDGET_BYTES
        )
