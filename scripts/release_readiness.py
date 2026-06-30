"""Release readiness inventory for docs, metadata, and CI surfaces."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


RELEASE_SURFACES = [
    "README.md",
    "INSTALL.md",
    "QUICKSTART.md",
    "RELEASE_PLAN.md",
    "GITHUB_SETUP.md",
    "CHANGELOG.md",
    "Cargo.toml",
    "pyproject.toml",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    ".github/workflows/release-please.yml",
]

PLACEHOLDER_PATTERNS = [
    "github.com/yourusername/fyi-cli",
    "codecov.io/gh/yourusername/fyi-cli",
]

LEGACY_COMMAND_PATTERNS = [
    "fyi-system ",
    "fyi-system\n",
    "fyi-system.",
    "build/fyi-system",
]

RUST_CHECK_COMMANDS = [
    "cargo +stable-x86_64-pc-windows-gnu fmt --all -- --check",
    "cargo +stable-x86_64-pc-windows-gnu clippy --workspace --all-targets --all-features -- -D warnings",
    "cargo +stable-x86_64-pc-windows-gnu test --workspace --all-features",
]


@dataclass(frozen=True)
class Issue:
    code: str
    severity: str
    path: str
    line: int | None
    message: str

    def as_dict(self) -> dict[str, Any]:
        return {
            "code": self.code,
            "severity": self.severity,
            "path": self.path,
            "line": self.line,
            "message": self.message,
        }


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def _line_number(text: str, needle: str) -> int | None:
    index = text.find(needle)
    if index < 0:
        return None
    return text.count("\n", 0, index) + 1


def _surface(repo_root: Path, relative: str) -> dict[str, Any]:
    path = repo_root / relative
    return {
        "path": relative,
        "exists": path.exists(),
        "bytes": path.stat().st_size if path.exists() else 0,
    }


def _scan_placeholder_urls(relative: str, text: str) -> list[Issue]:
    issues = []
    for pattern in PLACEHOLDER_PATTERNS:
        if pattern in text:
            issues.append(
                Issue(
                    code="placeholder_repository_url",
                    severity="high",
                    path=relative,
                    line=_line_number(text, pattern),
                    message=f"Release-facing file still references placeholder URL '{pattern}'.",
                )
            )
    return issues


def _scan_legacy_commands(relative: str, text: str) -> list[Issue]:
    issues = []
    for pattern in LEGACY_COMMAND_PATTERNS:
        if pattern in text:
            issues.append(
                Issue(
                    code="legacy_python_command",
                    severity="medium",
                    path=relative,
                    line=_line_number(text, pattern),
                    message=(
                        "Release-facing file still presents legacy Python-era "
                        f"command text matching '{pattern.strip()}'."
                    ),
                )
            )
    return issues


def _scan_rust_release_commands(repo_root: Path) -> list[Issue]:
    combined = "\n".join(
        _read_text(repo_root / relative)
        for relative in RELEASE_SURFACES
        if (repo_root / relative).exists()
    )
    issues = []
    for command in RUST_CHECK_COMMANDS:
        if command not in combined:
            issues.append(
                Issue(
                    code="rust_release_command_missing",
                    severity="medium",
                    path="release surfaces",
                    line=None,
                    message=f"Release docs do not mention required check: `{command}`.",
                )
            )
    return issues


def build_report(repo_root: Path) -> dict[str, Any]:
    """Build a release-readiness inventory without modifying the repository."""
    repo_root = repo_root.resolve()
    surfaces = [_surface(repo_root, relative) for relative in RELEASE_SURFACES]
    issues: list[Issue] = []

    for surface in surfaces:
        if not surface["exists"]:
            issues.append(
                Issue(
                    code="missing_release_surface",
                    severity="medium",
                    path=surface["path"],
                    line=None,
                    message="Expected release-facing file is missing.",
                )
            )
            continue

        text = _read_text(repo_root / surface["path"])
        issues.extend(_scan_placeholder_urls(surface["path"], text))
        issues.extend(_scan_legacy_commands(surface["path"], text))

    issues.extend(_scan_rust_release_commands(repo_root))

    return {
        "surfaces": surfaces,
        "issues": [issue.as_dict() for issue in issues],
        "summary": {
            "surface_count": len(surfaces),
            "missing_surface_count": sum(1 for surface in surfaces if not surface["exists"]),
            "issue_count": len(issues),
            "high_count": sum(1 for issue in issues if issue.severity == "high"),
            "medium_count": sum(1 for issue in issues if issue.severity == "medium"),
        },
    }


def write_markdown_report(report: dict[str, Any], output_path: Path) -> None:
    """Write a markdown release-readiness inventory report."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# Release Readiness Inventory",
        "",
        "## Summary",
        "",
        f"- Surfaces scanned: {report['summary']['surface_count']}",
        f"- Missing surfaces: {report['summary']['missing_surface_count']}",
        f"- Issues found: {report['summary']['issue_count']}",
        f"- High severity: {report['summary']['high_count']}",
        f"- Medium severity: {report['summary']['medium_count']}",
        "",
        "## Surfaces",
        "",
    ]

    for surface in report["surfaces"]:
        status = "present" if surface["exists"] else "missing"
        lines.append(f"- `{surface['path']}`: {status}")

    lines.extend(["", "## Issues", ""])
    if report["issues"]:
        for issue in report["issues"]:
            location = issue["path"]
            if issue["line"]:
                location = f"{location}:{issue['line']}"
            lines.append(
                f"- `{issue['code']}` ({issue['severity']}) at `{location}`: {issue['message']}"
            )
    else:
        lines.append("- No issues found.")

    lines.extend(
        [
            "",
            "## Required Rust Release Checks",
            "",
            *[f"- `{command}`" for command in RUST_CHECK_COMMANDS],
            "",
        ]
    )
    output_path.write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--markdown",
        type=Path,
        default=Path("docs/release-readiness-inventory.md"),
        help="Path for the markdown checklist report.",
    )
    parser.add_argument("--json", action="store_true", help="Print JSON report to stdout.")
    args = parser.parse_args()

    report = build_report(args.repo_root)
    write_markdown_report(report, args.repo_root / args.markdown)
    if args.json:
        print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
