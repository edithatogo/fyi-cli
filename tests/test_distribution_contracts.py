"""Regression contracts for release-bound distribution metadata."""

from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path

import pytest

from scripts import verify_packaging_assets as packaging
from scripts.validate_install_smoke_matrix import validate as validate_smokes
from scripts.validate_registry_submission_ledger import validate as validate_ledger

ROOT = Path(__file__).resolve().parents[1]
HOME_BREW_ARTIFACTS = {
    "linux-x86_64": "fyi-cli-linux-amd64.tar.gz",
    "macos-x86_64": "fyi-cli-macos-amd64.tar.gz",
    "macos-arm64": "fyi-cli-macos-arm64.tar.gz",
}


def _write_receipt(root: Path, name: str, payload: dict) -> dict:
    relative = f"packaging/test-evidence/{name}.json"
    path = root / relative
    path.parent.mkdir(exist_ok=True)
    path.write_text(json.dumps(payload), encoding="utf-8")
    return {
        "path": relative,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "version": payload["version"],
        "source_commit": payload["source_commit"],
    }


def _receipt_payload(release: dict, kind: str, subjects: dict[str, str]) -> dict:
    return {
        "schema_version": 1,
        "kind": kind,
        "version": release["component_version"],
        "source_commit": release["source_commit"],
        "result": "passed",
        "exit_code": 0,
        "subjects": [{"name": name, "sha256": digest} for name, digest in subjects.items()],
    }


def _record_smoke(root: Path, target_id: str = "homebrew") -> None:
    release = json.loads((root / "packaging/release-evidence.json").read_text())
    if target_id in {"homebrew", "asdf-mise", "cargo-binstall"}:
        platforms = dict(HOME_BREW_ARTIFACTS)
        if target_id == "cargo-binstall":
            platforms["windows-x86_64"] = "fyi-cli-windows-amd64.exe.zip"
        digests = {asset["name"]: asset["sha256"] for asset in release["assets"]}
        subjects = {
            platform: {"name": name, "sha256": digests[name]}
            for platform, name in platforms.items()
        }
    else:
        subjects = {
            artifact["platform"]: artifact
            for artifact in release["native_artifacts"]
            if artifact["target"] == target_id
        }
    path = root / "packaging/install-smoke-matrix.json"
    matrix = json.loads(path.read_text())
    target = next(row for row in matrix["targets"] if row["id"] == target_id)
    samples = []
    for platform, artifact in subjects.items():
        payload = _receipt_payload(release, "clean_install", {artifact["name"]: artifact["sha256"]})
        payload.update(
            target=target_id,
            platform=platform,
            command=target["smoke"],
            clean_environment=True,
            network_during_smoke=False,
            remote_writes=False,
            timeout_seconds=target["timeout_seconds"],
            uninstalled=True,
        )
        receipt = _write_receipt(root, f"{target_id}-{platform}-smoke", payload)
        samples.append(
            {
                "platform": platform,
                "asset": artifact["name"],
                "sha256": artifact["sha256"],
                "transcript": receipt["path"],
                "transcript_sha256": receipt["sha256"],
                "verified_at": "2026-11-14",
            },
        )
    target.update(
        status="verified",
        evidence={
            "version": release["component_version"],
            "source_commit": release["source_commit"],
            "samples": samples,
        },
    )
    path.write_text(json.dumps(matrix))


def _record_native_artifacts(root: Path) -> None:
    path = root / "packaging/release-evidence.json"
    release = json.loads(path.read_text())
    release["native_artifacts"] = []
    for platform in ("linux-x86_64", "linux-aarch64"):
        name = f"fyi-mcp-{platform}.oci"
        digest = hashlib.sha256(f"synthetic OCI fixture {platform}".encode()).hexdigest()
        receipt = _receipt_payload(release, "native_artifact", {name: digest})
        receipt.update(target="ghcr", platform=platform)
        release["native_artifacts"].append(
            {
                "target": "ghcr",
                "platform": platform,
                "version": release["component_version"],
                "source_commit": release["source_commit"],
                "name": name,
                "sha256": digest,
                "verification_receipt": _write_receipt(root, f"ghcr-{platform}-artifact", receipt),
            },
        )
    path.write_text(json.dumps(release))


