#!/usr/bin/env python3
"""Verify packaging / multi-registry distribution assets exist and versions align.

Run from repo root (or any cwd with --repo-root):

    python scripts/verify_packaging_assets.py
    python scripts/verify_packaging_assets.py --json
    python scripts/verify_packaging_assets.py --expected-version 0.1.2

Exit codes:
    0 — all critical assets present; version checks pass (warnings allowed)
    1 — missing critical asset(s) and/or version mismatch failures
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
import xml.etree.ElementTree as ET
from dataclasses import asdict, dataclass, field, replace
from pathlib import Path
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Iterable

try:
    from scripts.validate_install_smoke_matrix import validate as validate_smokes
    from scripts.validate_registry_submission_ledger import (
        CLI_BINARY_ASSETS,
        PACKAGE_TARGETS,
        STABLE_VERSION,
        is_review_date,
        is_sha256,
        load_ledger,
        native_artifact_inventory,
        native_artifact_receipt_errors,
        parse_json_object,
        reject_metadata,
        release_asset_subjects,
        repository_file,
        verification_receipt_errors,
    )
    from scripts.validate_registry_submission_ledger import (
        validate as validate_ledger,
    )
except ModuleNotFoundError:
    from validate_install_smoke_matrix import validate as validate_smokes
    from validate_registry_submission_ledger import (
        CLI_BINARY_ASSETS,
        PACKAGE_TARGETS,
        STABLE_VERSION,
        is_review_date,
        is_sha256,
        load_ledger,
        native_artifact_inventory,
        native_artifact_receipt_errors,
        parse_json_object,
        reject_metadata,
        release_asset_subjects,
        repository_file,
        verification_receipt_errors,
    )
    from validate_registry_submission_ledger import (
        validate as validate_ledger,
    )

# ---------------------------------------------------------------------------
# Asset inventory (paths relative to repo root)
# ---------------------------------------------------------------------------

# Critical packaging files: absence fails the check.
CRITICAL_ASSETS: tuple[str, ...] = (
    "packaging/release-evidence.json",
    "packaging/registry-submissions.json",
    "packaging/install-smoke-matrix.json",
    "packaging/release-compatibility.json",
    "packaging/aur/PKGBUILD",
    "packaging/nix/default.nix",
    "packaging/snap/snapcraft.yaml",
    "packaging/flatpak/io.github.edithatogo.fyi-cli.yml",
    "packaging/flatpak/io.github.edithatogo.fyi-cli.metainfo.xml",
    "packaging/asdf/bin/install",
    "packaging/asdf/bin/list-all",
    "packaging/debian/control",
    "packaging/fedora/fyi-cli.spec",
    "packaging/scoop/fyi-cli.json",
    "packaging/winget/edithatogo.fyi-cli.yaml",
    "packaging/homebrew/fyi-cli.rb",
    "packaging/chocolatey/fyi-cli.nuspec",
    "packaging/chocolatey/tools/chocolateyinstall.ps1",
    "packaging/cargo-binstall/metadata.toml",
    "packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md",
    "packaging/mcp-catalogs/pulsemcp/submission.json",
    "packaging/mcp-catalogs/pulsemcp/README.md",
    "packaging/mcp-catalogs/mcp-so/listing.md",
    "packaging/mcp-catalogs/mcp-so/README.md",
    "packaging/mcp-catalogs/docker-mcp/README.md",
    "packaging/mcp-catalogs/mcp-get/README.md",
    "packaging/mcp-catalogs/opentools/README.md",
)

# Helpful but non-blocking presence checks.
OPTIONAL_ASSETS: tuple[str, ...] = (
    "packaging/debian/changelog",
    "packaging/debian/rules",
    "packaging/debian/copyright",
    "packaging/chocolatey/tools/chocolateyinstall.ps1",
    "packaging/asdf/README.md",
    "packaging/mise/backend.toml",
    "packaging/mcpb/fyi-mcp/manifest.json",
    "server.json",
    "Dockerfile",
    ".github/workflows/release.yml",
    ".github/workflows/container-publish.yml",
    "docs/registry-distribution-matrix.md",
    "docs/release-multi-registry.md",
)
BINSTALL_METADATA_ASSETS: tuple[str, ...] = (
    "crates/fyi-cli/Cargo.toml",
    "crates/fyi-mcp/Cargo.toml",
)
README_PATH = "README.md"
README_BINSTALL_BLOCKED_PHRASE = "| **cargo-binstall** | Blocked |"

# Files expected to embed a concrete release version string (e.g. 0.1.2).
# Paths relative to repo root. Checked only when the file exists.
VERSIONED_ASSETS: tuple[str, ...] = (
    "packaging/aur/PKGBUILD",
    "packaging/nix/default.nix",
    "packaging/snap/snapcraft.yaml",
    "packaging/flatpak/io.github.edithatogo.fyi-cli.yml",
    "packaging/flatpak/io.github.edithatogo.fyi-cli.metainfo.xml",
    "packaging/fedora/fyi-cli.spec",
    "packaging/scoop/fyi-cli.json",
    "packaging/winget/edithatogo.fyi-cli.yaml",
    "packaging/homebrew/fyi-cli.rb",
    "packaging/chocolatey/fyi-cli.nuspec",
    "packaging/chocolatey/tools/chocolateyinstall.ps1",
    "packaging/debian/changelog",
    "packaging/mcpb/fyi-mcp/manifest.json",
    "packaging/mcp-catalogs/pulsemcp/submission.json",
    "packaging/mcp-catalogs/mcp-so/listing.md",
    "packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md",
    "server.json",
    "crates/fyi-cli/Cargo.toml",
    "crates/fyi-mcp/Cargo.toml",
    "crates/fyi-core/Cargo.toml",
)

DEFAULT_EXPECTED_VERSION = "0.1.2"
_CRATE_VERSION_RE = re.compile(
    r'(?m)^version\s*=\s*"(?P<version>\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.]+)?)"\s*$',
)
_SEMVER_RE = re.compile(r"\b(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.]+)?)\b")


@dataclass(frozen=True)
class CheckResult:
    """One reproducible packaging check outcome."""

    path: str
    kind: str
    ok: bool
    message: str
    severity: str = "error"


@dataclass
class VerificationReport:
    """Aggregate local validity; publication requires separate operator evidence."""

    expected_version: str
    results: list[CheckResult] = field(default_factory=list)

    @property
    def errors(self) -> list[CheckResult]:
        return [result for result in self.results if not result.ok and result.severity == "error"]

    @property
    def warnings(self) -> list[CheckResult]:
        return [result for result in self.results if not result.ok and result.severity == "warning"]

    @property
    def ok(self) -> bool:
        return not self.errors

    def as_dict(self) -> dict[str, Any]:
        return {
            "expected_version": self.expected_version,
            "ok": self.ok,
            "error_count": len(self.errors),
            "warning_count": len(self.warnings),
            "results": [asdict(result) for result in self.results],
        }


def _failure(path: str, message: str, *, kind: str = "distribution") -> CheckResult:
    return CheckResult(path=path, kind=kind, ok=False, message=message)


def _success(path: str, message: str, *, kind: str = "distribution") -> CheckResult:
    return CheckResult(path=path, kind=kind, ok=True, message=message, severity="info")


def _object(value: object, description: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        reject_metadata(f"{description} must be an object")
    return value


def _read_text(root: Path, relative: str) -> str:
    path = repository_file(root, relative)
    if path.stat().st_size > 4 * 1024 * 1024:
        reject_metadata("metadata exceeds the size limit")
    return path.read_text(encoding="utf-8")


def _json_object(path: Path) -> dict[str, Any]:
    return load_ledger(path)


def discover_expected_version(repo_root: Path) -> str:
    """Read the CLI's package version, retaining the historical absent-file fallback."""
    path = repo_root / "crates/fyi-cli/Cargo.toml"
    if not path.is_file():
        return DEFAULT_EXPECTED_VERSION
    package = _object(tomllib.loads(path.read_text(encoding="utf-8")).get("package"), "package")
    version = package.get("version")
    if not isinstance(version, str) or STABLE_VERSION.fullmatch(version) is None:
        reject_metadata("CLI package version must be a stable semantic version")
    return version


