"""A local TLS origin behind a non-forwarding CONNECT fixture.

Only mcp-fixture.invalid:443 is accepted. No socket is ever opened to a remote
host. The ephemeral CA is trusted only by the tested child via SSL_CERT_FILE;
system trust and production source validation are unchanged. This trust path is
qualified on Linux only. Keys and synthetic request captures stay in pytest's
temporary directory/memory and are never committed as fixture credentials.
"""

from __future__ import annotations

import json
import socketserver
import ssl
import threading
from collections import deque
from contextlib import asynccontextmanager, contextmanager
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from http.server import BaseHTTPRequestHandler
from typing import TYPE_CHECKING

import anyio
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from tests.test_mcp_client_interop import _server_environment

if TYPE_CHECKING:
    from collections.abc import AsyncIterator, Iterator
    from pathlib import Path

HOST = "mcp-fixture.invalid"
SYNTHETIC_HEADER_VALUE = "synthetic-mcp-upstream-private-header-value"
PRIVATE_MARKER = "synthetic-upstream-private-error"
MAX_REQUEST_BYTES = 2 * 1024 * 1024


def _certificates(directory: Path) -> ssl.SSLContext:
    now = datetime.now(UTC)
    ca_key = ec.generate_private_key(ec.SECP256R1())
    ca_name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, "Synthetic MCP fixture CA")])
    ca = (
        x509.CertificateBuilder()
        .subject_name(ca_name)
        .issuer_name(ca_name)
        .public_key(ca_key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now - timedelta(hours=1))
        .not_valid_after(now + timedelta(days=1))
        .add_extension(x509.BasicConstraints(ca=True, path_length=0), critical=True)
        .add_extension(
            x509.KeyUsage(
                digital_signature=True,
                content_commitment=False,
                key_encipherment=False,
                data_encipherment=False,
                key_agreement=False,
                key_cert_sign=True,
                crl_sign=True,
                encipher_only=None,
                decipher_only=None,
            ),
            critical=True,
        )
        .sign(ca_key, hashes.SHA256())
    )
    key = ec.generate_private_key(ec.SECP256R1())
    leaf = (
        x509.CertificateBuilder()
        .subject_name(x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, HOST)]))
        .issuer_name(ca_name)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(now - timedelta(hours=1))
        .not_valid_after(now + timedelta(days=1))
        .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
        .add_extension(x509.SubjectAlternativeName([x509.DNSName(HOST)]), critical=False)
        .add_extension(x509.ExtendedKeyUsage([ExtendedKeyUsageOID.SERVER_AUTH]), critical=False)
        .sign(ca_key, hashes.SHA256())
    )
    (directory / "ca.pem").write_bytes(ca.public_bytes(serialization.Encoding.PEM))
    (directory / "server.pem").write_bytes(leaf.public_bytes(serialization.Encoding.PEM))
    key_path = directory / "server.key"
    key_path.touch(mode=0o600, exist_ok=False)
    key_path.write_bytes(
        key.private_bytes(
            serialization.Encoding.PEM,
            serialization.PrivateFormat.PKCS8,
            serialization.NoEncryption(),
        ),
    )
    (directory / "empty-ca-directory").mkdir()
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(directory / "server.pem", key_path)
    return context


@dataclass(frozen=True)
class ObservedRequest:
    method: str
    path: str
    headers: dict[str, str]
    body: bytes


class Upstream:
    def __init__(self, exchanges: list[dict]) -> None:
        self.remaining = deque(exchanges)
        self.observed: list[ObservedRequest] = []
        self.errors: list[str] = []
        self.lock = threading.Lock()
        self.port = 0
        self.stalled = any(
            exchange.get("fault") in {"stalled_headers", "stalled_body"} for exchange in exchanges
        )
        self.closed_after_stall = threading.Event()

    def next_response(self, request: ObservedRequest) -> dict:
        with self.lock:
            self.observed.append(request)
            if not self.remaining:
                self.errors.append("unexpected extra request")
                return {"status": 500, "body": PRIVATE_MARKER}
            response = self.remaining.popleft()
            if (request.method, request.path) != (response["method"], response["path"]):
                self.errors.append("request method/path disagreed with the versioned fixture")
            return response

    def assert_complete(self) -> None:
        assert not self.errors, self.errors
        assert not self.remaining, "expected source wire exchanges were not executed"


