"""Offline parity and fail-closed tests for Internet Archive CDX discovery."""

from __future__ import annotations

import json
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

import httpx
import pytest

from fyi_system.acquisition_receipts import AcquisitionRecorder, canonical_json_bytes
from fyi_system.internet_archive_cdx import (
    CDX_ENDPOINT,
    MAX_RESPONSE_BYTES,
    CdxConfig,
    CdxDiscoveryError,
    default_transport,
    discover_cdx,
)

FIXTURES = Path(__file__).parent / "fixtures" / "internet_archive_cdx"


def _fixture(name: str) -> bytes:
    return (FIXTURES / name).read_bytes()


def _response(url: str, payload: bytes, status: int = 200, **headers: str) -> httpx.Response:
    request = httpx.Request("GET", url)
    return httpx.Response(status, content=payload, headers=headers, request=request)


def _config(mode: str = "resume_key", **overrides: object) -> CdxConfig:
    values: dict[str, object] = {
        "url_pattern": "example.test/request/*",
        "allowed_host": "example.test",
        "pagination_mode": mode,
        "capture_mode": "url_index",
        "page_size": 1,
        "max_pages": 3,
        "max_rows": 10,
        "max_runtime_seconds": 60.0,
    }
    values.update(overrides)
    return CdxConfig(**values)  # type: ignore[arg-type]


def test_page_count_mode_matches_fyi_archive_fixture(tmp_path: Path) -> None:
    seen: list[str] = []

    def transport(url: str, _timeout: float) -> httpx.Response:
        seen.append(url)
        query = parse_qs(urlsplit(url).query)
        name = (
            "page-count.json"
            if "showNumPages" in query
            else f"page-{int(query['page'][0]):06d}.json"
        )
        return _response(url, _fixture(name))

    result = discover_cdx(
        _config("page_count"),
        output_path=tmp_path / "cdx.json",
        checkpoint_path=tmp_path / "checkpoint.json",
        transport=transport,
    )

    assert result == json.loads(_fixture("expected.json"))
    assert (tmp_path / "cdx.json").read_bytes() == canonical_json_bytes(result)
    assert len(seen) == 3


def test_resume_key_mode_matches_fixture_and_reuses_complete_checkpoint(tmp_path: Path) -> None:
    calls = 0

    def transport(url: str, _timeout: float) -> httpx.Response:
        nonlocal calls
        query = parse_qs(urlsplit(url).query)
        fixture = "chunk-000001.json" if "resumeKey" in query else "chunk-000000.json"
        calls += 1
        return _response(url, _fixture(fixture))

    paths = {"output_path": tmp_path / "cdx.json", "checkpoint_path": tmp_path / "state.json"}
    result = discover_cdx(_config(), transport=transport, **paths)
    assert result == json.loads(_fixture("expected.json"))
    assert calls == 2

    result = discover_cdx(
        _config(),
        transport=lambda *_: pytest.fail("complete checkpoint must avoid network"),
        **paths,
    )
    assert result == json.loads(_fixture("expected.json"))
    assert calls == 2


def test_checkpoint_resumes_at_verified_cursor(tmp_path: Path) -> None:
    checkpoint = tmp_path / "state.json"
    output = tmp_path / "cdx.json"

    def interrupted(url: str, _timeout: float) -> httpx.Response:
        if "resumeKey=" not in url:
            return _response(url, _fixture("chunk-000000.json"))
        message = "offline"
        raise httpx.ConnectError(message, request=httpx.Request("GET", url))

    with pytest.raises(CdxDiscoveryError, match="bounded retries"):
        discover_cdx(
            _config(),
            output_path=output,
            checkpoint_path=checkpoint,
            transport=interrupted,
            sleep=lambda _seconds: None,
        )
    assert not output.exists()

    seen: list[str] = []

    def resumed(url: str, _timeout: float) -> httpx.Response:
        seen.append(url)
        return _response(url, _fixture("chunk-000001.json"))

    result = discover_cdx(
        _config(),
        output_path=output,
        checkpoint_path=checkpoint,
        transport=resumed,
    )
    assert result == json.loads(_fixture("expected.json"))
    assert len(seen) == 1
    assert "resumeKey=next%2521" in seen[0]


