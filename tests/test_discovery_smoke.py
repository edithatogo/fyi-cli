"""Opt-in live smoke tests for FYI archive discovery."""

from __future__ import annotations

import os

import httpx
import pytest

from fyi_system.archive_capture import CaptureCaps, capture_request
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


@pytest.mark.skipif(
    os.environ.get("FYI_LIVE_SMOKE") != "1",
    reason="set FYI_LIVE_SMOKE=1 to run live RightToKnow smoke tests",
)
def test_righttoknow_discover_and_capture_live_smoke(tmp_path) -> None:
    """Prove bounded AU discovery plus read-only capture at default pacing."""
    base_url = "https://www.righttoknow.org.au"
    try:
        rows = discover_feed(
            base_url=base_url,
            max_pages=1,
            delay_seconds=1.0,
            shared_rate_limit_db_path=tmp_path / "rate-limit.db",
            shared_rate_limit_name="archive-discovery-au-rtk",
        )
    except httpx.HTTPStatusError as exc:
        if exc.response.status_code == 403:
            pytest.skip(
                "RightToKnow denied the bounded smoke request with HTTP 403; "
                "capture was not attempted",
            )
        raise
    if not rows:
        pytest.skip("RightToKnow returned no public requests on the smoke page")

    selected = rows[:5]
    for row in selected:
        summary = capture_request(
            request_ref=str(row.request_id),
            base_url=base_url,
            data_dir=tmp_path / "data",
            dist_dir=tmp_path / "dist",
            caps=CaptureCaps(max_bytes=25 * 1024 * 1024, max_runtime_minutes=5),
        )
        assert summary["resources"]
