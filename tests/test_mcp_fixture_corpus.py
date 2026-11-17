"""Shared synthetic source fixtures and their executable MCP contracts."""

from __future__ import annotations

import hashlib
import json
import shutil
import socket
import sys
from pathlib import Path

import anyio
import pytest

from scripts import verify_mcp_fixture_corpus as corpus
from tests import test_mcp_client_interop
from tests.mcp_upstream_fixture import upstream_server, upstream_session

mcp_binary = test_mcp_client_interop.mcp_binary
PRIVATE_ERROR = "remote operation failed; see operator audit telemetry"
REPLAY_CASES = [case for case in corpus.load_corpus()["cases"] if case["replay"] != "none"]


def test_versioned_shared_fixture_corpus_is_present() -> None:
    root = Path(__file__).resolve().parent / "fixtures/api_contracts/v1"
    assert (root / "manifest.json").is_file(), "versioned shared source manifest is missing"
    assert (root / "manifest.schema.json").is_file()
    assert (root / "payloads.schema.json").is_file()


def test_shared_fixture_corpus_validates_without_source_io() -> None:
    assert corpus.validate_corpus() == []
    assert len(corpus.load_corpus()["cases"]) == 17


@pytest.fixture
def copied_corpus(tmp_path: Path) -> Path:
    directory = tmp_path / "corpus"
    shutil.copytree(corpus.FIXTURE_ROOT, directory)
    return directory


def _manifest(root: Path) -> dict:
    return json.loads((root / corpus.MANIFEST_PATH).read_text(encoding="utf-8"))


def _write_manifest(root: Path, manifest: dict) -> None:
    (root / corpus.MANIFEST_PATH).write_text(json.dumps(manifest), encoding="utf-8")


def _replace_fixture(root: Path, identifier: str, data: bytes) -> None:
    manifest = _manifest(root)
    case = next(case for case in manifest["cases"] if case["id"] == identifier)
    (root / case["path"]).write_bytes(data)
    case["sha256"] = hashlib.sha256(data).hexdigest()
    _write_manifest(root, manifest)


@pytest.mark.parametrize("version", [None, True, 1.0, "1", 0, 2])
def test_unknown_or_ambiguous_versions_fail_closed(copied_corpus: Path, version: object) -> None:
    manifest = _manifest(copied_corpus)
    manifest["schema_version"] = version
    _write_manifest(copied_corpus, manifest)
    assert corpus.validate_corpus(copied_corpus)


@pytest.mark.parametrize("change", ["missing", "duplicate", "unknown", "path", "provenance"])
def test_incomplete_or_misrepresented_inventory_fails_closed(
    copied_corpus: Path,
    change: str,
) -> None:
    manifest = _manifest(copied_corpus)
    if change == "missing":
        manifest["cases"].pop()
    elif change == "duplicate":
        manifest["cases"][-1] = manifest["cases"][0]
    elif change == "unknown":
        manifest["cases"][0]["contract"] = "unreviewed-contract"
    elif change == "path":
        manifest["cases"][-1]["path"] = manifest["cases"][0]["path"]
    else:
        manifest["provenance"] = "live platform certification"
    _write_manifest(copied_corpus, manifest)
    assert corpus.validate_corpus(copied_corpus)


@pytest.mark.parametrize(
    "path",
    ["../private.json", "/private.json", "a//b", "./a", "a\\b", "C:/a", ""],
)
def test_noncanonical_fixture_paths_are_rejected(copied_corpus: Path, path: str) -> None:
    manifest = _manifest(copied_corpus)
    manifest["cases"][0]["path"] = path
    _write_manifest(copied_corpus, manifest)
    assert corpus.validate_corpus(copied_corpus)


@pytest.mark.skipif(sys.platform == "win32", reason="creating symlinks requires Windows privileges")
def test_fixture_symlink_cannot_escape_corpus(copied_corpus: Path, tmp_path: Path) -> None:
    target = copied_corpus / "request-list-success.json"
    external = tmp_path / "synthetic-private-outside.json"
    external.write_bytes(target.read_bytes())
    target.unlink()
    target.symlink_to(external)
    assert corpus.validate_corpus(copied_corpus) == ["fixture path traverses a symlink"]