def test_resume_can_increase_operational_caps_without_changing_query(tmp_path: Path) -> None:
    checkpoint = tmp_path / "state.json"

    with pytest.raises(CdxDiscoveryError, match="chunk cap"):
        discover_cdx(
            _config(max_pages=1),
            output_path=tmp_path / "cdx.json",
            checkpoint_path=checkpoint,
            transport=lambda url, _: _response(url, _fixture("chunk-000000.json")),
        )

    result = discover_cdx(
        _config(max_pages=3, max_rows=20),
        output_path=tmp_path / "cdx.json",
        checkpoint_path=checkpoint,
        transport=lambda url, _: _response(url, _fixture("chunk-000001.json")),
    )
    assert result == json.loads(_fixture("expected.json"))


def test_checkpoint_tampering_and_configuration_drift_fail_closed(tmp_path: Path) -> None:
    checkpoint = tmp_path / "state.json"
    discover_cdx(
        _config(),
        output_path=tmp_path / "cdx.json",
        checkpoint_path=checkpoint,
        transport=lambda url, _: _response(url, _fixture("chunk-000001.json")),
    )
    value = json.loads(checkpoint.read_text(encoding="utf-8"))
    value["rows"][0][0] = "tampered"
    checkpoint.write_text(json.dumps(value), encoding="utf-8")
    with pytest.raises(ValueError, match="self-digest"):
        discover_cdx(
            _config(),
            output_path=tmp_path / "other.json",
            checkpoint_path=checkpoint,
            transport=lambda *_: pytest.fail("tampered checkpoint must avoid network"),
        )

    clean_checkpoint = tmp_path / "clean-state.json"
    discover_cdx(
        _config(),
        output_path=tmp_path / "clean.json",
        checkpoint_path=clean_checkpoint,
        transport=lambda url, _: _response(url, _fixture("chunk-000001.json")),
    )
    with pytest.raises(ValueError, match="configuration"):
        discover_cdx(
            _config(page_size=2),
            output_path=tmp_path / "drift.json",
            checkpoint_path=clean_checkpoint,
            transport=lambda *_: pytest.fail("configuration drift must avoid network"),
        )


@pytest.mark.parametrize(
    ("pattern", "host"),
    [
        ("https://example.test/*", "example.test"),
        ("other.test/*", "example.test"),
        ("example.test:443/*", "example.test"),
        ("example.test/%2e%2e/*", "example.test"),
        ("example.test/path?url=other.test", "example.test"),
    ],
)
def test_source_patterns_cannot_escape_the_host_boundary(pattern: str, host: str) -> None:
    with pytest.raises(ValueError, match="url_pattern"):
        _config(url_pattern=pattern, allowed_host=host)


@pytest.mark.parametrize(
    "overrides",
    [
        {"page_size": 0},
        {"max_pages": 0},
        {"max_rows": 0},
        {"max_runtime_seconds": 0},
        {"max_stall_seconds": 0},
        {"from_timestamp": "2025", "to_timestamp": "2024"},
    ],
)
def test_invalid_bounds_are_rejected_before_network(overrides: dict[str, object]) -> None:
    with pytest.raises(ValueError, match="must"):
        _config(**overrides)


def test_repeated_cursor_and_chunk_fail_closed(tmp_path: Path) -> None:
    calls = 0

    def repeated_cursor(url: str, _timeout: float) -> httpx.Response:
        nonlocal calls
        calls += 1
        payload = canonical_json_bytes(
            [
                ["original"],
                [f"row-{calls}"],
                [],
                ["cursor"],
            ],
        )
        return _response(url, payload)

    with pytest.raises(CdxDiscoveryError, match="resumption key repeated"):
        discover_cdx(
            _config(),
            output_path=tmp_path / "cdx.json",
            checkpoint_path=tmp_path / "state.json",
            transport=repeated_cursor,
        )


def test_response_redirect_or_query_drift_is_rejected(tmp_path: Path) -> None:
    def escaped(_url: str, _timeout: float) -> httpx.Response:
        return _response("https://example.test/cdx", b"[]")

    with pytest.raises(CdxDiscoveryError, match="escaped"):
        discover_cdx(
            _config(),
            output_path=tmp_path / "cdx.json",
            checkpoint_path=tmp_path / "state.json",
            transport=escaped,
        )


