#!/usr/bin/env python3
"""Reject partial Rust releases that cannot satisfy the packaging contract."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from typing import Iterable


RUST_VERSION_FILES: tuple[str, ...] = (
    "crates/fyi-cli/Cargo.toml",
    "crates/fyi-core/Cargo.toml",
    "crates/fyi-mcp/Cargo.toml",
)

VERSIONED_RELEASE_ASSETS: tuple[str, ...] = (
    *RUST_VERSION_FILES,
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
)


def missing_release_assets(changed_paths: Iterable[str]) -> list[str]:
    """Return assets missing from a Rust version-changing pull request."""
    changed = {path.replace("\\", "/") for path in changed_paths}
    if not changed.intersection(RUST_VERSION_FILES):
        return []
    return [asset for asset in VERSIONED_RELEASE_ASSETS if asset not in changed]


def changed_paths(repo_root: Path, base: str, head: str) -> list[str]:
    """Read changed paths from git without shell interpolation."""
    result = subprocess.run(
        ["git", "diff", "--name-only", f"{base}...{head}"],
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="Base commit or ref for the diff")
    parser.add_argument("--head", default="HEAD", help="Head commit or ref for the diff")
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
    )
    args = parser.parse_args(argv)

    missing = missing_release_assets(changed_paths(args.repo_root, args.base, args.head))
    if not missing:
        print("Release scope verification: PASS")
        return 0

    print("Release scope verification: FAIL", file=sys.stderr)
    print(
        "A Rust package version changed without a synchronized distribution update.",
        file=sys.stderr,
    )
    print("Required changed assets:", file=sys.stderr)
    for asset in missing:
        print(f"  - {asset}", file=sys.stderr)
    print(
        "Use one coordinated release PR and update all listed assets before merging.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
