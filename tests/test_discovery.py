"""Tests for archive discovery helpers."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import httpx
import pytest

from fyi_system.db import acquire_shared_rate_limit
from fyi_system.discovery import (
    PoliteRateLimiter,
    backfill_ids,
    discover_feed,
    get_with_backoff,
    parse_feed_entries,
    reconcile_discovery_files,
    shared_rate_limit_status,
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
    assert entries[0].authority == "Agency"
    assert has_next is True


def test_parse_feed_entries_normalizes_authority_objects() -> None:
    entries, _ = parse_feed_entries(
        {
            "entries": [
                {
                    "id": 1,
                    "url_title": "one",
                    "authority": {"url_name": "agency", "name": "Agency"},
                },
            ],
        },
    )

    assert entries[0].authority == "agency"


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
        delay_seconds=0,
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


def test_get_with_backoff_recovers_from_transport_timeout() -> None:
    attempts = 0

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal attempts
        attempts += 1
        if attempts == 1:
            raise httpx.ReadTimeout("transient timeout", request=request)
        return httpx.Response(200, json={"ok": True}, request=request)

    with httpx.Client(
        base_url="https://example.test",
        transport=httpx.MockTransport(handler),
    ) as http:
        response = get_with_backoff(
            http,
            "/request/1.json",
            disallows=[],
            retries=2,
            backoff_seconds=0,
            sleeper=lambda _: None,
        )
    assert response.status_code == 200
    assert attempts == 2


def test_get_with_backoff_enforces_rate_cap() -> None:
    sleeps: list[float] = []
    now = {"value": 0.0}

    def sleeper(seconds: float) -> None:
        sleeps.append(seconds)
        now["value"] += seconds

    limiter = PoliteRateLimiter(
        1.0,
        jitter_seconds=0.5,
        clock=lambda: now["value"],
        sleeper=sleeper,
        randomizer=lambda: 0.5,
    )

    with httpx.Client(
        transport=httpx.MockTransport(lambda request: httpx.Response(200, request=request)),
        base_url="https://fyi.example",
    ) as http:
        get_with_backoff(
            http,
            "/search/all",
            disallows=[],
            rate_limiter=limiter,
            backoff_seconds=0,
        )
        get_with_backoff(
            http,
            "/search/all",
            disallows=[],
            rate_limiter=limiter,
            backoff_seconds=0,
        )

    assert sleeps == [1.25]


def test_get_with_backoff_falls_back_when_shared_limiter_fails() -> None:
    local_waits: list[str] = []

    class SharedLimiter:
        def wait(self, _interval_seconds: float) -> None:
            msg = "shared limiter unavailable"
            raise RuntimeError(msg)

        def backoff(self, _delay_seconds: float, *, status_code: int | None = None) -> None:
            _ = status_code
            msg = "shared limiter unavailable"
            raise RuntimeError(msg)

    class LocalLimiter:
        def wait(self) -> None:
            local_waits.append("wait")

    with httpx.Client(
        transport=httpx.MockTransport(lambda request: httpx.Response(200, request=request)),
        base_url="https://fyi.example",
    ) as http:
        response = get_with_backoff(
            http,
            "/search/all",
            disallows=[],
            shared_rate_limiter=SharedLimiter(),
            rate_limiter=LocalLimiter(),
            backoff_seconds=0,
        )

    assert response.status_code == 200
    assert local_waits == ["wait"]


def test_shared_rate_limit_reserves_across_calls(tmp_path: Path) -> None:
    db_path = tmp_path / "fyi.db"

    first = acquire_shared_rate_limit(
        db_path,
        name="archive-discovery",
        interval_seconds=1.0,
        jitter_seconds=0.0,
        owner_id="worker-1",
        now=0.0,
        randomizer=lambda: 0.0,
    )
    second = acquire_shared_rate_limit(
        db_path,
        name="archive-discovery",
        interval_seconds=1.0,
        jitter_seconds=0.0,
        owner_id="worker-2",
        now=0.0,
        randomizer=lambda: 0.0,
    )

    status = shared_rate_limit_status(db_path, name="archive-discovery")

    assert first["sleep_seconds"] == 0.0
    assert second["sleep_seconds"] == 1.0
    assert status is not None
    assert status["last_owner_id"] == "worker-2"
    assert status["interval_seconds"] == 1.0
    assert status["recent_events"][-1]["kind"] == "acquired"


def test_shared_rate_limit_records_backoff_events(tmp_path: Path) -> None:
    db_path = tmp_path / "fyi.db"
    attempts = {"count": 0}

    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.path == "/robots.txt":
            return httpx.Response(200, text="User-agent: *\nDisallow:\n", request=request)
        attempts["count"] += 1
        if attempts["count"] == 1:
            return httpx.Response(429, request=request)
        return httpx.Response(
            200,
            json={"entries": [{"request_id": 1, "url_title": "one"}]},
            request=request,
        )

    rows = discover_feed(
        base_url="https://fyi.example",
        delay_seconds=0,
        shared_rate_limit_db_path=db_path,
        transport=httpx.MockTransport(handler),
    )

    status = shared_rate_limit_status(db_path, name="archive-discovery")

    assert [row.request_id for row in rows] == [1]
    assert attempts["count"] == 2
    assert status is not None
    assert any(
        event["kind"] == "backoff" and event["status_code"] == 429
        for event in status["recent_events"]
    )


def test_discover_feed_uses_shared_rate_limit_db(tmp_path: Path) -> None:
    db_path = tmp_path / "fyi.db"

    rows = discover_feed(
        base_url="https://fyi.example",
        delay_seconds=0,
        shared_rate_limit_db_path=db_path,
        transport=httpx.MockTransport(
            lambda request: httpx.Response(
                200,
                json={"entries": [{"request_id": 1, "url_title": "one"}]},
                request=request,
            ),
        ),
    )

    status = shared_rate_limit_status(db_path, name="archive-discovery")

    assert [row.request_id for row in rows] == [1]
    assert status is not None


def test_reconcile_discovery_files_reports_set_gaps(tmp_path: Path) -> None:
    feed = tmp_path / "feed.jsonl"
    backfill = tmp_path / "backfill.jsonl"
    feed.write_text(
        '{"request_id": 1, "url_title": "one"}\n{"request_id": 2, "url_title": "two"}\n',
        encoding="utf-8",
    )
    backfill.write_text(
        '{"request_id": 2, "url_title": "two"}\n{"request_id": 3, "url_title": "three"}\n',
        encoding="utf-8",
    )

    report = reconcile_discovery_files(feed, backfill)

    assert report.feed_count == 2
    assert report.backfill_count == 2
    assert report.matched_count == 1
    assert report.missing_from_feed == [3]
    assert report.missing_from_backfill == [1]
    assert report.is_complete is False
