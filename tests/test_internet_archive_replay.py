"""Offline parity and adversarial tests for bounded Wayback replay."""

from __future__ import annotations

import json
from pathlib import Path

import httpx
import pytest

import fyi_system.cli as cli_module
from fyi_system.acquisition_receipts import AcquisitionRecorder, canonical_json_bytes
from fyi_system.cli import build_parser
from fyi_system.internet_archive_replay import (
    MAX_PAYLOAD_BYTES,
    REPLAY_ADAPTER_ID,
    ReplayConfig,
    ReplayError,
    default_transport,
    replay_approved_rows,
    seal_selection,
)

FIXTURES = Path(__file__).parent / "fixtures" / "internet_archive_replay"


class _ResponseContext:
    def __init__(self, response: httpx.Response) -> None:
        self.response = response

    def __enter__(self) -> httpx.Response:
        return self.response

    def __exit__(self, *_args: object) -> None:
        self.response.close()


def _payload() -> bytes:
    return (FIXTURES / "payload.json").read_bytes()


def _selection() -> dict[str, object]:
    value = json.loads((FIXTURES / "approved-selection.json").read_text(encoding="utf-8"))
    assert isinstance(value, dict)
    return value


def _response(url: str, payload: bytes, status: int = 200, **headers: str) -> httpx.Response:
    return httpx.Response(
        status,
        content=payload,
        headers=headers,
        request=httpx.Request("GET", url),
    )


def _config(**overrides: object) -> ReplayConfig:
    values: dict[str, object] = {
        "allowed_target_host": "example.test",
        "max_rows": 2,
        "max_payload_bytes": 1024,
        "max_redirects": 2,
        "max_runtime_seconds": 30.0,
        "request_timeout_seconds": 5.0,
    }
    values.update(overrides)
    return ReplayConfig(**values)  # type: ignore[arg-type]


def test_replay_matches_frozen_projection_and_resume_avoids_network(tmp_path: Path) -> None:
    calls: list[str] = []

    def transport(url: str, _timeout: float, _cap: int) -> httpx.Response:
        calls.append(url)
        return _response(url, _payload(), **{"Content-Type": "application/json"})

    output = tmp_path / "objects"
    checkpoint = tmp_path / "checkpoint.json"
    result = replay_approved_rows(
        _selection(),
        _config(),
        output_dir=output,
        checkpoint_path=checkpoint,
        transport=transport,
    )

    expected = json.loads((FIXTURES / "expected-result.json").read_text(encoding="utf-8"))
    assert result == expected
    assert (output / result["objects"][0]["path"]).read_bytes() == _payload()
    assert len(calls) == 1

    resumed = replay_approved_rows(
        _selection(),
        _config(),
        output_dir=output,
        checkpoint_path=checkpoint,
        transport=lambda *_: pytest.fail("complete checkpoint must avoid network"),
    )
    assert resumed == result


def test_same_host_redirect_is_followed_only_for_the_exact_target(tmp_path: Path) -> None:
    calls = 0

    def transport(url: str, _timeout: float, _cap: int) -> httpx.Response:
        nonlocal calls
        calls += 1
        if calls == 1:
            redirected = url.replace("20250101000000id_", "20250102000000id_")
            return _response(url, b"", 302, Location=redirected)
        return _response(url, _payload(), **{"Content-Type": "application/json"})

    result = replay_approved_rows(
        _selection(),
        _config(),
        output_dir=tmp_path / "objects",
        checkpoint_path=tmp_path / "checkpoint.json",
        transport=transport,
    )
    assert calls == 2
    assert "20250102000000id_" in result["objects"][0]["final_url"]


