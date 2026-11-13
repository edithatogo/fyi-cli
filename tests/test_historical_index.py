"""Offline allowlist, bounded retrieval, and receipt tests for historical index adapter."""

from __future__ import annotations

import json
from pathlib import Path

import httpx
import pytest

from fyi_system.acquisition_receipts import (
    AcquisitionRecorder,
    canonical_json_bytes,
    sha256_bytes,
    validate_receipt,
)
from fyi_system.historical_index import (
    ALLOWLISTED_HOSTS,
    HistoricalIndexConfig,
    HistoricalIndexError,
    is_allowlisted,
    retrieve_historical_index,
    validate_host,
)


def _response(url: str, payload: bytes, status: int = 200, **headers: str) -> httpx.Response:
    return httpx.Response(
        status,
        content=payload,
        headers=headers,
        request=httpx.Request("GET", url),
    )


def _config(**overrides: object) -> HistoricalIndexConfig:
    values: dict[str, object] = {
        "base_url": "https://morph.io",
        "index_path": "api/v1/requests",
        "query": {},
        "page_size": 2,
        "max_pages": 3,
        "max_rows": 100,
        "max_runtime_seconds": 60.0,
    }
    values.update(overrides)
    return HistoricalIndexConfig(**values)  # type: ignore[arg-type]


def test_allowlist_rejects_non_approved_hosts() -> None:
    """Only morph.io hosts may be queried; every other host is rejected."""
    assert is_allowlisted("morph.io")
    assert is_allowlisted("www.morph.io")
    assert not is_allowlisted("evil.test")
    assert not is_allowlisted("morph.io.evil.test")
    assert not is_allowlisted("localhost")
    assert not is_allowlisted("web.archive.org")


def test_validate_host_rejects_invalid_hosts() -> None:
    """Invalid hostnames are rejected before any network call."""
    with pytest.raises(ValueError):
        validate_host("localhost")
    with pytest.raises(ValueError):
        validate_host("not a host")
    assert validate_host("morph.io") == "morph.io"


def test_config_rejects_non_allowlisted_base_url() -> None:
    """HistoricalIndexConfig must fail before network for non-allowlisted hosts."""
    with pytest.raises(ValueError, match="not on the allowlist"):
        _config(base_url="https://evil.test/index")


def test_config_rejects_non_https_base_url() -> None:
    """base_url must be simple https:// (no credentials, ports)."""
    with pytest.raises(ValueError):
        _config(base_url="http://morph.io/index")
    with pytest.raises(ValueError):
        _config(base_url="https://user:pass@morph.io/index")
    with pytest.raises(ValueError):
        _config(base_url="https://morph.io:8443/index")


def test_config_rejects_invalid_bounds() -> None:
    """Invalid paging/row/runtime bounds are rejected before network."""
    with pytest.raises(ValueError):
        _config(page_size=0)
    with pytest.raises(ValueError):
        _config(max_pages=0)
    with pytest.raises(ValueError):
        _config(max_rows=0)
    with pytest.raises(ValueError):
        _config(max_runtime_seconds=0)


def test_retrieve_bounded_mock_index(tmp_path: Path) -> None:
    """A successful two-page retrieval returns all rows and writes output/checkpoint."""
    pages: list[str] = []

    def transport(url: str, _timeout: float) -> httpx.Response:
        pages.append(url)
        if "cursor=" not in url:
            payload = {
                "rows": [{"id": 1}, {"id": 2}],
                "next_cursor": "page-2",
            }
        else:
            payload = {
                "rows": [{"id": 3}],
                "next_cursor": None,
            }
        return _response(url, canonical_json_bytes(payload))

    output = tmp_path / "output.json"
    checkpoint = tmp_path / "checkpoint.json"
    result = retrieve_historical_index(
        _config(),
        output_path=output,
        checkpoint_path=checkpoint,
        transport=transport,
    )

    assert len(pages) == 2
    assert result[0]["rows"] == [{"id": 1}, {"id": 2}, {"id": 3}]
    assert result[0]["pages_fetched"] == 2
    assert result[0]["complete"] is True
    assert json.loads(checkpoint.read_text())["next_cursor"] is None