def check_presence(
    repo_root: Path,
    assets: Iterable[str],
    *,
    kind: str,
    severity_if_missing: str,
) -> list[CheckResult]:
    """Require ordinary files within the repository, including resolved links."""
    results: list[CheckResult] = []
    for relative in assets:
        try:
            repository_file(repo_root, relative)
        except (OSError, ValueError) as error:
            results.append(
                CheckResult(
                    path=relative,
                    kind=kind,
                    ok=False,
                    message=str(error),
                    severity=severity_if_missing,
                ),
            )
        else:
            results.append(_success(relative, "present", kind=kind))
    return results


def file_mentions_version(text: str, version: str) -> bool:
    """Check complete version tokens in prose documents only."""
    return version in {match.group(1) for match in _SEMVER_RE.finditer(text)}


def _xml_root(text: str) -> ET.Element:
    if re.search(r"<!\s*(?:DOCTYPE|ENTITY)\b", text, re.IGNORECASE):
        reject_metadata("XML DTD and entity declarations are not allowed")
    # Bounded UTF-8 text; DTD/entity declarations were rejected above.
    return ET.fromstring(text)  # noqa: S314 - declarations blocked before stdlib parsing


def _flatpak_version(data: dict[str, Any]) -> str:
    modules = data.get("modules")
    if not isinstance(modules, list) or len(modules) != 1:
        reject_metadata("Flatpak must identify one CLI module")
    module = _object(modules[0], "Flatpak module")
    sources = module.get("sources")
    if not isinstance(sources, list):
        reject_metadata("Flatpak module sources must be a list")
    versions = []
    for item in sources:
        source = _object(item, "Flatpak source")
        path = source.get("path", "")
        match = re.fullmatch(r"fyi-cli-(\d+\.\d+\.\d+)-vendor\.tar\.gz", str(path))
        if match:
            versions.append(match.group(1))
    return versions[0] if len(versions) == 1 else ""


