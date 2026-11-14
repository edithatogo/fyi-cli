#!/usr/bin/env python3
"""Run the credential-free, cross-platform release preflight."""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CRATE_FILES = (
    "crates/fyi-core/Cargo.toml",
    "crates/fyi-cli/Cargo.toml",
    "crates/fyi-mcp/Cargo.toml",
)
HOSTED_CONNECTOR_DOCS = {
    "deploy/remote-mcp/README.md": (
        "FYI_MCP_TRANSPORT=http",
        "FYI_MCP_HTTP_BEARER_TOKEN",
        "/healthz",
    ),
    "docs/release-multi-registry.md": (
        "FYI_MCP_TRANSPORT=http",
        "hosted MCP deployment contract",
        "/healthz",
    ),
    "docs/containers.md": (
        "FYI_MCP_TRANSPORT=http",
        "/healthz",
        "deploy/remote-mcp/README.md",
    ),
}
VERSION_RE = re.compile(r'^version\s*=\s*"(?P<version>[^"\n]+)"$', re.MULTILINE)


def load_packaging_module():
    path = ROOT / "scripts" / "verify_packaging_assets.py"
    spec = importlib.util.spec_from_file_location("verify_packaging_assets", path)
    if spec is None or spec.loader is None:
        message = f"cannot load {path}"
        raise RuntimeError(message)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def crate_versions(repo_root: Path) -> dict[str, str]:
    versions: dict[str, str] = {}
    for relative in CRATE_FILES:
        text = (repo_root / relative).read_text(encoding="utf-8")
        match = VERSION_RE.search(text)
        if match is None:
            message = f"missing package version in {relative}"
            raise ValueError(message)
        versions[relative] = match.group("version")
    return versions


def hosted_connector_docs_check(repo_root: Path) -> dict[str, Any]:
    missing: list[str] = []
    for relative, snippets in HOSTED_CONNECTOR_DOCS.items():
        path = repo_root / relative
        if not path.is_file():
            missing.append(f"{relative} missing")
            continue
        text = path.read_text(encoding="utf-8")
        missing.extend(
            f"{relative} missing {snippet!r}" for snippet in snippets if snippet not in text
        )
    return {
        "id": "hosted-connector-docs",
        "ok": not missing,
        "message": (
            "hosted connector docs cover opt-in HTTP transport and deployment prerequisites"
            if not missing
            else "; ".join(missing)
        ),
    }


def build_report(repo_root: Path = ROOT, requested_version: str | None = None) -> dict[str, Any]:
    """Return a deterministic release report suitable for CI or operators."""
    repo_root = repo_root.resolve()
    versions = crate_versions(repo_root)
    unique_versions = sorted(set(versions.values()))
    version = requested_version or unique_versions[0]
    checks: list[dict[str, Any]] = []
    checks.append(
        {
            "id": "crate-version-sync",
            "ok": len(unique_versions) == 1,
            "message": (
                f"all workspace crates use {version}"
                if len(unique_versions) == 1
                else f"workspace crates disagree: {versions}"
            ),
        },
    )
    checks.append(
        {
            "id": "requested-version",
            "ok": requested_version is None or requested_version == unique_versions[0],
            "message": f"requested version: {requested_version or version}",
        },
    )
    packaging_module = load_packaging_module()
    packaging = packaging_module.verify_packaging_assets(
        repo_root,
        expected_version=version,
    )
    blockers = packaging_module.submission_blockers(repo_root)
    checks.append(
        {
            "id": "packaging-assets",
            "ok": packaging.ok,
            "message": f"{len(packaging.errors)} errors, {len(packaging.warnings)} warnings",
        },
    )
    checks.append(hosted_connector_docs_check(repo_root))
    local_checks_pass = all(check["ok"] for check in checks)
    return {
        "schema_version": 1,
        "ok": local_checks_pass,
        "version": version,
        "checks": checks,
        "submission_ready": local_checks_pass and not blockers,
        "external_blockers": blockers,
        "scope": (
            "Repository contract preflight; not a publication authorization "
            "or native install result."
        ),
        "operator_actions": [
            (
                "Reconcile the observed v1.2.0 tag / Rust 0.1.2 component mismatch "
                "with the existing release authority; do not retag historical assets."
            ),
            (
                "Resolve the occupied fyi-cli and fyi-core crate names before enabling "
                "any registry publication or binstall strategies."
            ),
            (
                "Prepare a coordinated signed release with independently verified payload "
                "hashes, SHA256SUMS, release-manifest.json, SBOM and authenticated provenance; "
                "run clean native install/help/uninstall checks for each target."
            ),
            (
                "If targeting hosted Connector directories, verify an HTTPS deployment "
                "with FYI_MCP_TRANSPORT=http, bearer-token auth, and /healthz against "
                "deploy/remote-mcp/README.md before claiming compatibility."
            ),
            "Record public release URLs, digests, and registry outcomes after publication.",
        ],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", help="Expected synchronized release version")
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    parser.add_argument("--require-submission-ready", action="store_true")
    args = parser.parse_args(argv)
    report = build_report(args.repo_root, args.version)
    if args.json:
        sys.stdout.write(json.dumps(report, indent=2, sort_keys=True) + "\n")
    else:
        lines = [
            f"Release preflight: {'PASS' if report['ok'] else 'FAIL'}",
            f"Version: {report['version']}",
            f"Submission ready: {report['submission_ready']}",
        ]
        lines.extend(
            f"- {'PASS' if check['ok'] else 'FAIL'} {check['id']}: {check['message']}"
            for check in report["checks"]
        )
        sys.stdout.write("\n".join(lines) + "\n")
    ready = not args.require_submission_ready or report["submission_ready"]
    return 0 if report["ok"] and ready else 1


if __name__ == "__main__":
    raise SystemExit(main())
