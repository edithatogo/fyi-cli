"""Rerun the frozen replay corpus against the unchanged Python reference.

The Rust replay tests consume the same oracle. These finite, socket-free cases
check common behavior and record two intentional native improvements separately;
they do not establish live-source, receipt, or downstream archive parity.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import TYPE_CHECKING

import httpx
import pytest

from fyi_system.acquisition_receipts import canonical_json_bytes, sha256_bytes
from fyi_system.internet_archive_replay import (
    ReplayConfig,
    ReplayError,
    replay_approved_rows,
    validate_selection,
)

if TYPE_CHECKING:
    from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "conformance/internet-archive-replay"
ORACLE = json.loads((FIXTURES / "oracle-v1.json").read_text(encoding="utf-8"))


@pytest.fixture(autouse=True)
def forbid_live_http(monkeypatch: pytest.MonkeyPatch) -> None:
    """Reject any accidental fallback from the injected in-process transport."""

    def unexpected_http(*_args: object, **_kwargs: object) -> NoReturn:
        pytest.fail("replay conformance must never use a live HTTP transport")

    monkeypatch.setattr(httpx, "stream", unexpected_http)
    monkeypatch.setattr(httpx.Client, "send", unexpected_http)
    monkeypatch.setattr(httpx.AsyncClient, "send", unexpected_http)


def _execute_case(case: dict[str, Any], root: Path) -> dict[str, Any]:
    output = root / "objects"
    checkpoint = root / "checkpoint.json"
    if initial := case["initial"]:
        output.mkdir()
        for name, body_hex in initial["files"].items():
            assert Path(name).name == name
            (output / name).write_bytes(bytes.fromhex(body_hex))
        if checkpoint_json := initial.get("checkpoint_json"):
            checkpoint.write_bytes(checkpoint_json.encode("utf-8"))

    calls: list[dict[str, Any]] = []
    observations: list[dict[str, Any]] = []

    def transport(url: str, timeout: float, byte_cap: int) -> httpx.Response:
        assert len(calls) < len(case["events"]), "request exceeds frozen exchange budget"
        event = case["events"][len(calls)]
        calls.append({"url": url, "timeout_seconds": timeout, "byte_cap": byte_cap})
        if "error" in event:
            assert event["error"] == "retryable"
            message = "synthetic offline transport failure"
            raise httpx.ConnectError(message, request=httpx.Request("GET", url))
        return httpx.Response(
            event["status"],
            headers=event["headers"],
            content=bytes.fromhex(event["body_hex"]),
            request=httpx.Request("GET", event.get("url", url)),
        )

    def observe(response: httpx.Response) -> None:
        observations.append(
            {
                "url": str(response.url),
                "status": response.status_code,
                "bytes": len(response.content),
                "sha256": sha256_bytes(response.content),
                "attempts": 1,
                "retry_delays_seconds": [],
            },
        )

    result_json = None
    failure = None
    try:
        result = replay_approved_rows(
            json.loads(case["selection_json"]),
            ReplayConfig(**case["config"]),
            output_dir=output,
            checkpoint_path=checkpoint,
            transport=transport,
            observer=observe,
            clock=lambda: 0.0,
        )
        result_json = canonical_json_bytes(result).decode("utf-8")
    except (ReplayError, ValueError, httpx.ConnectError) as error:
        failure = {"type": type(error).__name__, "message": str(error)}

    objects = {}
    if output.exists():
        for path in output.iterdir():
            assert path.is_file()
            assert not path.is_symlink()
            objects[path.name] = path.read_bytes().hex()
    return {
        "calls": calls,
        "observations": observations,
        "result_json": result_json,
        "error": failure,
        "checkpoint_json": checkpoint.read_bytes().decode("utf-8") if checkpoint.exists() else None,
        "files": objects,
    }


def test_replay_oracle_provenance_and_source_are_pinned() -> None:
    unsigned = dict(ORACLE)
    digest = unsigned.pop("fixture_sha256")
    assert sha256_bytes(canonical_json_bytes(unsigned)) == digest
    provenance = ORACLE["oracle"]
    assert provenance["network"] == "mocked-only"
    assert provenance["python"] == "3.15.1"
    assert provenance["httpx"] == "0.28.1"
    assert provenance["jsonschema"] == "4.26.0"
    assert provenance["source_commit"] == "9c52d73e496b3d5f972d51566eb8da855a8b167d"
    assert (
        sha256_bytes((FIXTURES / "selection-v1.schema.json").read_bytes())
        == provenance["schema_sha256"]
    )
    assert (FIXTURES / "selection-v1.schema.json").read_bytes() == (
        ROOT / "src/fyi_system/schemas/internet-archive-replay-selection-v1.schema.json"
    ).read_text(encoding="utf-8").encode("utf-8")
    # Source is normal text in Git; normalize checkout CRLF to its recorded LF.
    source = (ROOT / "src/fyi_system/internet_archive_replay.py").read_text(encoding="utf-8")
    assert sha256_bytes(source.encode("utf-8")) == provenance["source_sha256"]
    assert len(ORACLE["cases"]) == 30
    assert len(ORACLE["invalid_selections"]) == 24
    assert len(ORACLE["invalid_configs"]) == 10
    assert {case["name"] for case in ORACLE["legacy_negative_evidence"]} == {
        "legacy_encoded_length_failure",
        "legacy_resigned_checkpoint_approval_bypass",
    }


@pytest.mark.parametrize("case", ORACLE["cases"], ids=lambda case: case["name"])
def test_python_replay_matches_the_shared_oracle(case: dict[str, Any], tmp_path: Path) -> None:
    assert _execute_case(case, tmp_path) == case["expected"]


@pytest.mark.parametrize("case", ORACLE["invalid_selections"], ids=lambda case: case["name"])
def test_python_rejects_the_shared_invalid_selections(case: dict[str, Any]) -> None:
    with pytest.raises(ValueError, match=re.escape(case["error"]["message"])) as caught:
        validate_selection(json.loads(case["selection_json"]))
    assert {"type": type(caught.value).__name__, "message": str(caught.value)} == case["error"]


@pytest.mark.parametrize("case", ORACLE["invalid_configs"])
def test_python_rejects_the_shared_invalid_configs(case: dict[str, Any]) -> None:
    with pytest.raises(ValueError, match=re.escape(case["error"])) as caught:
        ReplayConfig(**case["config"])
    assert str(caught.value) == case["error"]


@pytest.mark.parametrize(
    "case",
    ORACLE["legacy_negative_evidence"],
    ids=lambda case: case["name"],
)
def test_documented_legacy_divergences_remain_explicit(
    case: dict[str, Any],
    tmp_path: Path,
) -> None:
    """These observations are legacy defects, never native acceptance targets."""
    observed = _execute_case(case, tmp_path)
    assert observed == case["expected"]
    if case["name"] == "legacy_encoded_length_failure":
        assert observed["result_json"] is None
        assert case["native_expected_result_json"] is not None
    else:
        assert case["name"] == "legacy_resigned_checkpoint_approval_bypass"
        approved = json.loads(case["selection_json"])["rows"][0]
        legacy_object = json.loads(observed["result_json"])["objects"][0]
        assert legacy_object["sha256"] != approved["expected_payload_sha256"]
        assert observed["calls"] == []