def _qualify_release(root: Path, target_id: str = "homebrew") -> None:
    """Build synthetic positive evidence, never a claim about the real public release."""
    path = root / "packaging/release-evidence.json"
    release = json.loads(path.read_text())
    release["release"]["tag"] = "v0.1.2"
    release["release"]["url"] = "https://github.com/edithatogo/fyi-cli/releases/tag/v0.1.2"
    for asset in release["assets"]:
        asset["url"] = asset["url"].replace("/v1.2.0/", "/v0.1.2/")
    for name in ("release-manifest.json", "SHA256SUMS", "fyi-cli.spdx.json"):
        release["assets"].append(
            {
                "name": name,
                "sha256": hashlib.sha256(f"synthetic fixture {name}".encode()).hexdigest(),
                "size": 123,
                "url": f"https://github.com/edithatogo/fyi-cli/releases/download/v0.1.2/{name}",
            },
        )
    all_subjects = {asset["name"]: asset["sha256"] for asset in release["assets"]}
    all_subjects.update(
        {asset["name"]: asset["sha256"] for asset in release.get("native_artifacts", [])},
    )
    installed = {
        name: digest
        for name, digest in all_subjects.items()
        if name.endswith((".tar.gz", ".zip", ".oci"))
    }
    subjects = {
        "downloaded_bytes": all_subjects,
        "clean_install": installed,
        "signed_release": {
            name: all_subjects[name] for name in ("release-manifest.json", "SHA256SUMS")
        },
        "sbom": {**installed, "fyi-cli.spdx.json": all_subjects["fyi-cli.spdx.json"]},
        "provenance": installed,
    }
    release.update(
        submission_ready=True,
        blockers=[],
        verification=dict.fromkeys(subjects, True),
        verification_receipts={
            kind: _write_receipt(root, kind, _receipt_payload(release, kind, bound))
            for kind, bound in subjects.items()
        },
    )
    path.write_text(json.dumps(release))
    for relative in (
        "packaging/homebrew/fyi-cli.rb",
        "packaging/scoop/fyi-cli.json",
        "packaging/winget/edithatogo.fyi-cli.yaml",
        "packaging/chocolatey/tools/chocolateyinstall.ps1",
    ):
        manifest = root / relative
        manifest.write_text(manifest.read_text().replace("/v1.2.0/", "/v0.1.2/"))
    ledger_path = root / "packaging/registry-submissions.json"
    ledger = json.loads(ledger_path.read_text())
    target = next(row for row in ledger["targets"] if row["id"] == target_id)
    target.update(status="assets-ready", submission_ready=True, blockers=[])
    ledger_path.write_text(json.dumps(ledger))
    _record_smoke(root, target_id)


@pytest.fixture
def distribution_tree(tmp_path: Path) -> Path:
    shutil.copytree(ROOT / "packaging", tmp_path / "packaging")
    shutil.copytree(ROOT / "crates", tmp_path / "crates")
    shutil.copy(ROOT / "README.md", tmp_path / "README.md")
    ledger = json.loads((ROOT / "packaging/registry-submissions.json").read_text())
    for target in ledger["targets"]:
        for relative in target.get("packet_assets", []):
            destination = tmp_path / relative
            if not destination.exists():
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / relative, destination)
    baseline = packaging.check_distribution_contracts(tmp_path, "0.1.2")
    assert all(check.ok for check in baseline), baseline
    return tmp_path


def test_packaging_version_cannot_be_satisfied_by_a_comment(tmp_path: Path) -> None:
    relative = "packaging/scoop/fyi-cli.json"
    path = tmp_path / relative
    path.parent.mkdir(parents=True)
    path.write_text(
        '{"version":"9.9.9","notes":["formerly 0.1.2"]}',
        encoding="utf-8",
    )
    assert not packaging.check_versions(tmp_path, "0.1.2", (relative,))[0].ok


def test_version_token_is_not_a_substring() -> None:
    assert not packaging.file_mentions_version('version = "10.1.20"', "0.1.2")


def test_binary_packet_is_bound_to_observed_digest(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/scoop/fyi-cli.json"
    payload = json.loads(path.read_text(encoding="utf-8"))
    payload["architecture"]["64bit"]["hash"] = "1" * 64
    path.write_text(json.dumps(payload), encoding="utf-8")
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "digest" in check.message for check in checks)


def test_cli_packet_rejects_an_mcp_bundle(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/scoop/fyi-cli.json"
    payload = json.loads(path.read_text(encoding="utf-8"))
    payload["architecture"]["64bit"]["url"] = (
        "https://github.com/edithatogo/fyi-cli/releases/download/"
        "fyi-mcp-v0.1.2/fyi-mcp-0.1.2-win32.mcpb"
    )
    path.write_text(json.dumps(payload), encoding="utf-8")
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "asset URL" in check.message for check in checks)


