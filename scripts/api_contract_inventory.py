"""Generate the FYI/Alaveteli API contract inventory report."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any
from urllib.parse import parse_qsl, urlsplit

# These expectations describe the referenced upstream controller, not the
# version or behavior of any deployed instance. The official prose differs on
# form parameter names; keep the pinned implementation and prose visible.
UPSTREAM_REVISION = "3966415f0b2a9f445fa3bb25c06eaeb75ded6609"
UPSTREAM_ROOT = f"https://github.com/mysociety/alaveteli/blob/{UPSTREAM_REVISION}"
UPSTREAM_CASES = [
    {
        "id": "authority_read_request",
        "fixture_case": "request_complete",
        "exchange_index": 0,
        "method": "GET",
        "status": 200,
        "path": "/api/v2/request/{request_id}.json",
        "parameters": ["k"],
        "response_fields": ["id", "title", "url", "request_text"],
    },
    {
        "id": "authority_create_external_request",
        "fixture_case": "create",
        "exchange_index": 0,
        "method": "POST",
        "status": 200,
        "path": "/api/v2/request.json",
        "parameters": ["k", "request_json"],
        "json_string_parameters": ["request_json"],
        "response_fields": ["id", "url"],
    },
    {
        "id": "authority_correspondence",
        "fixture_case": "correspondence_json",
        "exchange_index": 0,
        "method": "POST",
        "status": 200,
        "path": "/api/v2/request/{request_id}.json",
        "parameters": ["k", "correspondence_json", "state"],
        "json_string_parameters": ["correspondence_json"],
        "response_fields": ["url"],
    },
    {
        "id": "authority_response_attachment",
        "fixture_case": "correspondence_multipart",
        "exchange_index": 0,
        "method": "POST",
        "status": 200,
        "path": "/api/v2/request/{request_id}.json",
        "parameters": ["k", "correspondence_json", "state", "attachments"],
        "json_string_parameters": ["correspondence_json"],
        "response_fields": ["url"],
    },
    {
        "id": "authority_update_state",
        "fixture_case": "state_preflight",
        "exchange_index": 1,
        "method": "POST",
        "status": 200,
        "path": "/api/v2/request/{request_id}/update.json",
        "parameters": ["k", "state"],
        "response_fields": ["url"],
    },
]
MCP_FIXTURE = "tests/fixtures/remote_mcp/upstream-v2.json"
MCP_STOCK_FIXTURE = "tests/fixtures/remote_mcp/upstream-stock-v1.json"

REQUIRED_SURFACES = {
    "rust_api_payloads",
    "rust_sync_client",
    "rust_cli_sync_surface",
    "rust_mcp_sync_surface",
    "legacy_python_api",
    "archive_public_web",
}


def _is_json_object_text(value: Any) -> bool:
    if not isinstance(value, str):
        return False
    try:
        return isinstance(json.loads(value), dict)
    except ValueError:
        return False


def compare_upstream_case(
    reference: dict[str, Any],
    exchange: dict[str, Any],
    *,
    request_id: int,
) -> list[str]:
    """Compare synthetic fixture fields with a pinned upstream controller case.

    Values, including synthetic credentials, are never included in findings.
    This comparison does not execute a client or qualify a deployed instance.
    """
    findings = []
    if exchange.get("method") != reference["method"]:
        findings.append("method")
    if "status" in reference and exchange.get("status") != reference["status"]:
        findings.append("status")
    parsed = urlsplit(exchange.get("path", ""))
    if parsed.path != reference["path"].format(request_id=request_id):
        findings.append("path")
    parameters = dict(parse_qsl(parsed.query, keep_blank_values=True))
    for body_name in ("request_json", "request_parts"):
        body = exchange.get(body_name, {})
        if not isinstance(body, dict):
            findings.append(f"invalid fixture {body_name}")
            continue
        parameters.update((name.removesuffix("[]"), value) for name, value in body.items())
    findings.extend(
        f"missing parameter {name}"
        for name in reference.get("parameters", [])
        if name not in parameters
    )
    if "k" in parameters and (not isinstance(parameters["k"], str) or not parameters["k"].strip()):
        findings.append("parameter k is empty or invalid")
    findings.extend(
        f"parameter {name} must be JSON text"
        for name in reference.get("json_string_parameters", [])
        if name in parameters and not _is_json_object_text(parameters[name])
    )
    response = exchange.get("json")
    response = response if isinstance(response, dict) else {}
    findings.extend(
        f"missing response field {name}"
        for name in reference.get("response_fields", [])
        if name not in response
    )
    return findings


def compare_upstream_fixture(
    reference: list[dict[str, Any]],
    fixture: dict[str, Any],
) -> list[dict[str, Any]]:
    """Return a finding for every reference case, including absent fixtures."""
    ids = [case["id"] for case in reference]
    if not ids or len(ids) != len(set(ids)):
        message = "reference cases must be present and have unique ids"
        raise ValueError(message)
    cases = fixture.get("cases", [])
    if not isinstance(cases, list):
        message = "fixture cases must be a list"
        raise TypeError(message)
    result = []
    for expected in reference:
        case_name = expected["fixture_case"]
        matching = [
            case for case in cases if isinstance(case, dict) and case.get("id") == case_name
        ]
        if not matching:
            differences = ["fixture case missing"]
        elif len(matching) != 1:
            differences = ["fixture case duplicated"]
        else:
            case = matching[0]
            exchanges = case.get("exchanges")
            index = expected["exchange_index"]
            if not isinstance(exchanges, list) or not 0 <= index < len(exchanges):
                differences = ["fixture exchange missing"]
            elif not isinstance(exchanges[index], dict):
                differences = ["invalid fixture exchange"]
            else:
                request_id = case.get("arguments", {}).get("request_id", 0)
                differences = compare_upstream_case(
                    expected,
                    exchanges[index],
                    request_id=request_id,
                )
        result.append({"id": expected["id"], "fixture_case": case_name, "differences": differences})
    return result


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
                "Local client paths for request reads, search, source writes, "
                "feeds, retry queues and conflict preservation; an explicit "
                "pinned-upstream authority read and import profile is available."
            ),
            "files": [
                "crates/fyi-core/src/sync.rs",
                "crates/fyi-core/src/sync/read_contract.rs",
                "crates/fyi-core/src/sync/alaveteli_stock.rs",
            ],
            "tests": [
                "crates/fyi-core/src/sync.rs::tests",
                "crates/fyi-core/src/sync/read_contract/tests.rs",
            ],
            "coverage": (
                "Mocked successful pull, feed pull, malformed JSON, missing "
                "required fields, HTTP 401/403/404/429/5xx failures, health, "
                "push retry, scheduler, and conflict merge behavior. The "
                "pinned authority read and selected imports have socket-free "
                "injected-transport tests."
            ),
            "risk": "high",
        },
        {
            "id": "rust_cli_sync_surface",
            "surface": "Rust CLI sync commands",
            "contract": (
                "CLI presentation for sync status, pull, push, conflicts, and conflict resolution."
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
                "Parser and local CLI tests; register-request is local only; "
                "push receipt transport evidence is unavailable."
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
                "Local tools and separately enabled remote reads and governed "
                "create, correspondence and state writes."
            ),
            "files": ["crates/fyi-mcp/src/main.rs"],
            "tests": [
                "crates/fyi-mcp/src/main.rs::tests",
                "tests/test_mcp_upstream_contract.py",
                "crates/fyi-mcp/src/stock_contract_tests.rs",
            ],
            "coverage": (
                "The existing provider has actual-binary synthetic tests. "
                "The opt-in stock profile has five socket-free handler "
                "fixture cases; neither fixture certifies a deployment."
            ),
            "risk": "high",
        },
        {
            "id": "legacy_python_api",
            "surface": "Legacy Python Alaveteli client",
            "contract": (
                "Read, disclosure-log write, correspondence, attachment and "
                "state methods; not an ordinary account-session client."
            ),
            "files": ["src/fyi_system/alaveteli_client.py"],
            "tests": ["tests/test_alaveteli_client.py"],
            "coverage": (
                "Mocked request and response methods use `k` and the documented "
                "`json` form field; the pinned controller expects different "
                "parameter names and separate state."
            ),
            "risk": "high",
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
            "risk": "high",
            "gap": (
                "The synthetic source profile differs from the pinned upstream "
                "controller on routes, authority authentication, parameters "
                "and response shapes; see the reference comparison below."
            ),
        },
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
                "Push cannot yet produce an acquisition receipt. Local "
                "register-request does not submit or create a user account."
            ),
        },
        {
            "surface": "rust_mcp_sync_surface",
            "risk": "high",
            "gap": (
                "The stock profile's selected handler cases use an in-process "
                "synthetic fixture; the pinned upstream application and deployed "
                "instances have not been exercised."
            ),
        },
        {
            "surface": "legacy_python_api",
            "risk": "high",
            "gap": (
                "Python's mocked disclosure API requests follow the online "
                "prose's `json` field, not the pinned controller's named "
                "parameters; no live deployment is qualified."
            ),
        },
        {
            "surface": "legacy_python_api",
            "risk": "closed",
            "gap": (
                "Authority `k` is redacted from the covered URL, structured "
                "metadata and legacy HTTP error paths; source compatibility "
                "and complete credential handling remain separate gaps."
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

    fixture = json.loads((repo_root / MCP_FIXTURE).read_text(encoding="utf-8"))
    stock_fixture = json.loads((repo_root / MCP_STOCK_FIXTURE).read_text(encoding="utf-8"))
    return {
        "surfaces": surfaces,
        "gaps": gaps,
        "reference_comparison": compare_upstream_fixture(UPSTREAM_CASES, fixture),
        "stock_reference_comparison": compare_upstream_fixture(UPSTREAM_CASES, stock_fixture),
    }


def render_markdown(inventory: dict[str, Any]) -> str:
    """Render inventory data as a stable markdown report."""
    lines = [
        "# API Contract Inventory",
        "",
        "Generated by `scripts/api_contract_inventory.py` from checked-in "
        "source, tests and the pinned upstream reference. A synthetic fixture "
        "or a matching line-coverage percentage does not qualify a provider.",
        "",
        "## Pinned Alaveteli reference",
        "",
        f"The baseline is upstream commit [`{UPSTREAM_REVISION[:7]}`]"
        f"(https://github.com/mysociety/alaveteli/commit/{UPSTREAM_REVISION}): "
        f"[routes]({UPSTREAM_ROOT}/config/routes.rb), "
        f"[API controller]({UPSTREAM_ROOT}/app/controllers/api_controller.rb), "
        f"[API tests]({UPSTREAM_ROOT}/spec/controllers/api_controller_spec.rb), "
        f"[signup tests]({UPSTREAM_ROOT}/spec/controllers/user_controller_spec.rb), "
        f"[session tests]({UPSTREAM_ROOT}/spec/controllers/users/sessions_controller_spec.rb), "
        f"and [confirmation tests]({UPSTREAM_ROOT}/spec/controllers/users/"
        "confirmations_controller_spec.rb).",
        "The [official API prose](https://alaveteli.org/docs/developers/api/) "
        "describes `/api/v2/request` and `json` as a form field; the pinned "
        "route uses `/api/v2/request.json`, and its controller reads "
        "`request_json` or `correspondence_json` and a separate `state`. "
        "Resolve this discrepancy against the exact deployed version; do not "
        "assume either behavior on a fork.",
        "The checked [paired fork revision](https://github.com/edithatogo/alaveteli/"
        "blob/58d2f13e089145307b0d1d5ca460d08cfb84384a/app/controllers/api_controller.rb) "
        "uses the same named parameters, but its deployed use has not been established.",
        "",
        "The experimental authority API records external requests and "
        "correspondence. It does not establish an ordinary requester's "
        "signup, session or request-submission journey. The API controller "
        "uses the authority's `k` parameter; the fork-local bot header is a "
        "separate mechanism. User signup, confirmation, login and logout are "
        "web flows, not documented API operations.",
        "",
        "## Requested capability coverage",
        "",
        "| Capability | Current evidence | Remaining gap |",
        "| --- | --- | --- |",
        "| Public reads, search, authorities and feeds | Rust and Python paths and synthetic read "
        "fixtures exist. | Qualify each selected deployment, response shape, credentials and "
        "completeness. |",
        "| Authority API request read | The explicit Rust library and CLI authority profile "
        "use `k` and `request_text`; socket-free tests verify its raw/result receipt. "
        "The opt-in MCP handler consumes the same stock read profile. | "
        "A successful CLI-to-source run and "
        "deployed instance or requester session are not qualified. |",
        "| Register and confirm an account | Upstream web routes and controller tests are "
        "documented. | No FYI CLI/MCP remote-account signup and confirmation journey is "
        "verified. |",
        "| Login, challenges and logout | Local MFA/session primitives and synthetic "
        "authorization failures exist. | No supported FOI-site account session lifecycle or "
        "provider-specific challenge behavior is verified. |",
        "| Requester submits a new request or follow-up | Local `register-request` and a prefilled "
        "URL are available. | Neither establishes authenticated web submission or downstream "
        "delivery. |",
        "| Authority imports a request, correspondence or state | Rust/Python client and governed "
        "MCP paths have synthetic tests, including five stock handler fixture cases. | "
        "The legacy fixture differs from the pinned upstream wire contract; "
        "a running upstream and deployed compatibility are unverified. |",
        "| Response attachments | Synthetic multipart tests check bytes and size/path bounds. | "
        "Immutable prepare-to-send bytes and actual source acceptance "
        "remain unverified. |",
        "| Wayback receipt, WARC and WACZ | One offline fixture preserved the exact encoded "
        "entity and linked it to its verified receipt. | Other source commands, attachments, "
        "live replay and downstream archive shadow parity remain open. |",
        "",
        "## Pinned upstream versus current synthetic MCP fixture",
        "",
        "The comparator checks only the selected fixture method, status, path, "
        "parameter names and response-field presence. It does not execute "
        "the client or establish the behavior of a live upstream service. "
        f"Fixture: `{MCP_FIXTURE}`. Run "
        "`.venv/bin/python scripts/api_contract_inventory.py --check-upstream` "
        "to fail closed while any selected case differs.",
        "The five selected actual-binary SDK/fixture cases passed in the "
        "[PR #509 Python test run](https://github.com/edithatogo/fyi-cli/"
        "actions/runs/44996077720); "
        "that run tested the synthetic profile, not the pinned server.",
        f"The separate `{MCP_STOCK_FIXTURE}` contains the pinned reference "
        "fields and placeholder keys. The stock handler test executes all five "
        "cases with an injected in-process transport; it does not run the "
        "upstream application or certify a deployed server.",
        "",
        "| Upstream operation | Synthetic fixture case | Comparison |",
        "| --- | --- | --- |",
    ]

    for case in inventory["reference_comparison"]:
        comparison = "; ".join(case["differences"]) or "selected fields match; not qualified"
        lines.append(f"| `{case['id']}` | `{case['fixture_case']}` | {comparison} |")
    lines.extend(
        [
            "",
            "## Contract Matrix",
            "",
            "| Surface | Contract | Coverage | Risk |",
            "| --- | --- | --- | --- |",
        ],
    )

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
        f"- **{gap['risk']}** `{gap['surface']}`: {gap['gap']}" for gap in inventory["gaps"]
    )

    lines.extend(
        [
            "",
            "## Next actions",
            "",
            "1. Exercise the explicit CLI authority read end to end against an "
            "isolated source; reconcile the selected MCP stock profile and "
            "remaining operations against the target deployment.",
            "2. Reuse socket-free tests for selected reads, authority writes "
            "and account/session failures; cover the requester web journey "
            "separately from the experimental authority API.",
            "3. Verify a local isolated upstream workflow with synthetic "
            "accounts and intercepted mail; separately authorize any live "
            "site smoke. Keep `archive_cutover_allowed=false` until the "
            "existing downstream acceptance gates pass.",
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
    parser.add_argument(
        "--check-upstream",
        action="store_true",
        help="Compare the synthetic MCP fixture with the pinned upstream wire fields",
    )
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    inventory = build_contract_inventory(repo_root)
    if args.check_upstream:
        for case in inventory["reference_comparison"]:
            result = "; ".join(case["differences"]) or "selected fields match"
            sys.stdout.write(f"{case['id']}: {result}\n")
        for case in inventory["stock_reference_comparison"]:
            result = "; ".join(case["differences"]) or "selected fields match"
            sys.stdout.write(f"stock/{case['id']}: {result}\n")
        sys.stdout.write("Synthetic fixture comparison only; deployed behavior is not qualified.\n")
        return int(
            any(case["differences"] for case in inventory["reference_comparison"])
            or any(case["differences"] for case in inventory["stock_reference_comparison"]),
        )
    output = args.output
    if not output.is_absolute():
        output = repo_root / output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        render_markdown(inventory),
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
