"""Opt-in live smoke tests for FYI archive discovery."""

from __future__ import annotations

import os

import pytest

from fyi_system.discovery import discover_feed

pytestmark = pytest.mark.smoke


@pytest.mark.skipif(
    os.environ.get("FYI_LIVE_SMOKE") != "1",
    reason="set FYI_LIVE_SMOKE=1 to run live FYI.org.nz smoke tests",
)
def test_discover_feed_live_single_page_smoke() -> None:
    rows = discover_feed(max_pages=1, delay_seconds=1.0)

    assert isinstance(rows, list)
    for row in rows:
        assert row.request_id > 0
        assert row.url_title
