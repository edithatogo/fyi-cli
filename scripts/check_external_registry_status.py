#!/usr/bin/env python3
"""Poll external MCP registry surfaces without mutating them.

The check is intentionally not part of default CI. It is used by the scheduled
external-registry monitor and can be run locally with ``--json`` or ``--output``.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

if TYPE_CHECKING:
    from collections.abc import Callable

SMITHERY_NAMESPACE_URL = "https://registry.smithery.ai/servers?namespace=edithatogo"
SMITHERY_DETAIL_URL = "https://registry.smithery.ai/servers/@edithatogo/fyi-mcp"
OFFICIAL_REGISTRY_URL = "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp"
GITHUB_SEARCH_URL = "https://github.com/mcp?q=fyi-mcp"
GITHUB_DIRECT_URL = "https://github.com/mcp/io.github.edithatogo/fyi-mcp"
USER_AGENT = "fyi-cli external-registry-monitor/1.0 (+https://github.com/edithatogo/fyi-cli)"
ALLOWED_ENDPOINTS = frozenset(
    {
        SMITHERY_NAMESPACE_URL,
        SMITHERY_DETAIL_URL,
        OFFICIAL_REGISTRY_URL,
        GITHUB_SEARCH_URL,
        GITHUB_DIRECT_URL,
    },
)


@dataclass(frozen=True)
class FetchResult:
    status: int
    body: str
    error: str | None = None


def fetch_url(url: str) -> FetchResult:
    if url not in ALLOWED_ENDPOINTS:
        message = f"unsupported registry endpoint: {url}"
        raise ValueError(message)
    request = Request(url, headers={"User-Agent": USER_AGENT})  # noqa: S310
    try:
        with urlopen(request, timeout=30) as response:  # noqa: S310 - fixed HTTPS endpoints only
            return FetchResult(response.status, response.read().decode("utf-8", "replace"))
    except HTTPError as exc:
        try:
            body = exc.read().decode("utf-8", "replace")
        except Exception:  # noqa: BLE001 - an HTTP error must not crash the monitor
            body = ""
        return FetchResult(exc.code, body)
    except (TimeoutError, URLError, OSError) as exc:
        return FetchResult(0, "", str(exc))


def _json(result: FetchResult) -> Any:
    if result.error or result.status < 200 or result.status >= 300:
        return None
    try:
        return json.loads(result.body)
    except json.JSONDecodeError:
        return None


def _official_latest(payload: Any) -> dict[str, Any] | None:
    rows = payload.get("servers", []) if isinstance(payload, dict) else []
    matches = [
        row
        for row in rows
        if isinstance(row, dict)
        and isinstance(row.get("server"), dict)
        and row["server"].get("name") == "io.github.edithatogo/fyi-mcp"
    ]
    latest = [
        row
        for row in matches
        if row.get("_meta", {}).get("io.modelcontextprotocol.registry/official", {}).get("isLatest")
    ]
    row = latest[0] if latest else (matches[-1] if matches else None)
    if row is None:
        return None
    metadata = row.get("_meta", {}).get("io.modelcontextprotocol.registry/official", {})
    return {
        "version": row.get("server", {}).get("version"),
        "status": metadata.get("status"),
        "is_latest": bool(metadata.get("isLatest")),
    }


def build_report(fetcher: Callable[[str], FetchResult] = fetch_url) -> dict[str, Any]:
    namespace = fetcher(SMITHERY_NAMESPACE_URL)
    detail = fetcher(SMITHERY_DETAIL_URL)
    official = fetcher(OFFICIAL_REGISTRY_URL)
    github_search = fetcher(GITHUB_SEARCH_URL)
    github_direct = fetcher(GITHUB_DIRECT_URL)

    namespace_payload = _json(namespace) or {}
    listing = next(
        (row for row in namespace_payload.get("servers", []) if row.get("slug") == "fyi-mcp"),
        {},
    )
    detail_payload = _json(detail) or {}
    official_latest = _official_latest(_json(official))
    errors = {
        name: result.error
        for name, result in {
            "smithery_namespace": namespace,
            "smithery_detail": detail,
            "official_registry": official,
            "github_search": github_search,
            "github_direct": github_direct,
        }.items()
        if result.error
    }
    github_listed = github_direct.status == 200
    evidence = {
        "smithery": {
            "status": namespace.status,
            "score": listing.get("score"),
            "use_count": listing.get("useCount"),
            "is_deployed": listing.get("isDeployed"),
            "remote": listing.get("remote"),
            "tools": len(detail_payload.get("tools", [])),
            "resources": len(detail_payload.get("resources", [])),
            "prompts": len(detail_payload.get("prompts", [])),
        },
        "official_registry": {"status": official.status, "latest": official_latest},
        "github_curated": {
            "search_status": github_search.status,
            "direct_status": github_direct.status,
            "listed": github_listed,
        },
        "errors": errors,
    }
    fingerprint = hashlib.sha256(
        json.dumps(evidence, sort_keys=True, separators=(",", ":")).encode(),
    ).hexdigest()[:16]
    report = {
        "checked_at": datetime.now(UTC).isoformat(),
        "fingerprint": fingerprint,
        "status": "listed" if github_listed else "blocked-external",
        "evidence": evidence,
    }
    report["markdown"] = format_markdown(report)
    return report


def format_markdown(report: dict[str, Any]) -> str:
    evidence = report["evidence"]
    smithery = evidence["smithery"]
    official = evidence["official_registry"]["latest"] or {}
    github = evidence["github_curated"]
    return "\n".join(
        [
            f"<!-- registry-monitor:{report['fingerprint']} -->",
            f"## External registry monitor — {report['checked_at']}",
            f"- Status: **{report['status']}**",
            f"- Smithery: score={smithery['score']!r}, "
            f"useCount={smithery['use_count']!r}, "
            f"tools/resources/prompts={smithery['tools']}/{smithery['resources']}/{smithery['prompts']}",
            f"- Official MCP Registry: version={official.get('version')!r}, "
            f"status={official.get('status')!r}, isLatest={official.get('is_latest')!r}",
            f"- GitHub curated surface: search HTTP {github['search_status']}, "
            f"direct listing HTTP {github['direct_status']}",
            "- This check is read-only; manual curation remains required until "
            "the GitHub listing appears.",
        ],
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="write JSON to stdout")
    parser.add_argument("--output", type=Path, help="write JSON to this path")
    args = parser.parse_args()
    report = build_report()
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    if args.json or not args.output:
        sys.stdout.write(rendered)
    return 1 if report["evidence"]["errors"] else 0


if __name__ == "__main__":
    sys.exit(main())