@pytest.mark.parametrize("fault", ["changed", "missing", "directory", "oversized"])
def test_fixture_identity_and_file_bounds_fail_closed(copied_corpus: Path, fault: str) -> None:
    fixture = copied_corpus / "request-list-success.json"
    if fault == "changed":
        fixture.write_text("[]", encoding="utf-8")
    elif fault == "oversized":
        fixture.write_bytes(b" " * (corpus.MAX_FILE_BYTES + 1))
    else:
        fixture.unlink()
        if fault == "directory":
            fixture.mkdir()
    assert corpus.validate_corpus(copied_corpus)


@pytest.mark.parametrize("data", [b'{"schema_version":1,"schema_version":1}', b'{"value":NaN}'])
def test_manifest_rejects_duplicate_keys_and_nonfinite_numbers(
    copied_corpus: Path,
    data: bytes,
) -> None:
    (copied_corpus / corpus.MANIFEST_PATH).write_bytes(data)
    assert corpus.validate_corpus(copied_corpus) == ["fixture is not strict UTF-8 JSON"]


@pytest.mark.parametrize("field", ["$ref", "$dynamicRef", "$recursiveRef"])
def test_schema_cannot_introduce_external_references(
    copied_corpus: Path,
    field: str,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def deny_network(*_args: object, **_kwargs: object) -> None:
        pytest.fail("external schema resolution attempted network I/O")

    monkeypatch.setattr(socket, "getaddrinfo", deny_network)
    monkeypatch.setattr(socket.socket, "connect", deny_network)
    path = copied_corpus / "v1/payloads.schema.json"
    schema = json.loads(path.read_text(encoding="utf-8"))
    schema["$defs"]["request"][field] = "https://never-contact.invalid/private-schema"
    path.write_text(json.dumps(schema), encoding="utf-8")
    errors = corpus.validate_corpus(copied_corpus)
    assert errors
    assert "never-contact" not in str(errors)


@pytest.mark.parametrize("identifier", ["requests-wrapped", "rss-links", "atom-links", "http-401"])
def test_changed_semantic_expectations_are_rejected(copied_corpus: Path, identifier: str) -> None:
    manifest = _manifest(copied_corpus)
    case = next(case for case in manifest["cases"] if case["id"] == identifier)
    field = "status" if identifier == "http-401" else "ids"
    case["expected"][field] = 403 if field == "status" else [9999]
    _write_manifest(copied_corpus, manifest)
    assert corpus.validate_corpus(copied_corpus)


@pytest.mark.parametrize(
    "identifier",
    ["truncated-rss", "truncated-atom", "truncated-json", "requests-missing-title"],
)
def test_invalid_fixtures_cannot_be_replaced_with_successes(
    copied_corpus: Path,
    identifier: str,
) -> None:
    replacements = {
        "truncated-rss": b'<rss version="2.0"><channel/></rss>',
        "truncated-atom": b'<feed xmlns="http://www.w3.org/2005/Atom"/>',
        "truncated-json": b'{"results":[]}',
        "requests-missing-title": b'{"requests":[]}',
    }
    _replace_fixture(copied_corpus, identifier, replacements[identifier])
    assert corpus.validate_corpus(copied_corpus)


def test_xml_entity_declarations_are_denied_even_in_a_malformed_case(copied_corpus: Path) -> None:
    _replace_fixture(
        copied_corpus,
        "truncated-rss",
        b'<!DOCTYPE rss [<!ENTITY synthetic SYSTEM "file:///synthetic-private">]><rss>&synthetic;',
    )
    errors = corpus.validate_corpus(copied_corpus)
    assert errors == ["XML corpus fixtures must not contain DTD or entity declarations"]
    assert "synthetic-private" not in str(errors)


def test_other_xml_encodings_cannot_hide_entity_declarations(copied_corpus: Path) -> None:
    payload = (
        '<?xml version="1.0" encoding="UTF-16"?>'
        '<!DOCTYPE rss [<!ENTITY synthetic SYSTEM "file:///synthetic-private">]>'
        "<rss>&synthetic;</rss>"
    )
    _replace_fixture(copied_corpus, "truncated-rss", payload.encode("utf-16"))
    assert corpus.validate_corpus(copied_corpus) == ["feed fixture is not UTF-8 XML"]


def test_invalid_feed_urls_fail_without_echoing_the_rejected_value(copied_corpus: Path) -> None:
    _replace_fixture(
        copied_corpus,
        "rss-links",
        b'<rss version="2.0"><channel><item>'
        b"<link>https://[synthetic-private/request/1001</link>"
        b"</item></channel></rss>",
    )
    assert corpus.validate_corpus(copied_corpus) == [
        "feed fixture contains a malformed request URL",
    ]


def test_cli_fails_closed_without_echoing_private_paths(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    caplog: pytest.LogCaptureFixture,
) -> None:
    monkeypatch.setattr(
        sys,
        "argv",
        ["verify_mcp_fixture_corpus.py", "--root", str(tmp_path / "synthetic-private")],
    )
    assert corpus.main() == 1
    assert "synthetic-private" not in caplog.text


def _replay_spec(case: dict) -> tuple[str, dict, dict, list[str]]:
    data = corpus.read_fixture(corpus.FIXTURE_ROOT, case["path"])
    arguments = {"instance_id": "synthetic"}
    exchange = {"method": "GET", "status": 200, "body": data.decode("utf-8")}
    private = ["synthetic-private-json-marker", "Missing the required title field"]
    if case["replay"] == "search":
        arguments.update(query="synthetic", limit=10)
        exchange["path"] = "/search.json?q=synthetic&per_page=10"
        return "remote_search_requests", arguments, exchange, private
    if case["replay"] == "authorities":
        exchange["path"] = "/api/v2/authority.json"
        return "remote_list_authorities", arguments, exchange, private
    value = json.loads(data)
    exchange.update(path="/api/v2/version.json", status=case["expected"]["status"])
    if case["contract"] == "rate-limit":
        exchange.update(body=json.dumps(value["body"]), headers=value["headers"])
        private.append(value["body"]["message"])
    else:
        private.append(value["message"])
    return "remote_version", arguments, exchange, private


def _expected_result(case: dict) -> dict:
    payload = json.loads(corpus.read_fixture(corpus.FIXTURE_ROOT, case["path"]))
    if case["replay"] == "search":
        records = (
            payload
            if isinstance(payload, list)
            else payload.get("results", payload.get("requests"))
        )
        fields = {
            "id",
            "title",
            "body",
            "user_name",
            "status",
            "created_at",
            "updated_at",
            "url",
            "tags",
        }
        return {
            "instance_id": "synthetic",
            "query": "synthetic",
            "requests": [
                {key: value for key, value in row.items() if key in fields and value is not None}
                for row in records
            ],
        }
    records = payload if isinstance(payload, list) else payload["authorities"]
    return {
        "instance_id": "synthetic",
        "authorities": [
            {key: row.get(key) for key in ("id", "slug", "name", "url")} for row in records
        ],
    }


@pytest.mark.skipif(
    sys.platform != "linux",
    reason="child-only TLS trust fixture is qualified on Linux",
)
@pytest.mark.integration
@pytest.mark.compatibility
@pytest.mark.parametrize("case", REPLAY_CASES, ids=lambda case: case["id"])
def test_shared_fixture_replay_uses_production_source_parser(
    mcp_binary: Path,
    tmp_path: Path,
    case: dict,
) -> None:
    """Replay the shared corpus itself through existing read-only SDK/source plumbing."""
    tool, arguments, exchange, private = _replay_spec(case)

    async def scenario() -> None:
        with upstream_server(tmp_path, [exchange]) as upstream:
            async with upstream_session(mcp_binary, tmp_path, upstream) as session:
                result = await session.call_tool(tool, arguments)
                if case["expected"]["outcome"] == "accepted":
                    assert not result.isError
                    assert result.structuredContent == _expected_result(case)
                else:
                    assert result.isError
                    assert result.content[0].text == PRIVATE_ERROR
                    for marker in private:
                        assert marker not in result.model_dump_json()
                await session.send_ping()
            upstream.assert_complete()
            assert len(upstream.observed) == 1
            assert upstream.observed[0].method == "GET"
            assert upstream.observed[0].body == b""
            assert "x-fyi-bot-token" not in upstream.observed[0].headers

    anyio.run(scenario)
    diagnostics = (tmp_path / "upstream-server.stderr").read_text(encoding="utf-8")
    for marker in private:
        assert marker not in diagnostics