def test_missing_release_evidence_fails(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/release-evidence.json"
    path.unlink(missing_ok=True)
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "release-evidence" in check.path for check in checks)


def test_malformed_release_evidence_fails_without_traceback(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/release-evidence.json"
    path.write_text('{"assets": []}', encoding="utf-8")
    assert any(
        not check.ok for check in packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    )


def test_blocked_packet_cannot_claim_readiness() -> None:
    ledger = json.loads((ROOT / "packaging/registry-submissions.json").read_text())
    target = next(row for row in ledger["targets"] if row["id"] == "homebrew")
    target["status"] = "assets-ready"
    target["submission_ready"] = True
    target["blockers"] = ["release-integrity-unverified"]
    assert any("blockers" in error for error in validate_ledger(ledger))


def test_help_substring_does_not_authorize_shell_commands(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    payload["targets"][0]["smoke"] = "fyi-cli --help; fyi-cli submit"
    path.write_text(json.dumps(payload))
    assert any("help-only" in error for error in validate_smokes(distribution_tree))


def test_verified_smoke_requires_versioned_evidence(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    payload["targets"][0]["status"] = "verified"
    payload["targets"][0]["evidence"] = None
    path.write_text(json.dumps(payload))
    assert any("evidence" in error for error in validate_smokes(distribution_tree))


@pytest.mark.parametrize(
    "contents",
    [
        '{"notes":["formerly 0.1.2"]}',
        '{"version":"9.9.9","version":"0.1.2"}',
        '[{"version":"0.1.2"}]',
    ],
)
def test_invalid_native_version_cannot_fall_back_to_prose(tmp_path: Path, contents: str) -> None:
    relative = "packaging/scoop/fyi-cli.json"
    path = tmp_path / relative
    path.parent.mkdir(parents=True)
    path.write_text(contents, encoding="utf-8")
    assert not packaging.check_versions(tmp_path, "0.1.2", (relative,))[0].ok


def test_xml_entity_declarations_fail_closed(tmp_path: Path) -> None:
    relative = "packaging/chocolatey/fyi-cli.nuspec"
    path = tmp_path / relative
    path.parent.mkdir(parents=True)
    path.write_text(
        '<!DOCTYPE package [<!ENTITY version "0.1.2">]>'
        "<package><metadata><version>&version;</version></metadata></package>",
        encoding="utf-8",
    )
    result = packaging.check_versions(tmp_path, "0.1.2", (relative,))
    assert not result[0].ok


@pytest.mark.parametrize("field", ["status", "class", "evidence", "packet_assets"])
def test_malformed_ledger_fields_return_errors(field: str) -> None:
    ledger = json.loads((ROOT / "packaging/registry-submissions.json").read_text())
    target = next(row for row in ledger["targets"] if row["id"] == "homebrew")
    target[field] = {"malformed": []}
    assert validate_ledger(ledger)


def test_ready_status_requires_explicit_readiness() -> None:
    ledger = json.loads((ROOT / "packaging/registry-submissions.json").read_text())
    target = next(row for row in ledger["targets"] if row["id"] == "homebrew")
    target.update(status="assets-ready", submission_ready=False, blockers=[])
    assert any("submission_ready" in error for error in validate_ledger(ledger))


def test_smoke_source_release_matches_component(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    payload["source_release"] = "9.9.9"
    path.write_text(json.dumps(payload))
    assert any("source_release" in error for error in validate_smokes(distribution_tree))


def test_verified_smoke_cannot_use_invalid_hash_or_missing_transcript(
    distribution_tree: Path,
) -> None:
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    payload["targets"][0].update(
        status="verified",
        evidence={
            "version": "0.1.2",
            "platform": "linux-x86_64",
            "sha256": "not-a-hash",
            "transcript": "missing-transcript.txt",
            "verified_at": "not-a-date",
        },
    )
    path.write_text(json.dumps(payload))
    assert any("evidence" in error for error in validate_smokes(distribution_tree))


def test_smoke_executable_is_bound_to_channel(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    ghcr = next(row for row in payload["targets"] if row["id"] == "ghcr")
    ghcr["smoke"] = "fyi-cli --help"
    path.write_text(json.dumps(payload))
    assert any("smoke" in error for error in validate_smokes(distribution_tree))


def test_source_manifest_structure_is_validated(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/snap/snapcraft.yaml"
    payload = json.loads(path.read_text())
    payload["parts"] = []
    path.write_text(json.dumps(payload))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "snap" in check.path for check in checks)


def test_source_manifest_must_pin_observed_source(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/snap/snapcraft.yaml"
    payload = json.loads(path.read_text())
    payload["parts"]["fyi-cli"]["source"] = (
        "https://github.com/edithatogo/fyi-cli/archive/master.tar.gz"
    )
    path.write_text(json.dumps(payload))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "snap" in check.path for check in checks)


def test_readiness_flags_require_real_verification_receipts(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/release-evidence.json"
    payload = json.loads(path.read_text())
    payload["verification"] = dict.fromkeys(payload["verification"], True)
    payload.update(submission_ready=True, blockers=[])
    path.write_text(json.dumps(payload))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "receipt" in check.message for check in checks)


def test_ready_packet_requires_its_native_smoke_record(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/registry-submissions.json"
    payload = json.loads(path.read_text())
    target = next(row for row in payload["targets"] if row["id"] == "homebrew")
    target.update(status="assets-ready", submission_ready=True, blockers=[])
    path.write_text(json.dumps(payload))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "clean-install smoke" in check.message for check in checks)


def test_smoke_transcript_is_bound_to_recorded_bytes(distribution_tree: Path) -> None:
    _record_smoke(distribution_tree)
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    assert validate_smokes(distribution_tree) == []
    transcript = distribution_tree / payload["targets"][0]["evidence"]["samples"][0]["transcript"]
    transcript.write_text("altered evidence\n")
    assert any("receipt digest" in error for error in validate_smokes(distribution_tree))


def test_homebrew_smoke_rejects_windows_and_unbound_digest(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    transcript = distribution_tree / "packaging/failed-smoke.txt"
    transcript.write_text("verification FAILED; no SBOM or provenance supplied\n")
    payload["targets"][0].update(
        status="verified",
        evidence={
            "version": "0.1.2",
            "platform": "windows-x86_64",
            "sha256": "0" * 64,
            "transcript": "packaging/failed-smoke.txt",
            "transcript_sha256": hashlib.sha256(transcript.read_bytes()).hexdigest(),
            "verified_at": "2026-11-14",
        },
    )
    path.write_text(json.dumps(payload))
    assert validate_smokes(distribution_tree)


def test_failed_text_cannot_be_a_successful_verification_receipt(
    distribution_tree: Path,
) -> None:
    path = distribution_tree / "packaging/release-evidence.json"
    payload = json.loads(path.read_text())
    transcript = distribution_tree / "packaging/failed-verification.txt"
    transcript.write_text("verification FAILED; no SBOM or provenance supplied\n")
    payload["verification"] = dict.fromkeys(payload["verification"], True)
    payload["verification_receipts"] = {
        kind: {
            "path": "packaging/failed-verification.txt",
            "sha256": hashlib.sha256(transcript.read_bytes()).hexdigest(),
            "version": payload["component_version"],
            "source_commit": payload["source_commit"],
        }
        for kind in payload["verification"]
    }
    path.write_text(json.dumps(payload))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "receipt" in check.message for check in checks)


def test_packet_inventory_cannot_hide_canonical_package_metadata() -> None:
    ledger = json.loads((ROOT / "packaging/registry-submissions.json").read_text())
    target = next(row for row in ledger["targets"] if row["id"] == "aur")
    target.update(status="assets-ready", submission_ready=True, blockers=[])
    target["packet_assets"] = ["README.md"]
    assert any("packet_assets" in error for error in validate_ledger(ledger))


def test_complete_structured_binary_evidence_can_qualify_a_packet(distribution_tree: Path) -> None:
    _qualify_release(distribution_tree)
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert all(check.ok for check in checks), checks
    assert packaging.submission_blockers(distribution_tree, "homebrew") == []


@pytest.mark.parametrize("missing", ["release-manifest.json", "SHA256SUMS", "fyi-cli.spdx.json"])
def test_ready_release_requires_integrity_asset_inventory(
    distribution_tree: Path,
    missing: str,
) -> None:
    _qualify_release(distribution_tree)
    path = distribution_tree / "packaging/release-evidence.json"
    release = json.loads(path.read_text())
    release["assets"] = [asset for asset in release["assets"] if asset["name"] != missing]
    path.write_text(json.dumps(release))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "inventory" in check.message for check in checks)
    assert packaging.submission_blockers(distribution_tree, "homebrew")


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("kind", "unrelated_operation"),
        ("result", "failed"),
        ("exit_code", 1),
        ("exit_code", False),
        ("version", "9.9.9"),
        ("source_commit", "bad-commit"),
        ("subjects", []),
    ],
)
def test_receipts_require_successful_bound_results(
    distribution_tree: Path,
    field: str,
    value: object,
) -> None:
    _qualify_release(distribution_tree)
    path = distribution_tree / "packaging/release-evidence.json"
    release = json.loads(path.read_text())
    receipt = release["verification_receipts"]["provenance"]
    receipt_path = distribution_tree / receipt["path"]
    payload = json.loads(receipt_path.read_text())
    payload[field] = value
    receipt_path.write_text(json.dumps(payload))
    receipt["sha256"] = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
    path.write_text(json.dumps(release))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and field in check.message for check in checks)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("platform", "windows-x86_64"),
        ("sha256", "0" * 64),
        ("sha256", hashlib.sha256(b"unrelated bytes").hexdigest()),
        ("asset", "fyi-cli-windows-amd64.exe.zip"),
    ],
)
def test_smoke_samples_must_bind_channel_artifact(
    distribution_tree: Path,
    field: str,
    value: str,
) -> None:
    _record_smoke(distribution_tree)
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    payload["targets"][0]["evidence"]["samples"][0][field] = value
    path.write_text(json.dumps(payload))
    assert validate_smokes(distribution_tree)