def declared_version(relative: str, text: str) -> str | None:
    """Read native version fields; only prose documents may return None."""
    if relative.endswith("Cargo.toml"):
        package = _object(tomllib.loads(text).get("package"), "package")
        version = package.get("version")
        return version if isinstance(version, str) else ""
    if relative.endswith((".json", ".yaml", ".yml")):
        # JSON-form YAML is intentional: these files need no third-party parser in CI.
        data = parse_json_object(text)
        if relative == "packaging/flatpak/io.github.edithatogo.fyi-cli.yml":
            return _flatpak_version(data)
        version = data.get("PackageVersion", data.get("version"))
        return version if isinstance(version, str) else ""
    if relative.endswith(".nuspec"):
        return _xml_root(text).findtext("{*}metadata/{*}version", default="")
    if relative.endswith(".metainfo.xml"):
        release = _xml_root(text).find("releases/release")
        return release.get("version", "") if release is not None else ""
    patterns = {
        "PKGBUILD": r"(?m)^pkgver=([^\s]+)$",
        "default.nix": r'(?m)^\s*version = "([^"\n]+)";',
        "fyi-cli.spec": r"(?m)^Version:\s*(\S+)",
        "fyi-cli.rb": r'(?m)^\s*version "([^"\n]+)"$',
        "chocolateyinstall.ps1": r"(?m)^\$version\s*=\s*'([^'\n]+)'$",
        "changelog": r"\Afyi-cli \(([^-]+)-",
    }
    pattern = patterns.get(Path(relative).name)
    if pattern is None:
        return None
    matches = re.findall(pattern, text)
    return matches[0] if len(matches) == 1 else ""


def check_versions(
    repo_root: Path,
    expected_version: str,
    assets: Iterable[str] = VERSIONED_ASSETS,
) -> list[CheckResult]:
    """Check native version fields and reject malformed or ambiguous metadata."""
    results: list[CheckResult] = []
    for relative in assets:
        if not (repo_root / relative).is_file():
            continue  # Presence checks retain their original severity.
        try:
            text = _read_text(repo_root, relative)
            native = declared_version(relative, text)
            matches = (
                native == expected_version
                if native is not None
                else file_mentions_version(
                    text,
                    expected_version,
                )
            )
        except (OSError, ValueError, ET.ParseError, TypeError) as error:
            results.append(_failure(relative, f"malformed metadata: {error}", kind="version"))
            continue
        if matches:
            results.append(_success(relative, f"version {expected_version}", kind="version"))
        else:
            results.append(
                _failure(
                    relative,
                    f"native metadata does not declare version {expected_version}",
                    kind="version",
                ),
            )
    return results


def check_binstall_metadata(
    repo_root: Path,
    assets: Iterable[str] = BINSTALL_METADATA_ASSETS,
) -> list[CheckResult]:
    """Parse the metadata section instead of accepting its name in a comment."""
    results = []
    for relative in assets:
        try:
            package = _object(
                tomllib.loads(_read_text(repo_root, relative)).get("package"),
                "package",
            )
            metadata = _object(package.get("metadata"), "package metadata")
            _object(metadata.get("binstall"), "package.metadata.binstall")
        except (OSError, ValueError, TypeError) as error:
            results.append(
                _failure(
                    relative,
                    f"missing or invalid [package.metadata.binstall]: {error}",
                    kind="binstall",
                ),
            )
        else:
            results.append(_success(relative, "cargo-binstall metadata parsed", kind="binstall"))
    return results


def check_packaging_readme(
    repo_root: Path,
    relative: str | None = README_PATH,
) -> list[CheckResult]:
    """Keep the confirmed cargo-binstall crate-name collision visible."""
    if relative is None:
        return []
    try:
        text = _read_text(repo_root, relative)
    except (OSError, ValueError) as error:
        return [_failure(relative, str(error), kind="docs")]
    errors = []
    if README_BINSTALL_BLOCKED_PHRASE not in text:
        errors.append("missing blocked cargo-binstall row")
    for line in text.splitlines():
        if line.startswith("| **cargo-binstall**") and "`cargo binstall fyi-cli`" in line:
            errors.append("cargo-binstall row recommends a conflicting crate")
        if line.startswith("**Draft / not yet submitted:**") and "cargo-binstall" in line:
            errors.append("draft/unsubmitted list still includes cargo-binstall")
    return (
        [_failure(relative, "; ".join(errors), kind="docs")]
        if errors
        else [
            _success(relative, "README cargo-binstall wording aligned", kind="docs"),
        ]
    )


def _release_header(evidence: dict[str, Any], version: str) -> dict[str, Any]:
    if type(evidence.get("schema_version")) is not int or evidence["schema_version"] != 1:
        reject_metadata("release-evidence schema mismatch")
    if evidence.get("component_version") != version:
        reject_metadata("release-evidence component version mismatch")
    if evidence.get("repository") != "https://github.com/edithatogo/fyi-cli":
        reject_metadata("release-evidence repository mismatch")
    if not is_review_date(evidence.get("checked_at")):
        reject_metadata("release-evidence checked_at must be an ISO date")
    commit = evidence.get("source_commit")
    if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
        reject_metadata("release-evidence requires a full source commit")
    release = _object(evidence.get("release"), "observed release")
    tag = release.get("tag")
    if (
        not isinstance(tag, str)
        or not tag.startswith("v")
        or STABLE_VERSION.fullmatch(tag[1:]) is None
    ):
        reject_metadata("release-evidence tag must be a concrete stable version")
    if release.get("draft") is not False or release.get("prerelease") is not False:
        reject_metadata("release-evidence must identify a public stable release")
    if release.get("url") != f"https://github.com/edithatogo/fyi-cli/releases/tag/{tag}":
        reject_metadata("release-evidence public URL does not match its tag")
    return release


