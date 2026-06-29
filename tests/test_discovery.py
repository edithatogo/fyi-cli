"""Tests for archive discovery helpers."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import httpx
import pytest

from fyi_system.discovery import (
    backfill_ids,
    discover_feed,
    get_with_backoff,
    parse_feed_entries,
    write_jsonl,
)

if TYPE_CHECKING:
    from pathlib import Path


def test_parse_feed_entries_dedupes_requests() -> None:
    entries, has_next = parse_feed_entries(
        {
            "entries": [
                {
                    "id": "1",
                    "url": "https://fyi.org.nz/request/example_request",
                    "title": "Example",
                    "public_body": "Agency",
                },
                {"request_id": 1, "url_title": "example_request_duplicate"},
            ],
            "next": "page-2",
        },
    )

    assert len(entries) == 1
    assert entries[0].request_id == 1
    assert entries[0].url_title == "example_request"
    assert has_next is True


def test_discover_feed_paginates_and_writes_checkpoint(tmp_path: Path) -> None:
    checkpoint = tmp_path / "checkpoint.json"

    def handler(request: httpx.Request) -> httpx.Response:
        page = request.url.params.get("page")
        if page == "1":
            return httpx.Response(
                200,
                json={
                    "entries": [
                        {
                            "request_id": 1,
                            "url_title": "one",
                            "title": "One",
                        },
                    ],
                    "next": "page-2",
                },
            )
        return httpx.Response(
            200,
            json={"entries": [{"request_id": 2, "url_title": "two", "title": "Two"}]},
        )

    rows = discover_feed(
        base_url="https://fyi.example",
        checkpoint_path=checkpoint,
        delay_seconds=0,
        transport=httpx.MockTransport(handler),
    )

    assert [row.request_id for row in rows] == [1, 2]
    assert json.loads(checkpoint.read_text(encoding="utf-8"))["next_page"] == 3


def test_backfill_ids_follows_redirect_and_skips_404() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        if str(request.url).endswith("/request/1.json"):
            return httpx.Response(
                200,
                json={"title": "One", "created_at": "2026-01-01"},
                request=request,
                headers={"content-type": "application/json"},
            )
        return httpx.Response(404, request=request)

    rows = backfill_ids(
        id_from=1,
        id_to=2,
        base_url="https://fyi.example",
        transport=httpx.MockTransport(handler),
    )

    assert [row.request_id for row in rows] == [1]


def test_write_jsonl(tmp_path: Path) -> None:
    output = tmp_path / "requests.jsonl"

    write_jsonl(
        output,
        discover_feed(
            base_url="https://fyi.example",
            delay_seconds=0,
            transport=httpx.MockTransport(
                lambda request: httpx.Response(
                    200,
                    json={"entries": [{"request_id": 1, "url_title": "one"}]},
                    request=request,
                ),
            ),
        ),
    )

    assert '"request_id": 1' in output.read_text(encoding="utf-8")


def test_discover_feed_honours_robots_disallow() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.path == "/robots.txt":
            return httpx.Response(200, text="User-agent: *\nDisallow: /search\n", request=request)
        return httpx.Response(200, json={"entries": []}, request=request)

    with pytest.raises(PermissionError, match=r"robots.txt disallows"):
        discover_feed(
            base_url="https://fyi.example",
            delay_seconds=0,
            transport=httpx.MockTransport(handler),
        )


def test_get_with_backoff_recovers_from_429() -> None:
    attempts = {"count": 0}

    def handler(request: httpx.Request) -> httpx.Response:
        attempts["count"] += 1
        if attempts["count"] == 1:
            return httpx.Response(429, request=request)
        return httpx.Response(200, json={"entries": []}, request=request)

    with httpx.Client(
        transport=httpx.MockTransport(handler),
        base_url="https://fyi.example",
    ) as http:
        response = get_with_backoff(
            http,
            "/search/all",
            disallows=[],
            backoff_seconds=0,
        )

    assert response.status_code == 200
    assert attempts["count"] == 2