def test_smoke_requires_every_advertised_platform(distribution_tree: Path) -> None:
    _record_smoke(distribution_tree)
    path = distribution_tree / "packaging/install-smoke-matrix.json"
    payload = json.loads(path.read_text())
    payload["targets"][0]["evidence"]["samples"].pop()
    path.write_text(json.dumps(payload))
    assert any("every advertised platform" in error for error in validate_smokes(distribution_tree))


def test_native_build_subjects_can_qualify_a_container_packet(distribution_tree: Path) -> None:
    _record_native_artifacts(distribution_tree)
    _qualify_release(distribution_tree, "ghcr")
    assert validate_smokes(distribution_tree) == []
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert all(check.ok for check in checks), checks
    assert packaging.submission_blockers(distribution_tree, "ghcr") == []


def test_native_smoke_cannot_reuse_release_archive_digest(distribution_tree: Path) -> None:
    _record_native_artifacts(distribution_tree)
    _record_smoke(distribution_tree, "ghcr")
    path = distribution_tree / "packaging/release-evidence.json"
    payload = json.loads(path.read_text())
    payload["native_artifacts"][0]["sha256"] = payload["assets"][0]["sha256"]
    path.write_text(json.dumps(payload))
    assert any("must not reuse" in error for error in validate_smokes(distribution_tree))