def _asset_entry(value: object, tag: str) -> dict[str, Any]:
    asset = _object(value, "release-evidence asset")
    name = asset.get("name")
    if not isinstance(name, str) or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", name) is None:
        reject_metadata("release-evidence asset has an unsafe name")
    digest = asset.get("sha256")
    if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
        reject_metadata("release-evidence asset digest must be SHA-256")
    if len(set(digest)) == 1:
        reject_metadata("release-evidence asset digest must not be a placeholder")
    if type(asset.get("size")) is not int or asset["size"] <= 0:
        reject_metadata("release-evidence asset requires a positive size")
    if asset.get("url") != f"https://github.com/edithatogo/fyi-cli/releases/download/{tag}/{name}":
        reject_metadata("release-evidence asset URL mismatch")
    return asset


def _observed_assets(evidence: dict[str, Any], version: str) -> dict[str, dict[str, Any]]:
    """Validate an API observation without claiming that bytes were downloaded."""
    release = _release_header(evidence, version)
    files = evidence.get("assets")
    if not isinstance(files, list) or not files:
        reject_metadata("release-evidence assets must be non-empty")
    assets = {}
    for value in files:
        asset = _asset_entry(value, release["tag"])
        name = asset["name"]
        if name in assets:
            reject_metadata("release-evidence contains a duplicate asset")
        assets[name] = asset
    return assets


def _binding_errors(assets: dict[str, Any], name: str, url: object, digest: object) -> list[str]:
    expected = assets.get(name)
    if expected is None:
        return [f"missing release asset {name}"]
    errors = []
    if url != expected["url"]:
        errors.append("asset URL is not the observed CLI asset")
    if not isinstance(digest, str) or digest.lower() != expected["sha256"]:
        errors.append("digest is not bound to observed release asset")
    return errors


def _scoop_checks(text: str, assets: dict[str, Any]) -> list[str]:
    data = parse_json_object(text)
    architecture = _object(data.get("architecture"), "Scoop architecture")
    win = _object(architecture.get("64bit"), "Scoop Windows installer")
    errors = _binding_errors(
        assets,
        "fyi-cli-windows-amd64.exe.zip",
        win.get("url"),
        win.get("hash"),
    )
    if set(architecture) != {"64bit"} or data.get("bin") != "fyi-cli.exe":
        errors.append("unsupported architecture or non-CLI executable")
    if "autoupdate" in data or "checkver" in data:
        errors.append("unqualified tag/component mapping cannot auto-update")
    return errors


def _winget_checks(text: str, assets: dict[str, Any]) -> list[str]:
    data = parse_json_object(text)
    installers = data.get("Installers")
    if not isinstance(installers, list) or len(installers) != 1:
        reject_metadata("WinGet must declare one Windows x64 installer")
    win = _object(installers[0], "WinGet installer")
    errors = _binding_errors(
        assets,
        "fyi-cli-windows-amd64.exe.zip",
        win.get("InstallerUrl"),
        win.get("InstallerSha256"),
    )
    expected = {
        "Architecture": "x64",
        "InstallerType": "zip",
        "NestedInstallerType": "portable",
        "NestedInstallerFiles": [
            {"RelativeFilePath": "fyi-cli.exe", "PortableCommandAlias": "fyi-cli"},
        ],
    }
    if data.get("PackageIdentifier") != "edithatogo.fyi-cli" or any(
        win.get(key) != value for key, value in expected.items()
    ):
        errors.append("WinGet requires the CLI ZIP with nested portable fyi-cli.exe")
    return errors


def _single_match(pattern: str, text: str) -> str:
    values = re.findall(pattern, text)
    return values[0] if len(values) == 1 else ""


def _chocolatey_checks(text: str, assets: dict[str, Any]) -> list[str]:
    url = _single_match(r"(?m)^\$url64\s*=\s*'([^'\n]+)'$", text)
    digest = _single_match(r"(?m)^\$checksum64\s*=\s*'([^'\n]+)'$", text)
    errors = _binding_errors(assets, "fyi-cli-windows-amd64.exe.zip", url, digest)
    if "Install-ChocolateyPath" in text:
        errors.append("use Chocolatey shims instead of modifying machine PATH")
    return errors


def _brew_checks(text: str, assets: dict[str, Any]) -> list[str]:
    pairs = re.findall(r'url "([^"\n]+)"\s+sha256 "([^"\n]+)"', text)
    names = {
        "fyi-cli-linux-amd64.tar.gz",
        "fyi-cli-macos-amd64.tar.gz",
        "fyi-cli-macos-arm64.tar.gz",
    }
    errors = []
    if len(pairs) != 3 or {url.rsplit("/", 1)[-1] for url, _ in pairs} != names:
        errors.append("Homebrew must bind exactly the three supported archives")
    for url, digest in pairs:
        errors.extend(_binding_errors(assets, url.rsplit("/", 1)[-1], url, digest))
    if not re.search(r"on_linux do\s+depends_on arch: :x86_64", text):
        errors.append("Homebrew must restrict its Linux binary to x86_64")
    if 'bin.install "fyi-cli"' not in text:
        errors.append("Homebrew must install the CLI binary")
    return errors


def _binary_packet_checks(root: Path, assets: dict[str, Any]) -> list[CheckResult]:
    results = []
    validators = {
        "packaging/scoop/fyi-cli.json": _scoop_checks,
        "packaging/winget/edithatogo.fyi-cli.yaml": _winget_checks,
        "packaging/chocolatey/tools/chocolateyinstall.ps1": _chocolatey_checks,
        "packaging/homebrew/fyi-cli.rb": _brew_checks,
    }
    for relative, validator in validators.items():
        try:
            errors = validator(_read_text(root, relative), assets)
        except (OSError, ValueError, TypeError, KeyError) as error:
            errors = [f"malformed binary packet: {error}"]
        results.extend(_failure(relative, error) for error in errors)
    return results


