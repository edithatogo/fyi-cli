#!/usr/bin/env python3
"""Validate registry identities, readiness claims, and their evidence references."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import date
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING, Any
from urllib.parse import urlsplit

if TYPE_CHECKING:
    from typing import NoReturn

STATUSES = {"planned", "assets-ready", "submitted", "live", "blocked-external", "not-applicable"}
READY_STATUSES = {"assets-ready", "submitted", "live"}
CLASSES = {"ai-plugin", "mcp-registry", "mcp-catalog", "community-catalog", "package", "container"}
PACKAGE_TARGETS = {
    "crates-io",
    "ghcr",
    "homebrew",
    "chocolatey",
    "scoop",
    "winget",
    "cargo-binstall",
    "aur",
    "nixpkgs",
    "snap",
    "flatpak",
    "asdf-mise",
    "debian",
    "fedora-copr",
}
CANONICAL_PACKET_ASSETS = {
    "crates-io": {
        "crates/fyi-core/Cargo.toml",
        "crates/fyi-cli/Cargo.toml",
        "crates/fyi-mcp/Cargo.toml",
    },
    "ghcr": {"Dockerfile", "scripts/verify_ghcr_multarch.py"},
    "homebrew": {"packaging/homebrew/fyi-cli.rb"},
    "chocolatey": {
        "packaging/chocolatey/fyi-cli.nuspec",
        "packaging/chocolatey/tools/chocolateyinstall.ps1",
        "packaging/chocolatey/tools/chocolateyuninstall.ps1",
        "packaging/chocolatey/tools/LICENSE.txt",
        "packaging/chocolatey/tools/VERIFICATION.txt",
    },
    "scoop": {"packaging/scoop/fyi-cli.json"},
    "winget": {"packaging/winget/edithatogo.fyi-cli.yaml"},
    "cargo-binstall": {
        "packaging/cargo-binstall/metadata.toml",
        "crates/fyi-cli/Cargo.toml",
        "crates/fyi-mcp/Cargo.toml",
    },
    "aur": {"packaging/aur/PKGBUILD"},
    "nixpkgs": {"packaging/nix/default.nix"},
    "snap": {"packaging/snap/snapcraft.yaml"},
    "flatpak": {
        "packaging/flatpak/io.github.edithatogo.fyi-cli.yml",
        "packaging/flatpak/io.github.edithatogo.fyi-cli.metainfo.xml",
    },
    "asdf-mise": {
        "packaging/asdf/bin/install",
        "packaging/asdf/bin/list-all",
        "packaging/asdf/README.md",
        "packaging/mise/backend.toml",
        "packaging/mise/README.md",
    },
    "debian": {
        "packaging/debian/control",
        "packaging/debian/rules",
        "packaging/debian/changelog",
        "packaging/debian/copyright",
        "packaging/debian/source/format",
    },
    "fedora-copr": {"packaging/fedora/fyi-cli.spec"},
}
CLI_BINARY_ASSETS = {
    "linux-x86_64": "fyi-cli-linux-amd64.tar.gz",
    "macos-x86_64": "fyi-cli-macos-amd64.tar.gz",
    "macos-arm64": "fyi-cli-macos-arm64.tar.gz",
    "windows-x86_64": "fyi-cli-windows-amd64.exe.zip",
}
UNIX_BINARY_PLATFORMS = ("linux-x86_64", "macos-x86_64", "macos-arm64")
BINARY_CHANNEL_PLATFORMS = {
    "homebrew": UNIX_BINARY_PLATFORMS,
    "chocolatey": ("windows-x86_64",),
    "scoop": ("windows-x86_64",),
    "winget": ("windows-x86_64",),
    "cargo-binstall": tuple(CLI_BINARY_ASSETS),
    "asdf-mise": UNIX_BINARY_PLATFORMS,
}
NATIVE_CHANNEL_PLATFORMS = dict.fromkeys(
    {"ghcr", "aur", "nixpkgs", "snap", "flatpak", "debian", "fedora-copr"},
    ("linux-x86_64", "linux-aarch64"),
)
NATIVE_CHANNEL_PLATFORMS["crates-io"] = (*CLI_BINARY_ASSETS, "linux-aarch64")
STABLE_VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
MAX_METADATA_BYTES = 4 * 1024 * 1024


def reject_metadata(message: str) -> NoReturn:
    """Report malformed data consistently without hiding parsing failures."""
    raise ValueError(message)


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    data: dict[str, Any] = {}
    for key, value in pairs:
        if key in data:
            reject_metadata(f"duplicate JSON field: {key}")
        data[key] = value
    return data


def parse_json_object(text: str) -> dict[str, Any]:
    """Reject duplicate fields, non-finite numbers, and non-object documents."""
    if len(text.encode("utf-8")) > MAX_METADATA_BYTES:
        reject_metadata("metadata exceeds the size limit")
    try:
        value = json.loads(text, object_pairs_hook=_unique_object, parse_constant=reject_metadata)
    except RecursionError:
        reject_metadata("metadata nesting is too deep")
    if not isinstance(value, dict):
        reject_metadata("metadata must be a JSON object")
    return value


def load_ledger(path: Path) -> dict[str, Any]:
    """Read a bounded UTF-8 JSON object without accepting ambiguous fields."""
    if path.stat().st_size > MAX_METADATA_BYTES:
        reject_metadata("metadata exceeds the size limit")
    return parse_json_object(path.read_text(encoding="utf-8"))


def is_public_url(value: object) -> bool:
    """Recognise credential-free HTTP(S) evidence references without fetching them."""
    if not isinstance(value, str) or any(character.isspace() for character in value):
        return False
    try:
        parsed = urlsplit(value)
        _ = parsed.port
        return (
            parsed.scheme in {"http", "https"}
            and bool(parsed.hostname)
            and parsed.username is None
            and parsed.password is None
        )
    except ValueError:
        return False


def is_review_date(value: object) -> bool:
    """Check a real, canonical ISO calendar date."""
    if not isinstance(value, str) or re.fullmatch(r"\d{4}-\d{2}-\d{2}", value) is None:
        return False
    try:
        date.fromisoformat(value)
    except ValueError:
        return False
    return True


def is_repository_path(value: object) -> bool:
    """Reject absolute, traversal, and platform-dependent metadata paths."""
    if not isinstance(value, str) or not value or "\\" in value or ":" in value:
        return False
    if any(ord(character) < 32 for character in value):
        return False
    return not PurePosixPath(value).is_absolute() and all(
        part not in {"", ".", ".."} for part in value.split("/")
    )


def repository_file(repo_root: Path, relative: object) -> Path:
    """Resolve a local evidence file without reading outside the repository."""
    if not is_repository_path(relative):
        reject_metadata("asset must use a safe repository-relative path")
    path = repo_root / str(relative)
    if not path.resolve().is_relative_to(repo_root.resolve()) or not path.is_file():
        reject_metadata("asset is missing or outside the repository")
    return path


def is_sha256(value: object) -> bool:
    """Require a lowercase SHA-256 value, excluding repeated-digit placeholders."""
    return (
        isinstance(value, str)
        and re.fullmatch(r"[0-9a-f]{64}", value) is not None
        and len(set(value)) > 1
    )


def _subject(value: object) -> tuple[str, str]:
    if not isinstance(value, dict):
        reject_metadata("artifact subject must be an object")
    name = value.get("name")
    if not isinstance(name, str) or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", name) is None:
        reject_metadata("artifact subject requires a safe unique name")
    digest = value.get("sha256")
    if not is_sha256(digest):
        reject_metadata("artifact subject requires a non-placeholder SHA-256 digest")
    return name, str(digest)


def release_asset_subjects(evidence: dict[str, Any]) -> dict[str, str]:
    """Return recorded release subjects; this does not authenticate their bytes."""
    values = evidence.get("assets")
    if not isinstance(values, list) or not values:
        reject_metadata("release assets must be a non-empty list")
    subjects: dict[str, str] = {}
    for value in values:
        name, digest = _subject(value)
        if name in subjects:
            reject_metadata("release assets contain a duplicate subject")
        subjects[name] = digest
    return subjects


def native_artifact_inventory(evidence: dict[str, Any]) -> dict[tuple[str, str], dict[str, Any]]:
    """Bind optional native build subjects to their channel, platform and source."""
    values = evidence.get("native_artifacts", [])
    if not isinstance(values, list):
        reject_metadata("native_artifacts must be a list")
    release = release_asset_subjects(evidence)
    names = set(release)
    inventory: dict[tuple[str, str], dict[str, Any]] = {}
    for value in values:
        name, digest = _subject(value)
        target = value.get("target")
        platform = value.get("platform")
        if not isinstance(target, str) or target not in NATIVE_CHANNEL_PLATFORMS:
            reject_metadata("native artifact target must be a source-build/container channel")
        if not isinstance(platform, str) or platform not in NATIVE_CHANNEL_PLATFORMS[target]:
            reject_metadata("native artifact platform is unsupported for its channel")
        if (target, platform) in inventory or name in names:
            reject_metadata("native artifacts must have unique subjects and channel/platform pairs")
        if digest in release.values():
            reject_metadata("native artifact must not reuse a release archive/source digest")
        identity = (value.get("version"), value.get("source_commit"))
        if identity != (evidence.get("component_version"), evidence.get("source_commit")):
            reject_metadata("native artifact must bind the observed version and source commit")
        names.add(name)
        inventory[(target, platform)] = value
    return inventory


def _receipt_subject_errors(payload: dict[str, Any], expected: dict[str, str]) -> list[str]:
    values = payload.get("subjects")
    if not isinstance(values, list) or not values:
        return ["verification receipt subjects must be a non-empty list"]
    subjects: dict[str, str] = {}
    for value in values:
        try:
            name, digest = _subject(value)
        except ValueError as error:
            return [f"verification receipt subject is invalid: {error}"]
        if name in subjects:
            return ["verification receipt subjects must not contain duplicates"]
        subjects[name] = digest
    if subjects != expected:
        return ["verification receipt subjects must exactly bind the required artifact digests"]
    return []


def _receipt_file(root: Path, value: dict[str, Any]) -> dict[str, Any]:
    if not is_sha256(value.get("sha256")):
        reject_metadata("verification receipt sha256 must bind its bytes")
    path = repository_file(root, value.get("path"))
    if not 0 < path.stat().st_size <= MAX_METADATA_BYTES:
        reject_metadata("verification receipt must contain at most 4 MiB of evidence")
    content = path.read_bytes()
    if hashlib.sha256(content).hexdigest() != value["sha256"]:
        reject_metadata("verification receipt digest differs from its bytes")
    return parse_json_object(content.decode("utf-8"))


def verification_receipt_errors(
    root: Path,
    value: object,
    *,
    context: dict[str, Any],
    subjects: dict[str, str],
) -> list[str]:
    """Check operator record consistency, without authenticating signatures or execution."""
    if not isinstance(value, dict):
        return ["verification receipt must identify path, sha256, version and source_commit"]
    if any(value.get(key) != context[key] for key in ("version", "source_commit")):
        return ["verification receipt must bind the observed version and source commit"]
    try:
        payload = _receipt_file(root, value)
    except (OSError, ValueError) as error:
        return [f"verification receipt is invalid: {error}"]
    errors: list[str] = []
    expected = {"schema_version": 1, "result": "passed", "exit_code": 0, **context}
    for key, required in expected.items():
        if type(payload.get(key)) is not type(required) or payload.get(key) != required:
            errors.append(f"verification receipt {key} must match {required!r}")
    errors.extend(_receipt_subject_errors(payload, subjects))
    return errors


def native_artifact_receipt_errors(root: Path, artifact: dict[str, Any]) -> list[str]:
    """Require a separate successful verification record for each native build subject."""
    return verification_receipt_errors(
        root,
        artifact.get("verification_receipt"),
        context={
            "kind": "native_artifact",
            "target": artifact["target"],
            "platform": artifact["platform"],
            "version": artifact["version"],
            "source_commit": artifact["source_commit"],
        },
        subjects={artifact["name"]: artifact["sha256"]},
    )


def _readiness_errors(target: dict[str, Any], prefix: str) -> list[str]:
    errors: list[str] = []
    blockers = target.get("blockers")
    if not isinstance(blockers, list) or any(
        not isinstance(item, str) or not item.strip() for item in blockers
    ):
        errors.append(f"{prefix}.blockers must be a list of non-empty strings")
    status = target.get("status")
    status_ready = isinstance(status, str) and status in READY_STATUSES
    ready = target.get("submission_ready")
    if type(ready) is not bool:
        errors.append(f"{prefix}.submission_ready must be boolean")
    elif ready != status_ready:
        errors.append(f"{prefix}.submission_ready conflicts with status")
    if (ready is True or status_ready) and blockers:
        errors.append(f"{prefix}.blockers prevent a ready/submitted/live claim")
    if status == "blocked-external" and not blockers:
        errors.append(f"{prefix}.blockers must explain blocked-external status")
    if status == "live" and not is_public_url(target.get("public_install_evidence")):
        errors.append(f"{prefix}.public_install_evidence must be a public install evidence URL")
    return errors


def _validate_package(target: dict[str, Any], prefix: str) -> list[str]:
    errors = _readiness_errors(target, prefix)
    maintainer = target.get("maintainer")
    if not isinstance(maintainer, str) or not maintainer.strip():
        errors.append(f"{prefix}.maintainer must be non-empty")
    if not is_review_date(target.get("checked_at")):
        errors.append(f"{prefix}.checked_at must be a canonical ISO date")
    if not is_public_url(target.get("submission_url")):
        errors.append(f"{prefix}.submission_url must be a public HTTP(S) URL")
    assets = target.get("packet_assets")
    if not isinstance(assets, list) or not assets or not all(map(is_repository_path, assets)):
        errors.append(f"{prefix}.packet_assets must list safe repository paths")
    elif len(assets) != len(set(assets)):
        errors.append(f"{prefix}.packet_assets must not contain duplicates")
    elif missing := CANONICAL_PACKET_ASSETS[str(target["id"])] - set(assets):
        errors.append(
            f"{prefix}.packet_assets omit canonical package files: {', '.join(sorted(missing))}",
        )
    return errors


def _target_errors(target: dict[str, Any], prefix: str) -> list[str]:
    errors: list[str] = []
    target_class = target.get("class")
    if not isinstance(target_class, str) or target_class not in CLASSES:
        errors.append(f"{prefix}.class is unsupported")
    status = target.get("status")
    if not isinstance(status, str) or status not in STATUSES:
        errors.append(f"{prefix}.status is unsupported")
    next_action = target.get("next_action")
    if not isinstance(next_action, str) or not next_action.strip():
        errors.append(f"{prefix}.next_action must be non-empty")
    evidence = target.get("evidence")
    needs_evidence = isinstance(status, str) and status in {"live", "submitted", "blocked-external"}
    if needs_evidence and not is_public_url(evidence):
        errors.append(f"{prefix}.evidence is required for status {status}")
    elif evidence is not None and not is_public_url(evidence):
        errors.append(f"{prefix}.evidence must be an HTTP(S) URL")
    if target_class == "ai-plugin" and not is_public_url(target.get("submission_url")):
        errors.append(f"{prefix}.submission_url is required for tracked AI plugins")
    target_id = target.get("id")
    if isinstance(target_id, str) and target_id in PACKAGE_TARGETS:
        errors.extend(_validate_package(target, prefix))
    return errors


def validate(ledger: object) -> list[str]:
    """Validate records without treating any local status as publication proof."""
    if not isinstance(ledger, dict):
        return ["ledger must be an object"]
    errors: list[str] = []
    if type(ledger.get("schema_version")) is not int or ledger["schema_version"] != 1:
        errors.append("schema_version must be 1")
    version = ledger.get("source_release")
    if not isinstance(version, str) or STABLE_VERSION.fullmatch(version) is None:
        errors.append("source_release must be a stable version")
    targets = ledger.get("targets")
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
        errors.extend(_target_errors(target, prefix))
    missing = PACKAGE_TARGETS - seen
    if missing:
        errors.append(f"missing package targets: {', '.join(sorted(missing))}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--path", type=Path, default=Path("packaging/registry-submissions.json"))
    args = parser.parse_args()
    try:
        ledger = load_ledger(args.path)
        errors = validate(ledger)
    except (OSError, ValueError) as error:
        errors = [f"registry ledger is unreadable: {error}"]
    if errors:
        sys.stdout.write("".join(f"ERROR: {error}\n" for error in errors))
        return 1
    sys.stdout.write(f"Registry ledger valid: {len(ledger['targets'])} targets\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
