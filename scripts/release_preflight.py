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
        raise RuntimeError(f"cannot load {path}")
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
            raise ValueError(f"missing package version in {relative}")
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
        for snippet in snippets:
            if snippet not in text:
                missing.append(f"{relative} missing {snippet!r}")
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
        }
    )
    checks.append(
        {
            "id": "requested-version",
            "ok": requested_version is None or requested_version == unique_versions[0],
            "message": f"requested version: {requested_version or version}",
        }
    )
    packaging = load_packaging_module().verify_packaging_assets(
        repo_root, expected_version=version
    )
    checks.append(
        {
            "id": "packaging-assets",
            "ok": packaging.ok,
            "message": f"{len(packaging.errors)} errors, {len(packaging.warnings)} warnings",
        }
    )
    checks.append(hosted_connector_docs_check(repo_root))
    return {
        "schema_version": 1,
        "ok": all(check["ok"] for check in checks),
        "version": version,
        "checks": checks,
        "operator_actions": [
            f"Create and push the annotated tag v{version} after this report passes.",
            "Confirm required package-registry credentials are available to GitHub Actions.",
            "If targeting hosted Connector directories, verify an HTTPS deployment with FYI_MCP_TRANSPORT=http, bearer-token auth, and /healthz against deploy/remote-mcp/README.md before claiming compatibility.",
            "Record public release URLs, digests, and registry outcomes after publication.",
        ],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", help="Expected synchronized release version")
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    report = build_report(args.repo_root, args.version)
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"Release preflight: {'PASS' if report['ok'] else 'FAIL'}")
        print(f"Version: {report['version']}")
        for check in report["checks"]:
            print(f"- {'PASS' if check['ok'] else 'FAIL'} {check['id']}: {check['message']}")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