def _checksum_field(value: object, token: str) -> bool:
    return value == token or (
        isinstance(value, str)
        and re.fullmatch(r"[0-9a-f]{64}", value) is not None
        and len(set(value)) > 1
    )


def _frozen_cargo_errors(commands: object) -> list[str]:
    if not isinstance(commands, str):
        return ["build commands must be text"]
    lines = [line.strip() for line in commands.splitlines() if not line.lstrip().startswith("#")]
    cargo = [line for line in lines if line.startswith("cargo ")]
    if not cargo or any("--frozen" not in line.split() or "||" in line for line in cargo):
        return ["Cargo build and test commands must be frozen without failure fallbacks"]
    if not all(any(action in line.split() for line in cargo) for action in ("build", "test")):
        return ["both frozen Cargo build and test commands are required"]
    return []


def _snap_checks(text: str, version: str, commit: str) -> list[str]:
    data = parse_json_object(text)
    parts = _object(data.get("parts"), "Snap parts")
    source = _object(parts.get("fyi-cli"), "Snap CLI source")
    vendor = _object(parts.get("vendor"), "Snap vendor source")
    toolchain = _object(parts.get("rust-toolchain"), "Snap qualified offline Rust toolchain")
    errors = []
    if data.get("name") != "fyi-cli" or data.get("version") != version:
        errors.append("Snap package name/version mismatch")
    if data.get("architectures") != [{"build-on": "amd64"}, {"build-on": "arm64"}]:
        errors.append("Snap must declare only Linux amd64/arm64")
    if source.get("source") != f"https://github.com/edithatogo/fyi-cli/archive/{commit}.tar.gz":
        errors.append("Snap source must pin the observed source commit")
    for part, token in (
        (source, "@SOURCE_SHA256@"),
        (vendor, "@CARGO_VENDOR_SHA256@"),
        (toolchain, "@RUST_TOOLCHAIN_SHA256@"),
    ):
        checksum = part.get("source-checksum")
        if (
            not isinstance(checksum, str)
            or not checksum.startswith("sha256/")
            or not _checksum_field(
                checksum.removeprefix("sha256/"),
                token,
            )
        ):
            errors.append("Snap source checksum must be SHA-256 or its documented blocked token")
    if vendor.get("source") != f"fyi-cli-{version}-vendor.tar.gz":
        errors.append("Snap vendor archive version mismatch")
    if source.get("after") != ["vendor", "rust-toolchain"] or any(
        toolchain.get(key) != value
        for key, value in {
            "source": "fyi-cli-rust-toolchain.tar.gz",
            "plugin": "dump",
            "source-type": "tar",
            "prime": ["-*"],
        }.items()
    ):
        errors.append("Snap requires the local qualified Rust toolchain before its offline build")
    errors.extend(_frozen_cargo_errors(source.get("override-build")))
    return errors


def _flatpak_checks(text: str, version: str, commit: str) -> list[str]:
    data = parse_json_object(text)
    if _flatpak_version(data) != version:
        reject_metadata("Flatpak vendor archive version mismatch")
    module = data["modules"][0]
    sources = module["sources"]
    if len(sources) != 3 or any(not isinstance(source, dict) for source in sources):
        reject_metadata("Flatpak requires source, vendor and AppStream inputs")
    errors = []
    if data.get("app-id") != "io.github.edithatogo.fyi-cli" or data.get("command") != "fyi-cli":
        errors.append("Flatpak identity or CLI command mismatch")
    if sources[0].get("url") != f"https://github.com/edithatogo/fyi-cli/archive/{commit}.tar.gz":
        errors.append("Flatpak source must pin the observed source commit")
    for source, token in zip(
        sources[:2],
        ("@SOURCE_SHA256@", "@CARGO_VENDOR_SHA256@"),
        strict=True,
    ):
        if source.get("type") != "archive" or not _checksum_field(source.get("sha256"), token):
            errors.append("Flatpak archive checksums must be SHA-256 or documented blocked tokens")
    commands = module.get("build-commands")
    if not isinstance(commands, list) or any(not isinstance(command, str) for command in commands):
        reject_metadata("Flatpak build commands must be a list of strings")
    errors.extend(_frozen_cargo_errors("\n".join(commands)))
    return errors


def _source_packet_checks(root: Path, version: str, commit: str) -> list[CheckResult]:
    results = []
    for relative, validator in {
        "packaging/snap/snapcraft.yaml": _snap_checks,
        "packaging/flatpak/io.github.edithatogo.fyi-cli.yml": _flatpak_checks,
    }.items():
        try:
            errors = validator(_read_text(root, relative), version, commit)
        except (OSError, ValueError, TypeError, KeyError) as error:
            errors = [f"malformed source packet: {error}"]
        results.extend(_failure(relative, error) for error in errors)
    return results