def test_retrieve_resumes_from_checkpoint(tmp_path: Path) -> None:
    """A complete checkpoint avoids re-fetching; an incomplete one resumes at cursor."""
    checkpoint = tmp_path / "checkpoint.json"
    output = tmp_path / "output.json"

    # First run: page 1 only (incomplete checkpoint).
    calls = 0

    def transport_page1(url: str, _timeout: float) -> httpx.Response:
        nonlocal calls
        calls += 1
        if "cursor=" not in url:
            return _response(url, canonical_json_bytes({"rows": [{"id": 1}], "next_cursor": "c2"}))
        return _response(url, canonical_json_bytes({"rows": [{"id": 2}], "next_cursor": None}))

    result = retrieve_historical_index(
        _config(max_pages=1),
        output_path=output,
        checkpoint_path=checkpoint,
        transport=transport_page1,
    )
    assert result[0]["rows"] == [{"id": 1}]
    assert calls == 1
    state = json.loads(checkpoint.read_text())
    assert state["next_cursor"] == "c2"
    assert state["complete"] is False

    # Second run: resume from cursor c2.
    calls2 = 0

    def transport_resume(url: str, _timeout: float) -> httpx.Response:
        nonlocal calls2
        calls2 += 1
        assert "cursor=c2" in url
        return _response(url, canonical_json_bytes({"rows": [{"id": 2}], "next_cursor": None}))

    result2 = retrieve_historical_index(
        _config(),
        output_path=tmp_path / "output2.json",
        checkpoint_path=checkpoint,
        transport=transport_resume,
    )
    assert result2[0]["rows"] == [{"id": 2}]
    assert result2[0]["complete"] is True
    assert calls2 == 1


def test_checkpoint_tampering_fails_closed(tmp_path: Path) -> None:
    """Tampered checkpoint digest must block re-acquisition without network."""
    checkpoint = tmp_path / "checkpoint.json"
    output = tmp_path / "output.json"

    def transport(url: str, _timeout: float) -> httpx.Response:
        return _response(url, canonical_json_bytes({"rows": [{"id": 1}], "next_cursor": None}))

    retrieve_historical_index(
        _config(max_pages=1),
        output_path=output,
        checkpoint_path=checkpoint,
        transport=transport,
    )

    value = json.loads(checkpoint.read_text())
    value["rows_collected"] = 999
    checkpoint.write_text(json.dumps(value), encoding="utf-8")

    with pytest.raises(ValueError, match="self-digest"):
        retrieve_historical_index(
            _config(),
            output_path=tmp_path / "output2.json",
            checkpoint_path=checkpoint,
            transport=lambda *_: pytest.fail("tampered checkpoint must avoid network"),
        )


def test_host_escape_is_rejected(tmp_path: Path) -> None:
    """A response whose URL escapes the https boundary is rejected fail-closed."""
    checkpoint = tmp_path / "checkpoint.json"
    output = tmp_path / "output.json"

    def escaped(url: str, _timeout: float) -> httpx.Response:
        return _response("http://evil.test/index", b"[]")

    with pytest.raises(HistoricalIndexError, match="escaped"):
        retrieve_historical_index(
            _config(),
            output_path=output,
            checkpoint_path=checkpoint,
            transport=escaped,
        )


def test_byte_cap_is_enforced(tmp_path: Path) -> None:
    """Responses exceeding MAX_RESPONSE_BYTES are rejected fail-closed."""
    checkpoint = tmp_path / "checkpoint.json"
    output = tmp_path / "output.json"
    oversized = b"x" * (17 * 1024 * 1024)

    def oversized_response(url: str, _timeout: float) -> httpx.Response:
        return _response(url, oversized)

    with pytest.raises(HistoricalIndexError, match="byte cap"):
        retrieve_historical_index(
            _config(),
            output_path=output,
            checkpoint_path=checkpoint,
            transport=oversized_response,
        )


def test_receipt_evidence_flows_to_observer(tmp_path: Path) -> None:
    """Retrieved responses feed the receipt observer with status, bytes, digest, retries."""
    recorder = AcquisitionRecorder(
        command="historical-index",
        adapter_id="historical-index",
        source_url="https://morph.io/api/v1/requests",
        request_bounds=_config().request_bounds(),
        checkpoint_path=tmp_path / "state.json",
    )

    calls = 0

    def transport(url: str, _timeout: float) -> httpx.Response:
        nonlocal calls
        calls += 1
        if calls == 1:
            return _response(url, b"busy", 503, **{"Retry-After": "3"})
        return _response(url, canonical_json_bytes({"rows": [{"id": 1}], "next_cursor": None}))

    retrieve_historical_index(
        _config(),
        output_path=tmp_path / "output.json",
        checkpoint_path=tmp_path / "checkpoint.json",
        transport=transport,
        observer=recorder.observe_response,
    )

    receipt = recorder.build(
        result_projection=canonical_json_bytes(
            {"rows": [{"id": 1}], "pages_fetched": 2, "complete": True},
        ),
        result_media_type="application/json",
    )
    validate_receipt(receipt)
    assert receipt["totals"]["retries"] == 1
    assert receipt["responses"][0]["status"] == 200
