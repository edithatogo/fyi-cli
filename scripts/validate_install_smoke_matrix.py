#!/usr/bin/env python3
"""Validate offline clean-install contracts and any recorded smoke transcripts."""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from datetime import datetime
from pathlib import Path
from typing import Any

try:
    from scripts.validate_registry_submission_ledger import (
        BINARY_CHANNEL_PLATFORMS,
        CLI_BINARY_ASSETS,
        NATIVE_CHANNEL_PLATFORMS,
        PACKAGE_TARGETS,
        is_review_date,
        load_ledger,
        native_artifact_inventory,
        native_artifact_receipt_errors,
        reject_metadata,
        release_asset_subjects,
        repository_file,
        verification_receipt_errors,
    )
except ModuleNotFoundError:
    from validate_registry_submission_ledger import (
        BINARY_CHANNEL_PLATFORMS,
        CLI_BINARY_ASSETS,
        NATIVE_CHANNEL_PLATFORMS,
        PACKAGE_TARGETS,
        is_review_date,
        load_ledger,
        native_artifact_inventory,
        native_artifact_receipt_errors,
        reject_metadata,
        release_asset_subjects,
        repository_file,
        verification_receipt_errors,
    )

REQUIRED_KEYS = {
    "id",
    "asset",
    "status",
    "smoke",
    "requires_release_asset",
    "remote_writes",
    "clean_environment",
    "network_during_smoke",
    "timeout_seconds",
    "expected_exit_code",
    "evidence",
}
ALLOWED_STATUSES = {"release-gated", "verified"}
TARGETS = PACKAGE_TARGETS
ASSETS = {
    "homebrew": "packaging/homebrew/fyi-cli.rb",
    "chocolatey": "packaging/chocolatey/fyi-cli.nuspec",
    "scoop": "packaging/scoop/fyi-cli.json",
    "winget": "packaging/winget/edithatogo.fyi-cli.yaml",
    "cargo-binstall": "packaging/cargo-binstall/metadata.toml",
    "aur": "packaging/aur/PKGBUILD",
    "nixpkgs": "packaging/nix/default.nix",
    "snap": "packaging/snap/snapcraft.yaml",
    "flatpak": "packaging/flatpak/io.github.edithatogo.fyi-cli.yml",
    "asdf-mise": "packaging/asdf/bin/install",
    "debian": "packaging/debian/control",
    "fedora-copr": "packaging/fedora/fyi-cli.spec",
    "crates-io": "crates/fyi-cli/Cargo.toml",
    "ghcr": "Dockerfile",
}
HELP_COMMANDS = dict.fromkeys(TARGETS, "fyi-cli --help")
HELP_COMMANDS.update(dict.fromkeys({"chocolatey", "scoop", "winget"}, "fyi-cli.exe --help"))
HELP_COMMANDS["ghcr"] = "fyi-mcp --help"


def _verification_date(value: object) -> bool:
    if is_review_date(value):
        return True
    if not isinstance(value, str):
        return False
    try:
        parsed = datetime.fromisoformat(value)
    except ValueError:
        return False
    return parsed.tzinfo is not None


def _smoke_release(root: Path, version: object) -> dict[str, Any]:
    release = load_ledger(repository_file(root, "packaging/release-evidence.json"))
    if release.get("component_version") != version:
        reject_metadata("smoke release evidence must match source_release")
    commit = release.get("source_commit")
    if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
        reject_metadata("smoke release evidence requires a full source commit")
    return release


