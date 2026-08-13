"""Contract tests for versioned acquisition receipts."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import httpx
import pytest
import requests

from fyi_system.acquisition_receipts import (
    AcquisitionRecorder,
    canonical_json_bytes,
    sanitize_url,
    sha256_bytes,
    validate_receipt,
    write_receipt_atomic,
)
from fyi_system.cli import build_parser, cmd_fetch_request_page
from fyi_system.db import init_db
from fyi_system.fetch import fetch_request_page

if TYPE_CHECKING:
    from pathlib import Path


def recorder(tmp_path: Path) -> AcquisitionRecorder:
    checkpoint = tmp_path / "checkpoint.json"
    checkpoint.write_text('{"next_page":2}\n', encoding="utf-8")
    return AcquisitionRecorder(
        command="discover",
        adapter_id="alaveteli-search-feed",
        adapter_version="test-adapter-v1",
        source_url="https://user:secret@example.test/search?token=secret&page=2#fragment",
        request_bounds={"page_from": 2, "max_pages": 1},
        rate_limit_name="archive-discovery-test",
        minimum_interval_seconds=1.0,
        checkpoint_path=checkpoint,
        started_at="2026-08-13T00:00:00Z",
    )


def test_receipt_is_schema_valid_canonical_and_credential_safe(tmp_path: Path) -> None:
    item = recorder(tmp_path)
    request = httpx.Request("GET", "https://example.test/request/1?api_key=secret")
    response = httpx.Response(200, content=b"payload", request=request)
    response.extensions["fyi_attempts"] = 2
    response.extensions["fyi_retry_delays_seconds"] = [1.5]
    item.observe_response(response)

    receipt = item.build(
        result_projection=b'{"request_id":1}\n',
        result_media_type="application/x-ndjson",
        completed_at="2026-08-13T00:00:01Z",
    )
    validate_receipt(receipt)

    assert receipt["source"]["url"] == "https://example.test/search?REDACTED"
    assert receipt["responses"][0]["url"] == "https://example.test/request/1?REDACTED"
    assert receipt["totals"] == {"requests": 1, "bytes": 7, "retries": 1}
    assert b"secret" not in canonical_json_bytes(receipt)


def test_canonical_output_is_deterministic(tmp_path: Path) -> None:
    item = recorder(tmp_path)
    first = item.build(
        result_projection=b"[]\n",
        result_media_type="application/json",
        completed_at="2026-08-13T00:00:01Z",
    )
    second = item.build(
        result_projection=b"[]\n",
        result_media_type="application/json",
        completed_at="2026-08-13T00:00:01Z",
    )
    assert canonical_json_bytes(first) == canonical_json_bytes(second)


def test_invalid_receipt_does_not_replace_previous_receipt(tmp_path: Path) -> None:
    path = tmp_path / "receipt.json"
    item = recorder(tmp_path)
    valid = item.build(
        result_projection=b"[]\n",
        result_media_type="application/json",
        completed_at="2026-08-13T00:00:01Z",
    )
    write_receipt_atomic(path, valid)
    original = path.read_bytes()
    invalid = dict(valid)
    invalid["command"] = ""

    with pytest.raises(ValueError, match="invalid acquisition receipt"):
        write_receipt_atomic(path, invalid)

    assert path.read_bytes() == original
    assert not list(tmp_path.glob("*.tmp"))


def test_tampered_self_digest_fails_closed(tmp_path: Path) -> None:
    receipt = recorder(tmp_path).build(
        result_projection=b"[]\n",
        result_media_type="application/json",
        completed_at="2026-08-13T00:00:01Z",
    )
    receipt["request_bounds"] = {"page_from": 999}
    with pytest.raises(ValueError, match="receipt_sha256 mismatch"):
        validate_receipt(receipt)


@pytest.mark.parametrize(
    "value",
    ["file:///tmp/source", "not-a-url", "https:///missing-host"],
)
def test_non_http_sources_are_rejected(value: str) -> None:
    with pytest.raises(ValueError, match="absolute HTTP"):
        sanitize_url(value)


def test_query_parameter_name_and_value_are_wholly_redacted() -> None:
    source = "https://example.test/path?secret-token-name=secret-value&ordinary=1#fragment"
    sanitized = sanitize_url(source)
    assert sanitized == "https://example.test/path?REDACTED"
    assert "secret-token-name" not in sanitized
    assert "ordinary" not in sanitized


def test_written_receipt_has_one_canonical_newline(tmp_path: Path) -> None:
    path = tmp_path / "receipt.json"
    item = recorder(tmp_path)
    item.write(path, result_projection=b"{}\n", result_media_type="application/json")
    raw = path.read_bytes()
    assert raw.endswith(b"\n")
    assert not raw.endswith(b"\n\n")
    validate_receipt(json.loads(raw))


def test_failed_receipt_is_atomic_valid_and_contains_exception_type_only(tmp_path: Path) -> None:
    path = tmp_path / "receipt.json"
    item = recorder(tmp_path)
    receipt = item.write_failure(path, failure_type="HTTPStatusError")

    validate_receipt(receipt)
    assert receipt["status"] == "failed"
    assert receipt["failure"] == {"type": "HTTPStatusError"}
    assert receipt["result"]["representation"] == "canonical_result_projection"
    assert b"secret" not in path.read_bytes()
    assert not list(tmp_path.glob(f".{path.name}.*.tmp"))


def test_failing_http_status_emits_failed_receipt_and_reraises(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    receipt_path = tmp_path / "failed-receipt.json"

    class Response:
        url = "https://fyi.example/request/7.json?credential-name=credential-value"
        status_code = 503
        content = b"service unavailable"

        def raise_for_status(self) -> None:
            message = "credential-value must not enter receipt"
            raise requests.HTTPError(message)

    monkeypatch.setattr("fyi_system.fetch.requests.get", lambda *_args, **_kwargs: Response())
    args = build_parser().parse_args(
        [
            "fetch-request-page",
            "7",
            "--base-url",
            "https://fyi.example",
            "--db",
            str(tmp_path / "unused.db"),
            "--receipt",
            str(receipt_path),
        ],
    )

    with pytest.raises(requests.HTTPError, match="credential-value"):
        cmd_fetch_request_page(args)

    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    validate_receipt(receipt)
    assert receipt["status"] == "failed"
    assert receipt["failure"] == {"type": "HTTPError"}
    assert receipt["responses"][0]["status"] == 503
    assert receipt["responses"][0]["url"] == "https://fyi.example/request/7.json?REDACTED"
    assert "credential" not in receipt_path.read_text(encoding="utf-8")
    assert not list(tmp_path.glob(f".{receipt_path.name}.*.tmp"))


def test_legacy_request_fetch_contributes_response_evidence(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class Response:
        url = "https://fyi.example/request/7.json?token=secret"
        status_code = 200
        content = b'{"id":7}'

        def raise_for_status(self) -> None:
            return None

        def json(self) -> dict[str, int]:
            return {"id": 7}

    monkeypatch.setattr("fyi_system.fetch.requests.get", lambda *_args, **_kwargs: Response())
    db = tmp_path / "fyi.db"
    init_db(db)
    item = AcquisitionRecorder(
        command="fetch-request-page",
        adapter_id="alaveteli-request-json",
        adapter_version="test",
        source_url="https://fyi.example/request/7.json",
        request_bounds={"request_id": 7},
        started_at="2026-08-13T00:00:00Z",
    )

    fetch_request_page(7, base_url="https://fyi.example", db_path=db, recorder=item)

    assert item.responses[0]["url"] == "https://fyi.example/request/7.json?REDACTED"
    assert item.responses[0]["sha256"] == sha256_bytes(Response.content)