def test_native_smoke_requires_build_verification_receipt(distribution_tree: Path) -> None:
    _record_native_artifacts(distribution_tree)
    _record_smoke(distribution_tree, "ghcr")
    path = distribution_tree / "packaging/release-evidence.json"
    payload = json.loads(path.read_text())
    payload["native_artifacts"][0].pop("verification_receipt")
    path.write_text(json.dumps(payload))
    assert any("receipt" in error for error in validate_smokes(distribution_tree))


def test_snap_offline_toolchain_checksum_is_required(distribution_tree: Path) -> None:
    path = distribution_tree / "packaging/snap/snapcraft.yaml"
    payload = json.loads(path.read_text())
    payload["parts"]["rust-toolchain"].pop("source-checksum")
    path.write_text(json.dumps(payload))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "Snap source checksum" in check.message for check in checks)


def test_disabled_binstall_policy_cannot_claim_readiness(distribution_tree: Path) -> None:
    _qualify_release(distribution_tree, "cargo-binstall")
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "disabled" in check.message for check in checks)


def test_asdf_readiness_requires_release_bound_checksum_approval(distribution_tree: Path) -> None:
    _qualify_release(distribution_tree, "asdf-mise")
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "checksum approval" in check.message for check in checks)
    release = json.loads((distribution_tree / "packaging/release-evidence.json").read_text())
    path = distribution_tree / "packaging/asdf/checksums/0.1.2.sha256"
    path.parent.mkdir(exist_ok=True)
    path.write_text(
        "# release-tag: v0.1.2\n# binary-version: 0.1.2\n"
        + "".join(
            f"{asset['sha256']}  {asset['name']}\n"
            for asset in release["assets"]
            if asset["name"] in HOME_BREW_ARTIFACTS.values()
        ),
    )
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert all(check.ok for check in checks), checks
    path.write_text(path.read_text().replace(release["assets"][0]["sha256"], "0" * 64))
    checks = packaging.check_distribution_contracts(distribution_tree, "0.1.2")
    assert any(not check.ok and "checksum approval" in check.message for check in checks)
