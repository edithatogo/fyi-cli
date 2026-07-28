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
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Iterable

# ---------------------------------------------------------------------------
# Asset inventory (paths relative to repo root)
# ---------------------------------------------------------------------------

# Critical packaging files: absence fails the check.
CRITICAL_ASSETS: tuple[str, ...] = (
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
    r'(?m)^version\s*=\s*"(?P<version>\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.]+)?)"\s*$'
)
_SEMVER_RE = re.compile(r"\b(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.]+)?)\b")


@dataclass(frozen=True)
class CheckResult:
    """Single packaging asset check outcome."""

    path: str
    kind: str  # critical | optional | version
    ok: bool
    message: str
    severity: str = "error"  # error | warning | info


@dataclass
class VerificationReport:
    """Aggregate result of packaging asset verification."""

    expected_version: str
    results: list[CheckResult] = field(default_factory=list)

    @property
    def errors(self) -> list[CheckResult]:
        return [r for r in self.results if not r.ok and r.severity == "error"]

    @property
    def warnings(self) -> list[CheckResult]:
        return [r for r in self.results if not r.ok and r.severity == "warning"]

    @property
    def ok(self) -> bool:
        return not self.errors

    def as_dict(self) -> dict[str, Any]:
        return {
            "expected_version": self.expected_version,
            "ok": self.ok,
            "error_count": len(self.errors),
            "warning_count": len(self.warnings),
            "results": [asdict(r) for r in self.results],
        }


def discover_expected_version(repo_root: Path) -> str:
    """Prefer crates/fyi-cli version; fall back to DEFAULT_EXPECTED_VERSION."""
    crate = repo_root / "crates" / "fyi-cli" / "Cargo.toml"
    if crate.is_file():
        text = crate.read_text(encoding="utf-8", errors="replace")
        match = _CRATE_VERSION_RE.search(text)
        if match:
            return match.group("version")
    return DEFAULT_EXPECTED_VERSION


def _path_exists(repo_root: Path, relative: str) -> bool:
    return (repo_root / relative).is_file()


def check_presence(
    repo_root: Path,
    assets: Iterable[str],
    *,
    kind: str,
    severity_if_missing: str,
) -> list[CheckResult]:
    """Return presence checks for each relative path."""
    results: list[CheckResult] = []
    for relative in assets:
        exists = _path_exists(repo_root, relative)
        if exists:
            results.append(
                CheckResult(
                    path=relative,
                    kind=kind,
                    ok=True,
                    message="present",
                    severity="info",
                )
            )
        else:
            results.append(
                CheckResult(
                    path=relative,
                    kind=kind,
                    ok=False,
                    message="missing",
                    severity=severity_if_missing,
                )
            )
    return results


def file_mentions_version(text: str, version: str) -> bool:
    """True if the file text contains the exact version token."""
    return version in text


def check_binstall_metadata(
    repo_root: Path, assets: Iterable[str] = BINSTALL_METADATA_ASSETS
) -> list[CheckResult]:
    """Ensure the expected crates wire cargo-binstall metadata in Cargo.toml."""
    results: list[CheckResult] = []
    for relative in assets:
        path = repo_root / relative
        if not path.is_file():
            results.append(
                CheckResult(
                    path=relative,
                    kind="binstall",
                    ok=False,
                    message="missing",
                    severity="error",
                )
            )
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if "[package.metadata.binstall]" in text:
            results.append(
                CheckResult(
                    path=relative,
                    kind="binstall",
                    ok=True,
                    message="cargo-binstall metadata wired",
                    severity="info",
                )
            )
            continue
        results.append(
            CheckResult(
                path=relative,
                kind="binstall",
                ok=False,
                message="missing [package.metadata.binstall]",
                severity="error",
            )
        )
    return results


def check_versions(
    repo_root: Path,
    expected_version: str,
    assets: Iterable[str] = VERSIONED_ASSETS,
) -> list[CheckResult]:
    """Ensure versioned packaging files mention expected_version when present."""
    results: list[CheckResult] = []
    for relative in assets:
        path = repo_root / relative
        if not path.is_file():
            # Presence is handled separately; skip version scan for missing files.
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if file_mentions_version(text, expected_version):
            results.append(
                CheckResult(
                    path=relative,
                    kind="version",
                    ok=True,
                    message=f"mentions {expected_version}",
                    severity="info",
                )
            )
        else:
            # cargo-binstall metadata uses templates without a pinned version — warn only
            # if the path is in VERSIONED_ASSETS and has other semver-like tokens.
            other = sorted({m.group(1) for m in _SEMVER_RE.finditer(text)})
            detail = (
                f"does not mention expected version {expected_version}"
                + (f" (found: {', '.join(other)})" if other else "")
            )
            results.append(
                CheckResult(
                    path=relative,
                    kind="version",
                    ok=False,
                    message=detail,
                    severity="error",
                )
            )
    return results


