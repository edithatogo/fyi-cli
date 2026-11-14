"""Versioned synthetic upstream contracts exercised by the real Rust MCP server."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

FIXTURE = Path(__file__).parent / "fixtures/remote_mcp/upstream-v1.json"
REMOTE_TOOLS = {
    "remote_health",
    "remote_version",
    "remote_get_request",
    "remote_search_requests",
    "remote_list_authorities",
    "remote_create_request",
    "remote_add_correspondence",
    "remote_update_state",
}


@pytest.mark.compatibility
def test_versioned_upstream_fixture_covers_every_current_remote_tool() -> None:
    assert FIXTURE.is_file(), "versioned actual MCP-to-source exchange fixtures are missing"
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    assert fixture["schema_version"] == 1
    assert fixture["origin"] == "https://mcp-fixture.invalid"
    assert fixture["provenance"] == "synthetic; not live-platform certification"
    cases = fixture["cases"]
    assert {case["tool"] for case in cases} == REMOTE_TOOLS
    assert len({case["id"] for case in cases}) == len(cases)
    for case in cases:
        assert case["arguments"]["instance_id"] == "synthetic"
        assert case["exchanges"], "a source contract must observe real wire traffic"
        for exchange in case["exchanges"]:
            assert exchange["method"] in {"GET", "POST", "PUT"}
            assert exchange["path"].startswith("/")
            assert exchange["status"] in {200, 201}