def _evidence_readiness_errors(evidence: dict[str, Any]) -> list[str]:
    errors = []
    ready = evidence.get("submission_ready")
    blockers = evidence.get("blockers")
    if type(ready) is not bool:
        errors.append("submission_ready must be boolean")
    if not isinstance(blockers, list) or any(
        not isinstance(item, str) or not item for item in blockers
    ):
        errors.append("release blockers must be a list of non-empty strings")
    elif (ready is True and blockers) or (ready is False and not blockers):
        errors.append("release readiness and blockers disagree")
    verification = evidence.get("verification")
    keys = ("downloaded_bytes", "clean_install", "signed_release", "sbom", "provenance")
    if (
        not isinstance(verification, dict)
        or set(verification) != set(keys)
        or any(type(verification.get(key)) is not bool for key in keys)
    ):
        errors.append("byte/install/signature/SBOM/provenance verification flags must be boolean")
    elif ready is True and not all(verification[key] for key in keys):
        errors.append("readiness requires byte/install/signature/SBOM/provenance evidence")
    if (
        ready is True
        and evidence.get("release", {}).get("tag") != f"v{evidence.get('component_version')}"
    ):
        errors.append("ready release tag must match the component version")
    if ready is True:
        errors.extend(_release_inventory_errors(evidence))
    return errors


def _sbom_names(subjects: dict[str, str]) -> set[str]:
    return {
        name for name in subjects if name.endswith((".spdx.json", ".cdx.json", ".cyclonedx.json"))
    }


def _release_inventory_errors(evidence: dict[str, Any]) -> list[str]:
    try:
        subjects = release_asset_subjects(evidence)
    except ValueError as error:
        return [f"release inventory is invalid: {error}"]
    errors = []
    missing = {"release-manifest.json", "SHA256SUMS"} - set(subjects)
    if missing:
        errors.append(f"ready release inventory is missing {', '.join(sorted(missing))}")
    if not _sbom_names(subjects):
        errors.append("ready release inventory requires an identified SPDX/CycloneDX JSON SBOM")
    return errors


def _receipt_subjects(
    evidence: dict[str, Any],
    native: dict[tuple[str, str], dict[str, Any]],
    kind: str,
) -> dict[str, str]:
    assets = release_asset_subjects(evidence)
    native_subjects = {value["name"]: value["sha256"] for value in native.values()}
    if kind == "downloaded_bytes":
        return {**assets, **native_subjects}
    if kind == "signed_release":
        names = {"release-manifest.json", "SHA256SUMS"}
    else:
        names = set(CLI_BINARY_ASSETS.values())
        if kind == "sbom":
            sboms = _sbom_names(assets)
            if not sboms:
                reject_metadata("SBOM receipt requires an identified SBOM in the release inventory")
            names |= sboms
    if missing := names - set(assets):
        reject_metadata(
            f"verification receipt is missing release subjects: {', '.join(sorted(missing))}",
        )
    subjects = {name: assets[name] for name in names}
    return subjects if kind == "signed_release" else {**subjects, **native_subjects}


def _single_receipt_errors(
    root: Path,
    value: object,
    evidence: dict[str, Any],
    *,
    kind: str,
    native: dict[tuple[str, str], dict[str, Any]],
) -> list[str]:
    try:
        subjects = _receipt_subjects(evidence, native, kind)
    except ValueError as error:
        return [str(error)]
    return verification_receipt_errors(
        root,
        value,
        context={
            "kind": kind,
            "version": evidence["component_version"],
            "source_commit": evidence["source_commit"],
        },
        subjects=subjects,
    )


def _verification_receipt_checks(root: Path, evidence: dict[str, Any]) -> list[CheckResult]:
    relative = "packaging/release-evidence.json"
    try:
        native = native_artifact_inventory(evidence)
    except ValueError as error:
        return [_failure(relative, str(error))]
    errors = [
        f"{artifact['name']}: {error}"
        for artifact in native.values()
        for error in native_artifact_receipt_errors(root, artifact)
    ]
    verification = evidence.get("verification")
    if not isinstance(verification, dict):
        return []  # The schema error is already reported by _evidence_readiness_errors.
    required = [key for key, value in verification.items() if value is True]
    if not required:
        return [_failure(relative, error) for error in errors]
    receipts = evidence.get("verification_receipts")
    if not isinstance(receipts, dict):
        return [
            _failure(
                "packaging/release-evidence.json",
                "verified claims require verification_receipts",
            ),
        ]
    for key in required:
        errors.extend(
            f"{key}: {error}"
            for error in _single_receipt_errors(
                root,
                receipts.get(key),
                evidence,
                kind=key,
                native=native,
            )
        )
    return [_failure(relative, error) for error in errors]


def _ready_smoke_checks(root: Path, targets: list[dict[str, Any]]) -> list[CheckResult]:
    ready = {target["id"] for target in targets if target.get("submission_ready") is True}
    if not ready:
        return []
    relative = "packaging/install-smoke-matrix.json"
    errors = validate_smokes(root)
    if errors:
        return [_failure(relative, error) for error in errors]
    matrix = parse_json_object(_read_text(root, relative))
    verified = {target["id"] for target in matrix["targets"] if target["status"] == "verified"}
    return [
        _failure(relative, f"{target_id}: readiness requires a verified clean-install smoke record")
        for target_id in sorted(ready - verified)
    ]


def _operative_text(relative: str, text: str) -> str:
    if Path(relative).suffix in {".md", ".txt"}:
        return ""  # Prose may explain unresolved examples without declaring package values.
    text = re.sub(r"<!--.*?-->", "", text, flags=re.DOTALL)
    return "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("#"))