def verify_packaging_assets(
    repo_root: Path,
    *,
    expected_version: str | None = None,
    critical: Iterable[str] = CRITICAL_ASSETS,
    optional: Iterable[str] = OPTIONAL_ASSETS,
    versioned: Iterable[str] = VERSIONED_ASSETS,
    binstall_assets: Iterable[str] = BINSTALL_METADATA_ASSETS,
) -> VerificationReport:
    """Run all packaging asset checks and return a structured report."""
    repo_root = repo_root.resolve()
    version = expected_version or discover_expected_version(repo_root)
    report = VerificationReport(expected_version=version)
    report.results.extend(
        check_presence(repo_root, critical, kind="critical", severity_if_missing="error")
    )
    report.results.extend(
        check_presence(
            repo_root, optional, kind="optional", severity_if_missing="warning"
        )
    )
    report.results.extend(check_versions(repo_root, version, versioned))
    report.results.extend(check_binstall_metadata(repo_root, binstall_assets))
    return report


def format_human_report(report: VerificationReport) -> str:
    """Render a concise human-readable summary."""
    lines = [
        "Packaging assets verification",
        f"  expected version: {report.expected_version}",
        f"  status: {'PASS' if report.ok else 'FAIL'}",
        f"  errors: {len(report.errors)}  warnings: {len(report.warnings)}",
        "",
    ]

    critical_missing = [
        r for r in report.results if r.kind == "critical" and not r.ok
    ]
    version_fail = [r for r in report.results if r.kind == "version" and not r.ok]
    binstall_fail = [r for r in report.results if r.kind == "binstall" and not r.ok]
    optional_missing = [
        r for r in report.results if r.kind == "optional" and not r.ok
    ]

    if critical_missing:
        lines.append("Critical missing:")
        for r in critical_missing:
            lines.append(f"  - {r.path}")
        lines.append("")

    if version_fail:
        lines.append("Version mismatches:")
        for r in version_fail:
            lines.append(f"  - {r.path}: {r.message}")
        lines.append("")

    if binstall_fail:
        lines.append("cargo-binstall wiring issues:")
        for r in binstall_fail:
            lines.append(f"  - {r.path}: {r.message}")
        lines.append("")

    if optional_missing:
        lines.append("Optional missing (warnings):")
        for r in optional_missing:
            lines.append(f"  - {r.path}")
        lines.append("")

    if report.ok and not report.warnings:
        lines.append("All critical packaging assets present; versions consistent.")
    elif report.ok:
        lines.append("Critical checks passed (see warnings above).")
    else:
        lines.append("Fix errors above, then re-run:")
        lines.append("  python scripts/verify_packaging_assets.py")

    return "\n".join(lines).rstrip() + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Verify packaging / multi-registry distribution assets."
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=None,
        help="Repository root (default: parent of scripts/).",
    )
    parser.add_argument(
        "--expected-version",
        type=str,
        default=None,
        help=f"Version string that packaging files should mention "
        f"(default: crates/fyi-cli version or {DEFAULT_EXPECTED_VERSION}).",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit machine-readable JSON report to stdout.",
    )
    parser.add_argument(
        "--strict-optional",
        action="store_true",
        help="Treat missing optional assets as errors.",
    )
    args = parser.parse_args(argv)

    if args.repo_root is not None:
        repo_root = args.repo_root
    else:
        # scripts/ → repo root
        repo_root = Path(__file__).resolve().parent.parent

    report = verify_packaging_assets(
        repo_root, expected_version=args.expected_version
    )

    if args.strict_optional:
        # Promote optional misses to errors for stricter CI if desired.
        promoted: list[CheckResult] = []
        for r in report.results:
            if r.kind == "optional" and not r.ok:
                promoted.append(
                    CheckResult(
                        path=r.path,
                        kind=r.kind,
                        ok=False,
                        message=r.message,
                        severity="error",
                    )
                )
            else:
                promoted.append(r)
        report.results = promoted

    if args.json:
        print(json.dumps(report.as_dict(), indent=2))
    else:
        sys.stdout.write(format_human_report(report))

    return 0 if report.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