@pytest.mark.parametrize(
    "location",
    [
        "https://evil.test/web/20250101000000id_/https://example.test/request/1.json",
        "http://web.archive.org/web/20250101000000id_/https://example.test/request/1.json",
        "https://web.archive.org:444/web/20250101000000id_/https://example.test/request/1.json",
        "https://web.archive.org/web/20250101000000id_/https://other.test/request/1.json",
        "//evil.test/escape",
    ],
)
def test_redirect_escape_fails_without_advancing_checkpoint(
    tmp_path: Path,
    location: str,
) -> None:
    checkpoint = tmp_path / "checkpoint.json"
    with pytest.raises(ReplayError, match="redirect"):
        replay_approved_rows(
            _selection(),
            _config(),
            output_dir=tmp_path / "objects",
            checkpoint_path=checkpoint,
            transport=lambda url, *_: _response(url, b"", 302, Location=location),
        )
    assert not checkpoint.exists()
    assert not list((tmp_path / "objects").glob("*.raw"))


def test_target_origin_must_match_explicit_allowlist_before_network(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="target host"):
        replay_approved_rows(
            _selection(),
            _config(allowed_target_host="other.test"),
            output_dir=tmp_path / "objects",
            checkpoint_path=tmp_path / "checkpoint.json",
            transport=lambda *_: pytest.fail("invalid selection must avoid network"),
        )


@pytest.mark.parametrize(
    ("status", "payload", "message"),
    [
        (404, _payload(), "status"),
        (200, b"{}\n", "length"),
        (200, b"x" * len(_payload()), "digest"),
    ],
)
def test_status_length_and_digest_mismatch_fail_closed(
    tmp_path: Path,
    status: int,
    payload: bytes,
    message: str,
) -> None:
    output = tmp_path / "objects"
    checkpoint = tmp_path / "checkpoint.json"
    with pytest.raises(ReplayError, match=message):
        replay_approved_rows(
            _selection(),
            _config(),
            output_dir=output,
            checkpoint_path=checkpoint,
            transport=lambda url, *_: _response(
                url,
                payload,
                status,
                **{"Content-Type": "application/json"},
            ),
        )
    assert not checkpoint.exists()
    assert not list(output.glob("*.raw"))


def test_tampered_selection_self_digest_is_rejected(tmp_path: Path) -> None:
    selection = _selection()
    rows = selection["rows"]
    assert isinstance(rows, list)
    assert isinstance(rows[0], dict)
    rows[0]["expected_payload_bytes"] = 1
    with pytest.raises(ValueError, match="selection_sha256"):
        replay_approved_rows(
            selection,
            _config(),
            output_dir=tmp_path / "objects",
            checkpoint_path=tmp_path / "checkpoint.json",
            transport=lambda *_: pytest.fail("tampering must avoid network"),
        )


def test_checkpoint_tampering_and_configuration_drift_fail_closed(tmp_path: Path) -> None:
    checkpoint = tmp_path / "checkpoint.json"
    output = tmp_path / "objects"
    replay_approved_rows(
        _selection(),
        _config(),
        output_dir=output,
        checkpoint_path=checkpoint,
        transport=lambda url, *_: _response(
            url,
            _payload(),
            **{"Content-Type": "application/json"},
        ),
    )
    value = json.loads(checkpoint.read_text(encoding="utf-8"))
    value["completed"][0]["bytes"] = 1
    checkpoint.write_bytes(canonical_json_bytes(value))
    with pytest.raises(ValueError, match="self-digest"):
        replay_approved_rows(
            _selection(),
            _config(),
            output_dir=output,
            checkpoint_path=checkpoint,
            transport=lambda *_: pytest.fail("bad checkpoint must avoid network"),
        )


def test_receipt_records_redirect_and_verified_payload(tmp_path: Path) -> None:
    config = _config()
    checkpoint = tmp_path / "checkpoint.json"
    recorder = AcquisitionRecorder(
        command="internet-archive-replay",
        adapter_id=REPLAY_ADAPTER_ID,
        source_url="https://web.archive.org/",
        request_bounds=config.request_bounds(_selection()),
        checkpoint_path=checkpoint,
    )
    result = replay_approved_rows(
        _selection(),
        config,
        output_dir=tmp_path / "objects",
        checkpoint_path=checkpoint,
        transport=lambda url, *_: _response(
            url,
            _payload(),
            **{"Content-Type": "application/json"},
        ),
        observer=recorder.observe_response,
    )
    receipt = recorder.build(
        result_projection=canonical_json_bytes(result),
        result_media_type="application/json",
    )
    assert receipt["totals"] == {"requests": 1, "bytes": len(_payload()), "retries": 0}
    assert receipt["responses"][0]["sha256"] == result["objects"][0]["sha256"]
    assert receipt["checkpoint"]["after_sha256"]