def _packet_asset_checks(root: Path, target: dict[str, Any]) -> list[CheckResult]:
    results = []
    invalid = r"PLACEHOLDER_[A-Z0-9_]+|yourusername|lib\.fakeHash|sha256sums=\('SKIP'\)|sha256-AAAA"
    for relative in target["packet_assets"]:
        try:
            text = _operative_text(relative, _read_text(root, relative))
        except (OSError, ValueError) as error:
            results.append(_failure(relative, str(error)))
            continue
        if re.search(invalid, text, re.IGNORECASE):
            results.append(_failure(relative, "invalid placeholder metadata"))
        if target["submission_ready"] and re.search(r"@[A-Z][A-Z0-9_]*@", text):
            results.append(_failure(relative, "unresolved template cannot be submission-ready"))
    return results


def _asdf_approval_checks(root: Path, evidence: dict[str, Any]) -> list[CheckResult]:
    version = evidence["component_version"]
    relative = f"packaging/asdf/checksums/{version}.sha256"
    try:
        path = repository_file(root, relative)
        if path.is_symlink() or path.parent.is_symlink() or path.stat().st_size > 4096:
            reject_metadata("checksum approval must be a regular file of at most 4 KiB")
        lines = path.read_text(encoding="utf-8").splitlines()
        if lines[:2] != [f"# release-tag: v{version}", f"# binary-version: {version}"]:
            reject_metadata("checksum approval release tag and binary version must agree")
        approved: dict[str, str] = {}
        for line in lines[2:]:
            if not line:
                continue
            match = re.fullmatch(r"([0-9a-f]{64})  ([a-z0-9.-]+)", line)
            if match is None or not is_sha256(match[1]) or match[2] in approved:
                reject_metadata("checksum approval has an invalid, duplicate or placeholder entry")
            approved[match[2]] = match[1]
        observed = release_asset_subjects(evidence)
        required = {
            name: observed[name]
            for platform, name in CLI_BINARY_ASSETS.items()
            if platform != "windows-x86_64"
        }
        if approved != required:
            reject_metadata("checksum approval must bind all three observed Unix CLI assets")
    except (OSError, ValueError, KeyError) as error:
        return [_failure(relative, f"asdf checksum approval prevents readiness: {error}")]
    return []


def _installer_readiness_checks(
    root: Path,
    target: dict[str, Any],
    evidence: dict[str, Any],
) -> list[CheckResult]:
    if target["submission_ready"] is not True:
        return []
    if target["id"] == "cargo-binstall":
        return [
            _failure(
                "packaging/cargo-binstall/metadata.toml",
                "cargo-binstall is disabled; readiness requires a qualified policy update",
            ),
        ]
    if target["id"] == "asdf-mise":
        return _asdf_approval_checks(root, evidence)
    return []


def _ledger_contract_checks(
    root: Path,
    evidence: dict[str, Any],
    version: str,
) -> list[CheckResult]:
    relative = "packaging/registry-submissions.json"
    try:
        ledger = parse_json_object(_read_text(root, relative))
        errors = validate_ledger(ledger)
    except (OSError, ValueError) as error:
        return [_failure(relative, f"malformed submission ledger: {error}")]
    if errors:
        return [_failure(relative, error) for error in errors]
    results = []
    if ledger.get("source_release") != version:
        results.append(_failure(relative, "stale ledger source_release"))
    results.extend(
        _ready_smoke_checks(
            root,
            [target for target in ledger["targets"] if target["id"] in PACKAGE_TARGETS],
        ),
    )
    for target in ledger["targets"]:
        if target["id"] not in PACKAGE_TARGETS:
            continue
        if target["submission_ready"] and (
            evidence.get("submission_ready") is not True
            or evidence["release"]["tag"] != f"v{version}"
        ):
            results.append(
                _failure(relative, f"{target['id']}: release evidence prevents readiness"),
            )
        results.extend(_packet_asset_checks(root, target))
        results.extend(_installer_readiness_checks(root, target, evidence))
    return results


def _disabled_binstall_checks(root: Path) -> list[CheckResult]:
    results = []
    expected = {"disabled-strategies": ["crate-meta-data", "quick-install", "compile"]}
    for relative in (*BINSTALL_METADATA_ASSETS, "packaging/cargo-binstall/metadata.toml"):
        try:
            metadata = tomllib.loads(_read_text(root, relative))["package"]["metadata"]["binstall"]
            if metadata != expected:
                results.append(
                    _failure(relative, "unqualified binstall must disable all strategies"),
                )
        except (OSError, ValueError, KeyError, TypeError) as error:
            results.append(_failure(relative, f"invalid binstall metadata: {error}"))
    return results


def check_distribution_contracts(repo_root: Path, version: str) -> list[CheckResult]:
    """Bind local review packets to the public observation and preserve blockers."""
    relative = "packaging/release-evidence.json"
    try:
        evidence = parse_json_object(_read_text(repo_root, relative))
        assets = _observed_assets(evidence, version)
    except (OSError, ValueError, TypeError) as error:
        return [_failure(relative, str(error))]
    results = [_failure(relative, message) for message in _evidence_readiness_errors(evidence)]
    results.extend(_verification_receipt_checks(repo_root, evidence))
    results.extend(_binary_packet_checks(repo_root, assets))
    results.extend(_source_packet_checks(repo_root, version, evidence["source_commit"]))
    results.extend(_ledger_contract_checks(repo_root, evidence, version))
    results.extend(_disabled_binstall_checks(repo_root))
    return results or [
        _success("packaging/", "review packets consistent; publication remains gated"),
    ]


