"""Tests for the read-only external registry monitor."""

from __future__ import annotations

import json

from scripts.check_external_registry_status import FetchResult, build_report


def test_build_report_identifies_external_github_blocker() -> None:
    responses = {
        "https://registry.smithery.ai/servers?namespace=edithatogo": FetchResult(
            200,
            '{"servers":[{"slug":"fyi-mcp","score":null,"useCount":0,"isDeployed":true,"remote":false}]}',
        ),
        "https://registry.smithery.ai/servers/@edithatogo/fyi-mcp": FetchResult(
            200,
            '{"tools":[1,2],"resources":[1],"prompts":[1,2,3]}',
        ),
        "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp": FetchResult(
            200,
            '{"servers":[{"server":{"name":"io.github.edithatogo/fyi-mcp","version":"0.1.2"},'
            '"_meta":{"io.modelcontextprotocol.registry/official":{"status":"active","isLatest":true}}}]}',
        ),
        "https://github.com/mcp?q=fyi-mcp": FetchResult(200, "fyi-mcp"),
        "https://github.com/mcp/io.github.edithatogo/fyi-mcp": FetchResult(404, ""),
    }

    report = build_report(responses.__getitem__)

    assert report["status"] == "blocked-external"
    assert report["evidence"]["smithery"]["tools"] == 2
    assert report["evidence"]["official_registry"]["latest"]["is_latest"] is True
    assert report["evidence"]["github_curated"]["listed"] is False
    assert report["fingerprint"] in report["markdown"]


def test_build_report_marks_github_listing_ready() -> None:
    responses = {
        "https://registry.smithery.ai/servers?namespace=edithatogo": FetchResult(
            200,
            json.dumps({"servers": []}),
        ),
        "https://registry.smithery.ai/servers/@edithatogo/fyi-mcp": FetchResult(
            200,
            "{}",
        ),
        "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp": FetchResult(
            200,
            '{"servers":[]}',
        ),
        "https://github.com/mcp?q=fyi-mcp": FetchResult(200, "card"),
        "https://github.com/mcp/io.github.edithatogo/fyi-mcp": FetchResult(200, "listing"),
    }

    assert build_report(responses.__getitem__)["status"] == "listed"