class _Api(BaseHTTPRequestHandler):
    server: _Server
    protocol_version = "HTTP/1.1"

    def do_GET(self) -> None:
        self.respond()

    def do_POST(self) -> None:
        self.respond()

    def do_PUT(self) -> None:
        self.respond()

    def respond(self) -> None:
        length = int(self.headers.get("Content-Length", "0"))
        if length < 0 or length > MAX_REQUEST_BYTES:
            self.server.upstream.errors.append("fixture request exceeded its body bound")
            self.send_error(413)
            return
        request = ObservedRequest(
            self.command,
            self.path,
            {name.lower(): value for name, value in self.headers.items()},
            self.rfile.read(length),
        )
        response = self.server.upstream.next_response(request)
        fault = response.get("fault")
        self.close_connection = True
        if fault in {"stalled_headers", "stalled_body"}:
            self.stall_until_client_closes(fault)
            return
        if fault == "disconnect":
            return
        if fault == "malformed_headers":
            self.wfile.write(b"HTTP/1.1 200 OK\r\nContent-Length: invalid\r\n\r\n")
            return
        body = (
            json.dumps(response["json"], ensure_ascii=False, separators=(",", ":")).encode()
            if "json" in response
            else response.get("body", "").encode()
        )
        self.send_response(response["status"])
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Connection", "close")
        for name, value in response.get("headers", {}).items():
            self.send_header(name, value)
        if fault == "malformed_chunk" or response.get("chunked"):
            self.send_header("Transfer-Encoding", "chunked")
        else:
            self.send_header(
                "Content-Length",
                str(len(body) + (100 if fault == "truncated" else 0)),
            )
        self.end_headers()
        self.write_body(response, body)

    def write_body(self, response: dict, body: bytes) -> None:
        fault = response.get("fault")
        if fault == "malformed_chunk":
            self.wfile.write(b"not-hex\r\n" + body + b"\r\n0\r\n\r\n")
        elif response.get("chunked"):
            # Deliberately split UTF-8 sequences across raw wire chunks.
            for start in range(0, len(body), 7):
                chunk = body[start : start + 7]
                self.wfile.write(f"{len(chunk):x}\r\n".encode() + chunk + b"\r\n")
                self.wfile.flush()
            self.wfile.write(b"0\r\n\r\n")
        else:
            self.wfile.write(body)

    def stall_until_client_closes(self, fault: str) -> None:
        if fault == "stalled_body":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", "1024")
            self.end_headers()
            self.wfile.flush()
        try:
            unexpected = self.connection.recv(1)
        except ConnectionResetError:
            unexpected = b""
        if unexpected:
            self.server.upstream.errors.append("unexpected request data during source stall")
        else:
            self.server.upstream.closed_after_stall.set()

    def log_message(self, _format: str, *_args: object) -> None:
        pass


class _Tunnel(socketserver.BaseRequestHandler):
    server: _Server

    def handle(self) -> None:
        self.request.settimeout(75 if self.server.upstream.stalled else 5)
        headers = bytearray()
        while not headers.endswith(b"\r\n\r\n") and len(headers) < 8192:
            byte = self.request.recv(1)
            if not byte:
                return
            headers.extend(byte)
        if headers.split(b"\r\n", 1)[0] != f"CONNECT {HOST}:443 HTTP/1.1".encode():
            self.server.upstream.errors.append("proxy rejected an unexpected destination")
            self.request.sendall(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
            return
        self.request.sendall(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        try:
            with self.server.tls_context.wrap_socket(self.request, server_side=True) as tls:
                _Api(tls, self.client_address, self.server)
        except (BrokenPipeError, ConnectionResetError):
            # A fault can make the production client cancel before the final write.
            return


class _Server(socketserver.ThreadingTCPServer):
    def __init__(self, context: ssl.SSLContext, upstream: Upstream) -> None:
        self.tls_context = context
        self.upstream = upstream
        super().__init__(("127.0.0.1", 0), _Tunnel)

    def handle_error(self, _request: object, _client_address: object) -> None:
        # Never let a fixture worker exception disappear as console noise.
        self.upstream.errors.append("local TLS fixture worker failed")


@contextmanager
def upstream_server(directory: Path, exchanges: list[dict]) -> Iterator[Upstream]:
    """Serve bounded scripted exchanges and always stop/reap the local listener."""
    context = _certificates(directory)
    upstream = Upstream(exchanges)

    with _Server(context, upstream) as server:
        upstream.port = server.server_address[1]
        worker = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.01})
        worker.start()
        try:
            yield upstream
        finally:
            server.shutdown()
            worker.join(timeout=3)
            assert not worker.is_alive(), "local TLS fixture listener was not reaped"


@asynccontextmanager
async def upstream_session(
    binary: Path,
    directory: Path,
    upstream: Upstream,
    *,
    writes: bool = False,
) -> AsyncIterator[ClientSession]:
    proxy = f"http://127.0.0.1:{upstream.port}"
    environment = _server_environment(
        FYI_MCP_REMOTE_INSTANCES=f"synthetic=https://{HOST}",
        FYI_MCP_REMOTE_READ="synthetic",
        FYI_MCP_REMOTE_WRITE="synthetic" if writes else "",
        FYI_MCP_REMOTE_WRITE_STATE_DIR=str(directory / "write-state"),
        FYI_MCP_REMOTE_BOT_TOKEN_SYNTHETIC=SYNTHETIC_HEADER_VALUE,
        FYI_MCP_ATTACHMENT_ROOT=str(directory),
        HTTPS_PROXY=proxy,
        https_proxy=proxy,
        HTTP_PROXY=proxy,
        http_proxy=proxy,
        ALL_PROXY="",
        all_proxy="",
        NO_PROXY="",
        no_proxy="",
        SSL_CERT_FILE=str(directory / "ca.pem"),
        SSL_CERT_DIR=str(directory / "empty-ca-directory"),
        RUST_LOG="info",
    )
    server = StdioServerParameters(command=str(binary), cwd=directory, env=environment)
    with (
        anyio.fail_after(90 if upstream.stalled else 20),
        (directory / "upstream-server.stderr").open("a", encoding="utf-8") as diagnostics,
    ):
        async with (
            stdio_client(server, errlog=diagnostics) as streams,
            ClientSession(
                *streams,
                read_timeout_seconds=timedelta(seconds=70 if upstream.stalled else 5),
            ) as session,
        ):
            await session.initialize()
            yield session