def submission_blockers(repo_root: Path, target_id: str | None = None) -> list[str]:
    """Return explicit blockers without equating repository validity with readiness."""
    ledger = parse_json_object(_read_text(repo_root, "packaging/registry-submissions.json"))
    errors = validate_ledger(ledger)
    if errors:
        return errors
    evidence = parse_json_object(_read_text(repo_root, "packaging/release-evidence.json"))
    targets = [
        target
        for target in ledger["targets"]
        if target["id"] in PACKAGE_TARGETS and (target_id is None or target["id"] == target_id)
    ]
    if not targets:
        return [f"unknown distribution target: {target_id}"]
    blockers = [
        f"{check.path}: {check.message}"
        for check in check_distribution_contracts(repo_root, discover_expected_version(repo_root))
        if not check.ok
    ]
    if evidence.get("submission_ready") is not True:
        reasons = evidence.get("blockers")
        if isinstance(reasons, list) and all(isinstance(reason, str) for reason in reasons):
            blockers.extend(reasons or ["release is not submission-ready"])
    for target in targets:
        if target["submission_ready"] is not True:
            blockers.extend(
                f"{target['id']}: {reason}" for reason in target["blockers"] or [target["status"]]
            )
    return blockers


def verify_packaging_assets(
    repo_root: Path,
    *,
    expected_version: str | None = None,
    critical: Iterable[str] = CRITICAL_ASSETS,
    optional: Iterable[str] = OPTIONAL_ASSETS,
    versioned: Iterable[str] = VERSIONED_ASSETS,
    binstall_assets: Iterable[str] = BINSTALL_METADATA_ASSETS,
    readme_path: str | None = README_PATH,
) -> VerificationReport:
    """Run repository checks without network access or registry side effects."""
    repo_root = repo_root.resolve()
    try:
        version = expected_version or discover_expected_version(repo_root)
    except (OSError, ValueError) as error:
        return VerificationReport(
            expected_version="unknown",
            results=[
                _failure("crates/fyi-cli/Cargo.toml", str(error), kind="version"),
            ],
        )
    report = VerificationReport(expected_version=version)
    report.results.extend(
        check_presence(repo_root, critical, kind="critical", severity_if_missing="error"),
    )
    report.results.extend(
        check_presence(repo_root, optional, kind="optional", severity_if_missing="warning"),
    )
    report.results.extend(check_versions(repo_root, version, versioned))
    report.results.extend(check_binstall_metadata(repo_root, binstall_assets))
    report.results.extend(check_packaging_readme(repo_root, readme_path))
    if (repo_root / "packaging/registry-submissions.json").is_file():
        report.results.extend(check_distribution_contracts(repo_root, version))
    return report


def format_human_report(report: VerificationReport) -> str:
    """Show the checks that need attention, including retained optional warnings."""
    lines = [
        "Packaging assets verification",
        f"  expected version: {report.expected_version}",
        f"  status: {'PASS' if report.ok else 'FAIL'}",
        f"  errors: {len(report.errors)}  warnings: {len(report.warnings)}",
        "",
    ]
    labels = {
        "critical": "Critical missing",
        "version": "Version mismatches",
        "binstall": "cargo-binstall wiring issues",
        "docs": "Packaging README issues",
        "distribution": "Distribution contract errors",
        "optional": "Optional missing (warnings)",
    }
    for kind, title in labels.items():
        failures = [result for result in report.results if result.kind == kind and not result.ok]
        if failures:
            lines.append(title + ":")
            lines.extend(f"  - {result.path}: {result.message}" for result in failures)
            lines.append("")
    if report.ok:
        lines.append(
            "Repository packaging contracts pass; publication requires qualified evidence.",
        )
    else:
        lines.extend(
            ["Fix errors above, then re-run:", "  python scripts/verify_packaging_assets.py"],
        )
    return "\n".join(lines).rstrip() + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--expected-version", help="Expected CLI package version")
    parser.add_argument("--json", action="store_true", help="Emit a JSON report")
    parser.add_argument(
        "--strict-optional",
        action="store_true",
        help="Fail on missing optional assets",
    )
    parser.add_argument(
        "--require-submission-ready",
        action="store_true",
        help="Fail on publication blockers",
    )
    parser.add_argument("--target", help="Distribution channel for --require-submission-ready")
    args = parser.parse_args(argv)
    if args.target and not args.require_submission_ready:
        parser.error("--target requires --require-submission-ready")
    report = verify_packaging_assets(args.repo_root, expected_version=args.expected_version)
    if args.require_submission_ready:
        try:
            blockers = submission_blockers(args.repo_root, args.target)
        except (OSError, ValueError, KeyError, TypeError) as error:
            blockers = [f"submission evidence unreadable: {error}"]
        report.results.extend(
            _failure("packaging/registry-submissions.json", item) for item in blockers
        )
    if args.strict_optional:
        report.results = [
            replace(result, severity="error")
            if result.kind == "optional" and not result.ok
            else result
            for result in report.results
        ]
    output = (
        json.dumps(report.as_dict(), indent=2) + "\n" if args.json else format_human_report(report)
    )
    sys.stdout.write(output)
    return 0 if report.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