def _smoke_subjects(release: dict[str, Any], target_id: str) -> dict[str, dict[str, Any]]:
    assets = release_asset_subjects(release)
    if target_id in BINARY_CHANNEL_PLATFORMS:
        subjects: dict[str, dict[str, Any]] = {}
        for platform in BINARY_CHANNEL_PLATFORMS[target_id]:
            name = CLI_BINARY_ASSETS[platform]
            if name not in assets:
                reject_metadata(f"missing release artifact for smoke platform {platform}")
            subjects[platform] = {"name": name, "sha256": assets[name]}
        return subjects
    native = native_artifact_inventory(release)
    if any((target_id, platform) not in native for platform in NATIVE_CHANNEL_PLATFORMS[target_id]):
        reject_metadata("smoke requires native_artifacts for every advertised platform")
    return {
        platform: native[(target_id, platform)] for platform in NATIVE_CHANNEL_PLATFORMS[target_id]
    }


def _transcript_errors(
    root: Path,
    sample: dict[str, Any],
    target: dict[str, Any],
    release: dict[str, Any],
    artifact: dict[str, Any],
) -> list[str]:
    return verification_receipt_errors(
        root,
        {
            "path": sample.get("transcript"),
            "sha256": sample.get("transcript_sha256"),
            "version": release["component_version"],
            "source_commit": release["source_commit"],
        },
        context={
            "kind": "clean_install",
            "target": target["id"],
            "platform": sample["platform"],
            "version": release["component_version"],
            "source_commit": release["source_commit"],
            "command": target.get("smoke"),
            "clean_environment": True,
            "network_during_smoke": False,
            "remote_writes": False,
            "timeout_seconds": target.get("timeout_seconds"),
            "uninstalled": True,
        },
        subjects={artifact["name"]: artifact["sha256"]},
    )


def _sample_errors(
    root: Path,
    sample: dict[str, Any],
    target: dict[str, Any],
    release: dict[str, Any],
    artifact: dict[str, Any],
) -> list[str]:
    errors = []
    if sample.get("asset") != artifact["name"] or sample.get("sha256") != artifact["sha256"]:
        errors.append("sample must bind its channel/platform artifact name and digest")
    if not _verification_date(sample.get("verified_at")):
        errors.append("sample verified_at must be an ISO date or zoned timestamp")
    if target["id"] in NATIVE_CHANNEL_PLATFORMS:
        errors.extend(native_artifact_receipt_errors(root, artifact))
    errors.extend(_transcript_errors(root, sample, target, release, artifact))
    return errors


def _samples_errors(
    root: Path,
    target: dict[str, Any],
    release: dict[str, Any],
    subjects: dict[str, dict[str, Any]],
) -> list[str]:
    samples = target["evidence"].get("samples")
    if not isinstance(samples, list) or not samples:
        return ["evidence.samples must cover every advertised platform"]
    errors = []
    seen: set[str] = set()
    for sample in samples:
        if not isinstance(sample, dict):
            errors.append("evidence.samples entries must be objects")
            continue
        platform = sample.get("platform")
        if not isinstance(platform, str) or platform not in subjects or platform in seen:
            errors.append("sample platform must be supported by this channel and unique")
            continue
        seen.add(platform)
        errors.extend(_sample_errors(root, sample, target, release, subjects[platform]))
    if seen != set(subjects):
        errors.append("evidence.samples must cover every advertised platform")
    return errors


def _evidence_errors(
    target: dict[str, Any],
    root: Path,
    version: object,
    prefix: str,
) -> list[str]:
    evidence = target.get("evidence")
    if target.get("status") != "verified":
        return [] if evidence is None else [f"{prefix}.evidence requires verified status"]
    if not isinstance(evidence, dict):
        return [f"{prefix}.evidence must bind version, source_commit and samples"]
    errors: list[str] = []
    try:
        release = _smoke_release(root, version)
        subjects = _smoke_subjects(release, target["id"])
    except (OSError, ValueError) as error:
        return [f"{prefix}.evidence cannot bind artifact subjects: {error}"]
    if (
        evidence.get("version") != version
        or evidence.get("source_commit") != release["source_commit"]
    ):
        errors.append("evidence must bind the observed version and source commit")
    errors.extend(_samples_errors(root, target, release, subjects))
    return [f"{prefix}.{error}" for error in errors]