def test_default_transport_enforces_declared_and_streamed_byte_caps(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    request = httpx.Request("GET", "https://web.archive.org/")

    for response in (
        httpx.Response(
            200,
            headers={"Content-Length": str(MAX_PAYLOAD_BYTES + 1)},
            request=request,
        ),
        httpx.Response(
            200,
            request=request,
            stream=httpx.ByteStream(b"x" * (MAX_PAYLOAD_BYTES + 1)),
        ),
    ):
        context = _ResponseContext(response)
        monkeypatch.setattr(
            httpx,
            "stream",
            lambda *_args, _context=context, **_kwargs: _context,
        )
        with pytest.raises(ReplayError, match="byte cap"):
            default_transport(str(request.url), 5.0, MAX_PAYLOAD_BYTES)


def test_seal_selection_rejects_unsafe_urls_and_duplicate_rows() -> None:
    row = {
        "row_index": 0,
        "original": "https://example.test/request/1.json",
        "timestamp": "20250101000000",
        "cdx_digest": "ABC",
        "cdx_statuscode": 200,
        "cdx_length": 321,
        "expected_status": 200,
        "expected_payload_bytes": len(_payload()),
        "expected_payload_sha256": "0" * 64,
        "expected_media_type": "application/json",
    }
    with pytest.raises(ValueError, match="unique"):
        seal_selection(source_cdx_sha256="1" * 64, rows=[row, row])
    escaped = dict(row, original="https://example.test:444/request/1.json")
    with pytest.raises(ValueError, match="canonical HTTPS"):
        seal_selection(source_cdx_sha256="1" * 64, rows=[escaped])


def test_offline_cli_writes_payload_checkpoint_result_and_receipt(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    selection = tmp_path / "selection.json"
    selection.write_bytes(canonical_json_bytes(_selection()))
    output = tmp_path / "objects"
    result = tmp_path / "result.json"
    checkpoint = tmp_path / "checkpoint.json"
    receipt = tmp_path / "receipt.json"
    original_replay = replay_approved_rows

    def offline_replay(selection_value, config, **kwargs):
        return original_replay(
            selection_value,
            config,
            output_dir=kwargs["output_dir"],
            checkpoint_path=kwargs["checkpoint_path"],
            observer=kwargs["observer"],
            transport=lambda url, *_: _response(
                url,
                _payload(),
                **{"Content-Type": "application/json"},
            ),
        )

    monkeypatch.setattr(cli_module, "replay_approved_rows", offline_replay)
    args = build_parser().parse_args(
        [
            "internet-archive-replay",
            "--selection",
            str(selection),
            "--allowed-target-host",
            "example.test",
            "--output-dir",
            str(output),
            "--result",
            str(result),
            "--checkpoint",
            str(checkpoint),
            "--receipt",
            str(receipt),
        ],
    )
    args.func(args)

    assert json.loads(result.read_text(encoding="utf-8"))["objects"][0]["sha256"]
    assert json.loads(checkpoint.read_text(encoding="utf-8"))["next_position"] == 1
    assert json.loads(receipt.read_text(encoding="utf-8"))["status"] == "succeeded"
    assert len(list(output.glob("*.raw"))) == 1


def test_cli_rejects_colliding_evidence_paths_before_network(tmp_path: Path) -> None:
    selection = tmp_path / "selection.json"
    selection.write_bytes(canonical_json_bytes(_selection()))
    args = build_parser().parse_args(
        [
            "internet-archive-replay",
            "--selection",
            str(selection),
            "--allowed-target-host",
            "example.test",
            "--output-dir",
            str(tmp_path / "objects"),
            "--result",
            str(tmp_path / "same.json"),
            "--checkpoint",
            str(tmp_path / "same.json"),
            "--receipt",
            str(tmp_path / "receipt.json"),
        ],
    )
    with pytest.raises(SystemExit, match="paths must differ"):
        args.func(args)
