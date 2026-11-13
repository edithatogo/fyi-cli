"""Offline contracts for the non-Alaveteli research inventory."""

from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import sys
from datetime import date
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "verify_foi_platform_landscape.py"
SPEC = importlib.util.spec_from_file_location("landscape_checker", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


def inventory() -> dict:
    return checker.load_json(ROOT / "conformance/providers/foi-platform-landscape-v1.json")


def observations(data: dict) -> dict:
    return {
        "schema_version": "1.0.0",
        "observed_on": data["reviewed_on"],
        "platforms": [
            {"id": p["id"], "status": "available", "contract": copy.deepcopy(p["contract"])}
            for p in data["platforms"]
        ],
    }


def test_repository_inventory_covers_six_platforms_without_enabling_network_or_writes() -> None:
    report = checker.validate_inventory(inventory())
    assert report["platform_count"] == 6
    assert report["overdue"] == []
    assert inventory()["network_access"] is False
    assert inventory()["writes_enabled"] is False


@pytest.mark.parametrize("field", ["network_access", "writes_enabled"])
@pytest.mark.parametrize("enabled", [True, 0, "false", None])
def test_snapshot_cannot_enable_actions_or_coerce_boolean_fields(
    field: str, enabled: object,
) -> None:
    data = inventory()
    data[field] = enabled
    with pytest.raises(ValueError):
        checker.validate_inventory(data)


def test_missing_candidate_fails_closed() -> None:
    data = inventory()
    data["platforms"].pop()
    with pytest.raises(ValueError, match="platform inventory"):
        checker.validate_inventory(data)


def test_duplicate_candidates_and_ranks_are_rejected() -> None:
    data = inventory()
    data["platforms"][1]["id"] = data["platforms"][0]["id"]
    with pytest.raises(ValueError):
        checker.validate_inventory(data)
    data = inventory()
    data["platforms"][1]["rank"] = data["platforms"][0]["rank"]
    with pytest.raises(ValueError):
        checker.validate_inventory(data)


@pytest.mark.parametrize("field", ["reason", "next_experiment", "jurisdiction", "lineage"])
def test_decision_context_is_required(field: str) -> None:
    data = inventory()
    data["platforms"][0][field] = ""
    with pytest.raises(ValueError):
        checker.validate_inventory(data)


@pytest.mark.parametrize("platform_id", ["uscis", "canada_atip", "india_rti"])
def test_sensitive_and_undocumented_targets_cannot_be_promoted(platform_id: str) -> None:
    data = inventory()
    item = next(p for p in data["platforms"] if p["id"] == platform_id)
    item["decision"] = "read_only_pilot"
    with pytest.raises(ValueError, match="decision"):
        checker.validate_inventory(data)


@pytest.mark.parametrize(
    "url",
    ["http://help.muckrock.com/api", "https://name:secret@help.muckrock.com/api",
     "https://help.muckrock.com/api?token=secret", "https://help.muckrock.com/api#secret",
     "https://help.muckrock.com.evil.test/api", "https://127.0.0.1/api",
     "https://help.muckrock.com:444/api"],
)
def test_sources_require_allowlisted_credential_free_https_urls(url: str) -> None:
    data = inventory()
    data["platforms"][0]["sources"][0]["url"] = url
    with pytest.raises(ValueError, match="source URL"):
        checker.validate_inventory(data)


def test_missing_evidence_and_invalid_access_status_are_rejected() -> None:
    data = inventory()
    data["platforms"][0]["sources"] = []
    with pytest.raises(ValueError):
        checker.validate_inventory(data)
    data = inventory()
    data["platforms"][0]["sources"][0]["access"] = "live_api_verified"
    with pytest.raises(ValueError):
        checker.validate_inventory(data)


def test_review_dates_and_cadence_are_consistent() -> None:
    data = inventory()
    data["platforms"][0]["next_review_on"] = "2026-12-15"
    with pytest.raises(ValueError, match="cadence"):
        checker.validate_inventory(data)
    with pytest.raises(ValueError, match="future"):
        checker.validate_inventory(inventory(), as_of=date(2026, 11, 13))


def test_overdue_is_a_reported_fact_not_an_automatic_promotion() -> None:
    report = checker.validate_inventory(inventory(), as_of=date(2027, 3, 1))
    assert len(report["overdue"]) == 6


def test_fingerprint_is_order_independent_and_binds_contract_values() -> None:
    original = inventory()["platforms"][0]["contract"]
    reordered = dict(reversed(list(original.items())))
    assert checker.fingerprint(original) == checker.fingerprint(reordered)
    data = inventory()
    data["platforms"][0]["contract"]["authentication"] = "changed"
    with pytest.raises(ValueError, match="fingerprint"):
        checker.validate_inventory(data)


def test_matching_observations_report_no_drift_and_do_not_mutate_inputs() -> None:
    data = inventory()
    observed = observations(data)
    before = copy.deepcopy((data, observed))
    assert checker.compare_observations(data, observed) == []
    assert (data, observed) == before


def test_changed_and_deprecated_contracts_report_drift() -> None:
    data = inventory()
    observed = observations(data)
    observed["platforms"][0]["contract"]["authentication"] = "changed"
    observed["platforms"][1]["status"] = "deprecated"
    assert checker.compare_observations(data, observed) == ["muckrock", "fragdenstaat"]


@pytest.mark.parametrize("status", ["unavailable", "deprecated"])
def test_unavailable_or_deprecated_evidence_is_never_a_success(status: str) -> None:
    data = inventory()
    observed = observations(data)
    observed["platforms"][0]["status"] = status
    assert checker.compare_observations(data, observed) == ["muckrock"]


def test_observations_must_be_complete_unique_and_not_older_than_review() -> None:
    data = inventory()
    for mutation in ("missing", "duplicate", "old"):
        observed = observations(data)
        if mutation == "missing":
            observed["platforms"].pop()
        elif mutation == "duplicate":
            observed["platforms"][1]["id"] = "muckrock"
        else:
            observed["observed_on"] = "2026-11-13"
        with pytest.raises(ValueError):
            checker.compare_observations(data, observed)


@pytest.mark.parametrize("raw", ['{"x":1,"x":2}', '{"nested":{"x":1,"x":2}}',
                                  '{"x":NaN}', '{"x":Infinity}'])
def test_json_loader_rejects_duplicate_keys_and_nonfinite_values(tmp_path: Path, raw: str) -> None:
    path = tmp_path / "input.json"
    path.write_text(raw, encoding="utf-8")
    with pytest.raises(ValueError):
        checker.load_json(path)


def test_cli_emits_json_and_can_fail_on_overdue_without_network(tmp_path: Path) -> None:
    result = subprocess.run(
        [sys.executable, str(SCRIPT)], cwd=tmp_path, capture_output=True, text=True, check=False,
    )
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout)["platform_count"] == 6
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "--as-of", "2027-03-01", "--fail-on-overdue"],
        cwd=tmp_path, capture_output=True, text=True, check=False,
    )
    assert result.returncode == 1
    assert len(json.loads(result.stdout)["overdue"]) == 6


def test_cli_drift_and_malformed_input_fail_closed(tmp_path: Path) -> None:
    observed = observations(inventory())
    observed["platforms"][0]["status"] = "deprecated"
    path = tmp_path / "observation.json"
    path.write_text(json.dumps(observed), encoding="utf-8")
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "--observations", str(path)],
        capture_output=True, text=True, check=False,
    )
    assert result.returncode == 1
    assert json.loads(result.stdout)["drift"] == ["muckrock"]
    path.write_text("not JSON", encoding="utf-8")
    result = subprocess.run(
        [sys.executable, str(SCRIPT), "--inventory", str(path)],
        capture_output=True, text=True, check=False,
    )
    assert result.returncode == 1
    assert json.loads(result.stdout)["status"] == "invalid"
