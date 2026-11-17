"""Socket-free acceptance tests for documentation byte evidence and drift."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import socket
import subprocess
import sys
from datetime import date
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "verify_foi_source_snapshots.py"
SPEC = importlib.util.spec_from_file_location("source_checker", SCRIPT)
assert SPEC is not None
assert SPEC.loader is not None
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


@pytest.fixture(autouse=True)
def no_network(monkeypatch: pytest.MonkeyPatch) -> None:
    def forbidden(*_args: object, **_kwargs: object) -> None:
        pytest.fail("documentation evidence tests must not open sockets")

    monkeypatch.setattr(socket, "socket", forbidden)
    monkeypatch.setattr(socket, "getaddrinfo", forbidden)


def make_capture(root: Path, source_id: str, body: bytes = b"synthetic documentation\n") -> dict:
    platform, url, media_types = checker.SOURCES[source_id]
    entry = {
        "schema_version": "1.0.0",
        "adapter": "platform-documentation",
        "adapter_version": "1.0.0",
        "source_id": source_id,
        "platform_id": platform,
        "source_url": url,
        "retrieved_at": "2026-11-17T09:00:00Z",
        "http_status": 200,
        "media_type": min(media_types),
        "body_bytes": len(body),
        "body_sha256": hashlib.sha256(body).hexdigest(),
        "body_path": "body.bin",
        "receipt_path": "receipt.json",
    }
    folder = root / source_id
    folder.mkdir(parents=True, exist_ok=True)
    (folder / "body.bin").write_bytes(body)
    publish_capture(folder, entry)
    return entry


def publish_capture(folder: Path, entry: dict) -> None:
    """Create an independent synthetic receipt, not a receipt made by the checker."""
    result = canonical(entry)
    receipt = {
        "schema": "urn:fyi-cli:acquisition-receipt",
        "schema_version": "1.0.0",
        "command": "observe-platform-docs",
        "adapter": {"id": "platform-documentation", "version": "1.0.0"},
        "source": {"url": entry["source_url"]},
        "request_bounds": {
            "source_id": entry["source_id"],
            "max_requests": 1,
            "max_response_bytes": 2_097_152,
            "timeout_seconds": 30,
            "redirects": 0,
            "retries": 0,
        },
        "started_at": "2026-11-17T09:00:00Z",
        "completed_at": entry["retrieved_at"],
        "status": "succeeded",
        "responses": [
            {
                "url": entry["source_url"],
                "status": 200,
                "bytes": entry["body_bytes"],
                "sha256": entry["body_sha256"],
                "attempts": 1,
                "retry_delays_seconds": [],
            },
        ],
        "totals": {"requests": 1, "bytes": entry["body_bytes"], "retries": 0},
        "rate_limit": {"name": None, "minimum_interval_seconds": None},
        "checkpoint": {"before_sha256": None, "after_sha256": None},
        "result": {
            "representation": "canonical_result_projection",
            "media_type": "application/json",
            "bytes": len(result),
            "sha256": hashlib.sha256(result).hexdigest(),
        },
    }
    receipt["receipt_sha256"] = hashlib.sha256(canonical(receipt)).hexdigest()
    (folder / "receipt.json").write_bytes(canonical(receipt))
    (folder / "observation.json").write_bytes(result)


@pytest.fixture
def captured(tmp_path: Path) -> tuple[dict, Path]:
    folder = tmp_path / "captures"
    entries = [make_capture(folder, source_id) for source_id in checker.SOURCES]
    return {
        "schema_version": "1.0.0",
        "scope": "documentation_only",
        "fingerprint_kind": "raw-response-bytes-sha256",
        "reviewed_on": "2026-11-17",
        "network_access": False,
        "writes_enabled": False,
        "retention": "operator_local",
        "acquisition_method": "managed_proxy_https_review",
        "sources": [
            {
                key: value
                for key, value in entry.items()
                if key
                not in {"schema_version", "adapter", "adapter_version", "body_path", "receipt_path"}
            }
            for entry in entries
        ],
    }, folder


def test_metadata_validation_explicitly_does_not_claim_byte_verification(captured: tuple) -> None:
    baseline, _ = captured
    report = checker.validate_baseline(baseline)
    assert report["status"] == "metadata_valid"
    assert report["byte_verification"] == "not_performed"
    assert report["source_count"] == 6


def test_real_repository_baseline_records_the_actual_research_transport() -> None:
    data = checker.load_json(checker.DEFAULT_BASELINE)
    assert checker.validate_baseline(data)["source_count"] == 6
    assert data["acquisition_method"] == "managed_proxy_https_review"
    assert all("adapter" not in entry for entry in data["sources"])


def test_baseline_bodies_are_verified_without_claiming_rust_receipt_verification(
    captured: tuple,
) -> None:
    baseline, folder = captured
    report = checker.verify_baseline_bodies(baseline, folder)
    assert report["status"] == "baseline_bytes_verified"
    assert report["byte_verification"] == "baseline_verified"
    assert report["acquisition_method"] == "managed_proxy_https_review"
    (folder / "india_faq" / "body.bin").write_bytes(b"tampered")
    with pytest.raises(ValueError, match="raw response bytes"):
        checker.verify_baseline_bodies(baseline, folder)


def test_research_metadata_cannot_substitute_for_guarded_observation(captured: tuple) -> None:
    baseline, folder = captured
    path = folder / "muckrock_client" / "observation.json"
    path.write_bytes(canonical(baseline["sources"][0]))
    with pytest.raises(ValueError, match="observation fields"):
        checker.compare_snapshots(baseline, folder)


def test_unknown_baseline_acquisition_method_is_rejected(captured: tuple) -> None:
    baseline, _ = captured
    baseline["acquisition_method"] = "live_api_verified"
    with pytest.raises(ValueError, match="acquisition method"):
        checker.validate_baseline(baseline)


def test_complete_local_bundle_binds_raw_bytes_receipt_and_source(captured: tuple) -> None:
    baseline, folder = captured
    before = copy.deepcopy(baseline)
    report = checker.compare_snapshots(baseline, folder, as_of=date(2026, 11, 17))
    assert report["status"] == "verified"
    assert report["byte_verification"] == "verified"
    assert report["drift"] == []
    assert baseline == before


def test_changed_source_bytes_require_review_without_replacing_baseline(captured: tuple) -> None:
    baseline, folder = captured
    old_hash = baseline["sources"][0]["body_sha256"]
    make_capture(folder, "muckrock_client", b"changed synthetic documentation\n")
    report = checker.compare_snapshots(baseline, folder, as_of=date(2026, 11, 17))
    assert report["status"] == "drift"
    assert report["drift"] == ["muckrock_client"]
    assert baseline["sources"][0]["body_sha256"] == old_hash


@pytest.mark.parametrize("filename", ["body.bin", "receipt.json", "observation.json"])
def test_missing_capture_component_is_not_success(captured: tuple, filename: str) -> None:
    baseline, folder = captured
    (folder / "muckrock_client" / filename).unlink()
    with pytest.raises((ValueError, OSError)):
        checker.compare_snapshots(baseline, folder)


def test_tampered_body_and_self_digest_fail_closed(captured: tuple) -> None:
    baseline, folder = captured
    target = folder / "muckrock_client"
    (target / "body.bin").write_bytes(b"synthetic documentatioN\n")
    with pytest.raises(ValueError, match="raw response bytes"):
        checker.compare_snapshots(baseline, folder)
    make_capture(folder, "muckrock_client")
    receipt = json.loads((target / "receipt.json").read_text())
    receipt["receipt_sha256"] = "0" * 64
    (target / "receipt.json").write_bytes(canonical(receipt))
    with pytest.raises(ValueError, match="receipt does not bind"):
        checker.compare_snapshots(baseline, folder)


@pytest.mark.parametrize("target", ["body.bin", "receipt.json", "observation.json"])
def test_symlink_evidence_is_rejected(captured: tuple, target: str, tmp_path: Path) -> None:
    baseline, folder = captured
    original = folder / "muckrock_client" / target
    other = tmp_path / "outside"
    original.replace(other)
    original.symlink_to(other)
    with pytest.raises(ValueError, match="symlink"):
        checker.compare_snapshots(baseline, folder)


def test_symlink_capture_directory_is_rejected(captured: tuple, tmp_path: Path) -> None:
    baseline, folder = captured
    original = folder / "muckrock_client"
    other = tmp_path / "outside"
    original.replace(other)
    original.symlink_to(other, target_is_directory=True)
    with pytest.raises(ValueError, match="real directory"):
        checker.compare_snapshots(baseline, folder)


def test_symlink_ancestor_of_capture_root_is_rejected(captured: tuple, tmp_path: Path) -> None:
    baseline, folder = captured
    alias = tmp_path / "alias"
    alias.symlink_to(tmp_path, target_is_directory=True)
    with pytest.raises(ValueError, match="symlink ancestor"):
        checker.compare_snapshots(baseline, alias / folder.name)


@pytest.mark.parametrize("path", ["../body.bin", "/etc/passwd", "extra/body.bin"])
def test_snapshot_path_cannot_escape_source_bundle(captured: tuple, path: str) -> None:
    baseline, folder = captured
    target = folder / "muckrock_client" / "observation.json"
    source = json.loads(target.read_text())
    source["body_path"] = path
    target.write_bytes(canonical(source))
    with pytest.raises(ValueError, match="paths"):
        checker.compare_snapshots(baseline, folder)


@pytest.mark.parametrize("mutation", ["missing", "duplicate", "unknown"])
def test_source_set_must_be_complete_and_unique(captured: tuple, mutation: str) -> None:
    baseline, _ = captured
    if mutation == "missing":
        baseline["sources"].pop()
    elif mutation == "duplicate":
        baseline["sources"][1] = copy.deepcopy(baseline["sources"][0])
    else:
        baseline["sources"][0]["source_id"] = "unknown"
    with pytest.raises(ValueError, match="source"):
        checker.validate_baseline(baseline)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("source_url", "https://www.muckrock.com/api_v2/requests/"),
        ("source_url", "https://raw.githubusercontent.com.evil.test/docs"),
        ("source_url", "https://name:secret@www.foia.gov/developer/"),
        ("source_url", "https://www.foia.gov/developer/?token=secret"),
        ("platform_id", "uscis"),
        ("http_status", 410),
        ("http_status", 304),
        ("body_bytes", True),
        ("body_bytes", 0),
        ("body_bytes", 2_097_153),
        ("body_sha256", "bad"),
        ("media_type", "application/octet-stream"),
        ("retrieved_at", "2026-11-17T09:00:00"),
        ("retrieved_at", "2026-11-17T09:00:00.123456001Z"),
        ("retrieved_at", "2026-11-17T09:00:30.000000999Z"),
        ("retrieved_at", "2026-11-18T00:00:00Z"),
    ],
)
def test_baseline_rejects_wrong_identity_transport_and_fingerprints(
    captured: tuple,
    field: str,
    value: object,
) -> None:
    baseline, _ = captured
    baseline["sources"][0][field] = value
    with pytest.raises(ValueError, match=r"identity|successful|byte|media|timestamp|future"):
        checker.validate_baseline(baseline)


@pytest.mark.parametrize("field", ["network_access", "writes_enabled"])
def test_baseline_cannot_enable_runtime_capabilities(captured: tuple, field: str) -> None:
    baseline, _ = captured
    baseline[field] = True
    with pytest.raises(ValueError, match="runtime capabilities"):
        checker.validate_baseline(baseline)


def test_stale_and_future_observations_fail(captured: tuple) -> None:
    baseline, folder = captured
    source = json.loads((folder / "muckrock_client" / "observation.json").read_text())
    for timestamp in ["2026-11-17T08:59:59Z", "2026-11-18T00:00:00Z"]:
        source["retrieved_at"] = timestamp
        publish_capture(folder / source["source_id"], source)
        with pytest.raises(ValueError, match=r"predates|future"):
            checker.compare_snapshots(baseline, folder, as_of=date(2026, 11, 17))


def test_rehashed_receipt_with_wrong_origin_is_rejected(captured: tuple) -> None:
    baseline, folder = captured
    path = folder / "muckrock_client" / "receipt.json"
    receipt = json.loads(path.read_text())
    receipt["responses"][0]["url"] = "https://example.org/"
    receipt.pop("receipt_sha256")
    receipt["receipt_sha256"] = hashlib.sha256(canonical(receipt)).hexdigest()
    path.write_bytes(canonical(receipt))
    with pytest.raises(ValueError, match="receipt does not bind"):
        checker.compare_snapshots(baseline, folder)


@pytest.mark.parametrize("value", ['{"x":1,"x":2}', '{"x":NaN}', "[1,2]"])
def test_malformed_metadata_fails_without_echoing_content(tmp_path: Path, value: str) -> None:
    path = tmp_path / "bad.json"
    path.write_text(value)
    with pytest.raises(ValueError, match=r"duplicate|nonfinite|root"):
        checker.load_json(path)


def test_cli_requires_evidence_to_claim_verification_and_fails_on_drift(
    captured: tuple,
    tmp_path: Path,
) -> None:
    baseline, folder = captured
    path = tmp_path / "baseline.json"
    path.write_bytes(canonical(baseline))
    command = [sys.executable, str(SCRIPT), "--baseline", str(path)]
    result = subprocess.run(  # noqa: S603 - fixed interpreter/script, local fixtures
        command,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0
    assert json.loads(result.stdout)["status"] == "metadata_valid"
    make_capture(folder, "muckrock_client", b"changed")
    result = subprocess.run(  # noqa: S603 - fixed interpreter/script, local fixtures
        [*command, "--observations", str(folder), "--as-of", "2026-11-17"],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 1
    assert json.loads(result.stdout)["drift"] == ["muckrock_client"]
    (folder / "india_faq" / "body.bin").unlink()
    result = subprocess.run(  # noqa: S603 - fixed interpreter/script, local fixtures
        [*command, "--observations", str(folder)],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 1
    assert json.loads(result.stdout)["status"] == "invalid"