def test_default_transport_stops_streaming_at_response_byte_cap(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    request = httpx.Request("GET", CDX_ENDPOINT)
    response = httpx.Response(
        200,
        request=request,
        stream=httpx.ByteStream(b"x" * MAX_RESPONSE_BYTES + b"y"),
    )

    class ResponseContext:
        def __enter__(self) -> httpx.Response:
            return response

        def __exit__(self, *_args: object) -> None:
            response.close()

    monkeypatch.setattr(httpx, "stream", lambda *_args, **_kwargs: ResponseContext())

    with pytest.raises(CdxDiscoveryError, match="byte cap"):
        default_transport(CDX_ENDPOINT, 10.0)


def test_retry_after_and_response_digest_flow_into_receipt(tmp_path: Path) -> None:
    attempts = 0
    sleeps: list[float] = []
    recorder = AcquisitionRecorder(
        command="internet-archive-cdx",
        adapter_id="internet-archive-cdx",
        source_url=CDX_ENDPOINT,
        request_bounds=_config().request_bounds(),
        checkpoint_path=tmp_path / "state.json",
    )

    def transport(url: str, _timeout: float) -> httpx.Response:
        nonlocal attempts
        attempts += 1
        if attempts == 1:
            return _response(url, b"busy", 503, **{"Retry-After": "7"})
        return _response(url, _fixture("chunk-000001.json"))

    rows = discover_cdx(
        _config(),
        output_path=tmp_path / "cdx.json",
        checkpoint_path=tmp_path / "state.json",
        transport=transport,
        observer=recorder.observe_response,
        sleep=sleeps.append,
    )
    receipt = recorder.build(
        result_projection=canonical_json_bytes(rows),
        result_media_type="application/json",
    )
    assert sleeps == [7.0]
    assert receipt["totals"] == {
        "requests": 1,
        "bytes": len(_fixture("chunk-000001.json")),
        "retries": 1,
    }
    assert receipt["responses"][0]["sha256"]
    assert receipt["checkpoint"]["after_sha256"]


def test_failure_preserves_prior_output_and_writes_partial_checkpoint(tmp_path: Path) -> None:
    output = tmp_path / "cdx.json"
    output.write_bytes(b"prior-valid-output\n")

    def transport(url: str, _timeout: float) -> httpx.Response:
        if "resumeKey=" not in url:
            return _response(url, _fixture("chunk-000000.json"))
        return _response(url, b'[["wrong-header"],["value"]]')

    with pytest.raises(CdxDiscoveryError, match="header changed"):
        discover_cdx(
            _config(),
            output_path=output,
            checkpoint_path=tmp_path / "state.json",
            transport=transport,
        )
    assert output.read_bytes() == b"prior-valid-output\n"
    assert json.loads((tmp_path / "state.json").read_text())["next_index"] == 1


def test_page_count_change_is_rejected_on_resume(tmp_path: Path) -> None:
    checkpoint = tmp_path / "state.json"
    calls = 0

    def first(url: str, _timeout: float) -> httpx.Response:
        nonlocal calls
        calls += 1
        if "showNumPages" in url:
            return _response(url, _fixture("page-count.json"))
        if "page=0" in url:
            return _response(url, _fixture("page-000000.json"))
        message = "offline"
        raise httpx.ConnectError(message, request=httpx.Request("GET", url))

    with pytest.raises(CdxDiscoveryError):
        discover_cdx(
            _config("page_count"),
            output_path=tmp_path / "cdx.json",
            checkpoint_path=checkpoint,
            transport=first,
            sleep=lambda _seconds: None,
        )

    changed_count = canonical_json_bytes([["blocks"], ["3"]])
    with pytest.raises(CdxDiscoveryError, match="page count changed"):
        discover_cdx(
            _config("page_count"),
            output_path=tmp_path / "cdx.json",
            checkpoint_path=checkpoint,
            transport=lambda url, _: _response(url, changed_count),
        )