def _safety_errors(target: dict[str, Any], prefix: str) -> list[str]:
    errors: list[str] = []
    for key, required in {
        "remote_writes": False,
        "requires_release_asset": True,
        "clean_environment": True,
        "network_during_smoke": False,
    }.items():
        if target.get(key) is not required:
            errors.append(f"{prefix}.{key} must be {str(required).lower()}")
    timeout = target.get("timeout_seconds")
    if type(timeout) is not int or not 0 < timeout <= 60:
        errors.append(f"{prefix}.timeout_seconds must be an integer in 1..60")
    if type(target.get("expected_exit_code")) is not int or target["expected_exit_code"] != 0:
        errors.append(f"{prefix}.expected_exit_code must be 0")
    return errors


def _target_errors(
    target: dict[str, Any],
    root: Path,
    version: object,
    prefix: str,
) -> list[str]:
    errors = [f"{prefix} missing {key}" for key in sorted(REQUIRED_KEYS - target.keys())]
    target_id = target.get("id")
    status = target.get("status")
    if not isinstance(status, str) or status not in ALLOWED_STATUSES:
        errors.append(f"{prefix}.status is unsupported")
    if not isinstance(target_id, str) or target_id not in TARGETS:
        return [*errors, f"{prefix}.id is unsupported"]
    if target.get("smoke") != HELP_COMMANDS[target_id]:
        errors.append(f"{prefix}.smoke must be the channel's exact help-only command")
    try:
        repository_file(root, target.get("asset"))
    except (OSError, ValueError) as error:
        errors.append(f"{prefix}.asset is invalid: {error}")
    if target.get("asset") != ASSETS[target_id]:
        errors.append(f"{prefix}.asset must identify the channel's package manifest")
    errors.extend(_safety_errors(target, prefix))
    errors.extend(_evidence_errors(target, root, version, prefix))
    return errors


def _version_errors(payload: dict[str, Any], root: Path) -> list[str]:
    try:
        path = repository_file(root, "crates/fyi-cli/Cargo.toml")
        package = tomllib.loads(path.read_text(encoding="utf-8")).get("package")
        if not isinstance(package, dict) or not isinstance(package.get("version"), str):
            return ["source_release cannot be bound to the CLI package version"]
        if payload.get("source_release") != package["version"]:
            return ["source_release must match the CLI package version"]
    except (OSError, ValueError) as error:
        return [f"source_release cannot be checked: {error}"]
    return []


def validate(repo_root: Path) -> list[str]:
    """Check metadata only; this function never installs or runs a package."""
    try:
        payload = load_ledger(repository_file(repo_root, "packaging/install-smoke-matrix.json"))
    except (OSError, ValueError) as error:
        return [f"install-smoke matrix is unreadable: {error}"]
    errors = _version_errors(payload, repo_root)
    if type(payload.get("schema_version")) is not int or payload["schema_version"] != 1:
        errors.append("schema_version must be 1")
    targets = payload.get("targets")
    if not isinstance(targets, list) or not targets:
        return [*errors, "targets must be a non-empty list"]
    seen: set[str] = set()
    for index, target in enumerate(targets):
        prefix = f"targets[{index}]"
        if not isinstance(target, dict):
            errors.append(f"{prefix} must be an object")
            continue
        target_id = target.get("id")
        if not isinstance(target_id, str) or not target_id or target_id in seen:
            errors.append(f"{prefix}.id must be unique and non-empty")
        else:
            seen.add(target_id)
        errors.extend(_target_errors(target, repo_root, payload.get("source_release"), prefix))
    if seen != TARGETS:
        errors.append("install-smoke targets must match the fourteen distribution channels")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    errors = validate(args.repo_root)
    if errors:
        sys.stdout.write("".join(f"ERROR: {error}\n" for error in errors))
        return 1
    sys.stdout.write(
        "Install-smoke matrix valid: fourteen contracts; execution evidence is separate\n",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
