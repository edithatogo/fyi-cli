"""Synthetic feed retrieval for legacy scheduler lifecycle tests only."""

from __future__ import annotations

from types import SimpleNamespace

import pytest


@pytest.fixture
def local_scheduler_feed(monkeypatch: pytest.MonkeyPatch) -> list[str]:
    """Keep the real monitor/database/report path while making acquisition local."""
    received: list[str] = []

    def parse_synthetic_feed(feed_url: str) -> SimpleNamespace:
        received.append(feed_url)
        if feed_url == "https://invalid.invalid/feed":
            message = "synthetic unavailable feed"
            raise OSError(message)
        return SimpleNamespace(entries=[])

    monkeypatch.setattr("fyi_system.monitor.feedparser.parse", parse_synthetic_feed)
    return received
