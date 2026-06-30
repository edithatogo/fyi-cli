"""Generate the FYI/Alaveteli API contract inventory report."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

REQUIRED_SURFACES = {
    "rust_api_payloads",
    "rust_sync_client",
    "rust_cli_sync_surface",
    "rust_mcp_sync_surface",
    "archive_public_web",
}


def _exists(repo_root: Path, path: str) -> bool:
    return (repo_root / path).exists()


def _read(repo_root: Path, path: str) -> str:
    target = repo_root / path
    if not target.exists():
        return ""
    return target.read_text(encoding="utf-8")


def _status(repo_root: Path, paths: list[str]) -> str:
    missing = [path for path in paths if not _exists(repo_root, path)]
    if missing:
        return f"missing evidence: {', '.join(missing)}"
    return "documented"


def _risk(repo_root: Path, search_paths: list[str], terms: list[str]) -> str:
    haystack = "\n".join(_read(repo_root, path).lower() for path in search_paths)
    return "medium" if all(term.lower() in haystack for term in terms) else "high"


def build_contract_inventory(repo_root: Path) -> dict[str, Any]:
    """Build a matrix of API-adjacent contracts and their test coverage."""
    surfaces = [
        {
            "id": "rust_api_payloads",
            "surface": "Rust API payload structs",
            "contract": (
                "Serde JSON shapes for Alaveteli request, correspondence, "
                "create, update, and action responses."
            ),
            "files": ["crates/fyi-core/src/api.rs"],
            "tests": ["crates/fyi-core/src/api.rs::tests"],
            "coverage": (
                "Round-trip payload tests, optional field compatibility, "
                "and wiremock success responses."
            ),
            "risk": _risk(
                repo_root,
                ["crates/fyi-core/src/api.rs"],
                ["missing_fields", "wiremock_create_request"],
            ),
        },
        {
            "id": "rust_sync_client",
            "surface": "Rust sync API client",
            "contract": (
                "GET /api/v2/request.json, GET /api/v2/request/{id}.json, "
                "POST /api/v2/request, watched feed pulls, retry queues, "
                "and merge/conflict preservation."
            ),
            "files": ["crates/fyi-core/src/sync.rs"],
            "tests": ["crates/fyi-core/src/sync.rs::tests"],
            "coverage": (
                "Mocked successful pull, feed pull, malformed JSON, missing "
                "required fields, HTTP 401/403/404/429/5xx failures, health, "
                "push retry, scheduler, and conflict merge behavior."
            ),
            "risk": _risk(
                repo_root,
                ["crates/fyi-core/src/sync.rs"],
                ["429", "unauthorized", "malformed"],
            ),
        },
        {
            "id": "rust_cli_sync_surface",
            "surface": "Rust CLI sync commands",
            "contract": (
                "CLI presentation for sync status, pull, push, conflicts, "
                "and conflict resolution."
            ),
            "files": [
                "crates/fyi-cli/src/main.rs",
                "crates/fyi-cli/tests/cli_tests.rs",
                "crates/fyi-cli/tests/e2e_tests.rs",
            ],
            "tests": [
                "crates/fyi-cli/src/main.rs::tests",
                "crates/fyi-cli/tests/cli_tests.rs",
                "crates/fyi-cli/tests/e2e_tests.rs",
            ],
            "coverage": (
                "Parser and E2E coverage for sync command shapes and "
                "database-backed output."
            ),
            "risk": _risk(
                repo_root,
                [
                    "crates/fyi-cli/src/main.rs",
                    "crates/fyi-cli/tests/cli_tests.rs",
                    "crates/fyi-cli/tests/e2e_tests.rs",
                ],
                ["sync", "error", "json"],
            ),
        },
        {
            "id": "rust_mcp_sync_surface",
            "surface": "Rust MCP API-adjacent tools",
            "contract": (
                "JSON-RPC tools exposing request CRUD, authority import, "
                "sync status, sync monitor, conflicts, and resolution."
            ),
            "files": ["crates/fyi-mcp/src/main.rs"],
            "tests": ["crates/fyi-mcp/src/main.rs::tests"],
            "coverage": (
                "In-process JSON-RPC tests for tool listing, request flows, "
                "sync status, conflict, and monitor tools."
            ),
            "risk": _risk(
                repo_root,
                ["crates/fyi-mcp/src/main.rs"],
                ["test_sync_status_tool", "failed to fetch sync", "invalid"],
            ),
        },
        {
            "id": "archive_public_web",
            "surface": "Archive public-web endpoints",
            "contract": (
                "FYI public request pages, attachments, discovery feeds, "
                "diff manifests, and archive health JSON."
            ),
            "files": [
                "src/fyi_system/discovery.py",
                "src/fyi_system/archive_capture.py",
                "src/fyi_system/archive_diff.py",
                "src/fyi_system/archive_health.py",
            ],
            "tests": [
                "tests/test_discovery.py",
                "tests/test_discovery_smoke.py",
                "tests/test_archive_capture.py",
                "tests/test_archive_diff.py",
                "tests/test_archive_health.py",
            ],
            "coverage": (
                "Mocked discovery, capture, diff, and health tests; live "
                "discovery smoke test remains environment-gated."
            ),
            "risk": _risk(
                repo_root,
                [
                    "src/fyi_system/discovery.py",
                    "tests/test_discovery_smoke.py",
                    "tests/test_archive_capture.py",
                ],
                ["rate", "FYI_LIVE_SMOKE", "attachment"],
            ),
        },
    ]

    gaps = [
        {
            "surface": "rust_sync_client",
            "risk": "closed",
            "gap": (
                "HTTP 401/403/404/429/5xx responses are covered by mocked "
                "non-secret error contract tests."
            ),
        },
        {
            "surface": "rust_sync_client",
            "risk": "closed",
            "gap": (
                "Malformed JSON, missing required fields, and unexpected "
                "optional fields are covered by regression tests."
            ),
        },
        {
            "surface": "rust_cli_sync_surface",
            "risk": "medium",
            "gap": (
                "CLI output has sync coverage, but API failure presentation "
                "needs end-to-end assertions."
            ),
        },
        {
            "surface": "rust_mcp_sync_surface",
            "risk": "medium",
            "gap": (
                "MCP database errors are surfaced, but upstream API error "
                "normalization is indirect until sync error types are added."
            ),
        },
        {
            "surface": "archive_public_web",
            "risk": "medium",
            "gap": "Live public-web smoke remains opt-in to avoid network-dependent CI.",
        },
    ]

    for surface in surfaces:
        surface["status"] = _status(repo_root, surface["files"])

    return {"surfaces": surfaces, "gaps": gaps}


def render_markdown(inventory: dict[str, Any]) -> str:
    """Render inventory data as a stable markdown report."""
    lines = [
        "# API Contract Inventory",
        "",
        "Generated from checked-in source and test evidence for the "
        "`api-contract-hardening-20260630` Conductor track.",
        "",
        "## Contract Matrix",
        "",
        "| Surface | Contract | Coverage | Risk |",
        "| --- | --- | --- | --- |",
    ]

    for surface in inventory["surfaces"]:
        files = ", ".join(f"`{path}`" for path in surface["files"])
        tests = ", ".join(f"`{path}`" for path in surface["tests"])
        coverage = f"{surface['coverage']} Evidence: {files}; tests: {tests}."
        lines.append(
            "| {surface_id}: {surface} | {contract} | {coverage} | {risk} |".format(
                surface_id=surface["id"],
                surface=surface["surface"],
                contract=surface["contract"],
                coverage=coverage,
                risk=surface["risk"],
            ),
        )

    lines.extend(
        [
            "",
            "## Contract Gaps And Residual Risk",
            "",
        ],
    )
    lines.extend(
        f"- **{gap['risk']}** `{gap['surface']}`: {gap['gap']}"
        for gap in inventory["gaps"]
    )

    lines.extend(
        [
            "",
            "## Phase 1 Next Actions",
            "",
            "1. Keep live FYI smoke tests opt-in with `FYI_LIVE_SMOKE=1`.",
            "2. Run mocked contract tests before release.",
            "3. Refresh fixtures when FYI/Alaveteli response shapes change.",
            "",
        ],
    )
    return "\n".join(lines)


def main() -> int:
    """Generate the contract inventory markdown report."""
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root",
        default=Path(__file__).resolve().parents[1],
        type=Path,
    )
    parser.add_argument(
        "--output",
        default=Path("docs/api-contract-inventory.md"),
        type=Path,
    )
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    output = args.output
    if not output.is_absolute():
        output = repo_root / output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        render_markdown(build_contract_inventory(repo_root)),
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
