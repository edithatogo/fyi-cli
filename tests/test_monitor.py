"""Tests for monitor module - feed ingestion and snapshot monitoring."""
from types import SimpleNamespace

from fyi_system.db import connect
from fyi_system.monitor import ingest_feed, reconcile_events


class TestIngestFeed:
    """Test feed ingestion functionality."""

    def test_ingest_feed_returns_int(self, tmp_path):
        """Test that ingest_feed returns an integer count."""
        db_path = tmp_path / "test.db"
        # Initialize DB first
        from fyi_system.db import init_db
        init_db(str(db_path))
        # Use a real RSS feed for testing
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        result = ingest_feed(feed_url, str(db_path))
        assert isinstance(result, int)
        assert result >= 0  # Could be 0 if feed is empty or unavailable

    def test_ingest_feed_creates_database(self, tmp_path):
        """Test that ingest_feed creates the database file."""
        db_path = tmp_path / "test.db"
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        ingest_feed(feed_url, str(db_path))
        assert db_path.exists()

    def test_ingest_feed_with_path_object(self, tmp_path):
        """Test that ingest_feed accepts Path objects."""
        db_path = tmp_path / "test.db"
        # Initialize DB first
        from fyi_system.db import init_db
        init_db(str(db_path))
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        result = ingest_feed(feed_url, db_path)
        assert isinstance(result, int)

    def test_ingest_feed_default_db(self, monkeypatch, tmp_path):
        """Test that ingest_feed uses default database path."""
        # Change to temp directory to avoid creating files in current dir
        monkeypatch.chdir(tmp_path)
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        result = ingest_feed(feed_url)
        assert isinstance(result, int)


class TestIngestFeedDST:
    """Deterministic simulation tests for feed ingestion.

    These tests mock feedparser.parse() to return deterministic entries,
    ensuring replayable results without live network access.
    """

    def _make_entry(self, entry_id, title, link, published, summary):
        """Create a SimpleNamespace entry mimicking a feedparser entry."""
        entry = SimpleNamespace(
            id=entry_id,
            title=title,
            link=link,
            published=published,
            summary=summary,
        )
        # Support .keys() and .get() like a dict
        entry.keys = lambda: ["id", "title", "link", "published", "summary"]
        entry.get = lambda k, default=None: getattr(entry, k, default)
        return entry

    def test_ingest_feed_dst_deterministic_entries(self, tmp_path, monkeypatch):
        """DST: ingest_feed produces identical results across repeated runs
        with the same simulated feed entries."""
        import fyi_system.monitor as monitor_module

        db_path = tmp_path / "test_dst.db"
        from fyi_system.db import init_db
        init_db(str(db_path))

        entries = [
            self._make_entry(
                "1", "Request One", "https://fyi.org.nz/request/1",
                "2026-01-01", "First request",
            ),
            self._make_entry(
                "2", "Request Two", "https://fyi.org.nz/request/2",
                "2026-01-02", "Second request",
            ),
        ]

        def mock_parse(_url):
            return SimpleNamespace(entries=entries)

        monkeypatch.setattr(monitor_module.feedparser, "parse", mock_parse)

        result1 = ingest_feed("https://example.test/feed", str(db_path))
        db_path.unlink()
        init_db(str(db_path))
        result2 = ingest_feed("https://example.test/feed", str(db_path))

        assert result1 == 2
        assert result2 == 2

    def test_ingest_feed_dst_bounded_runtime(self, tmp_path, monkeypatch):
        """DST: ingest_feed completes within bounded runtime for a feed
        with many entries (no wall-clock dependencies)."""
        import time

        import fyi_system.monitor as monitor_module

        db_path = tmp_path / "test_dst_bounded.db"
        from fyi_system.db import init_db
        init_db(str(db_path))

        # Simulate 50 entries
        entries = [
            self._make_entry(
                str(i), f"Request {i}", f"https://fyi.org.nz/request/{i}",
                "2026-01-01", f"Request {i}",
            )
            for i in range(50)
        ]

        def mock_parse(_url):
            return SimpleNamespace(entries=entries)

        monkeypatch.setattr(monitor_module.feedparser, "parse", mock_parse)

        start = time.monotonic()
        result = ingest_feed("https://example.test/feed", str(db_path))
        elapsed = time.monotonic() - start

        assert result == 50
        # Bounded runtime: should complete well under 1 second for 50 entries
        assert elapsed < 1.0, f"ingest_feed took {elapsed:.3f}s for 50 entries, exceeding 1s bound"

    def test_ingest_feed_dst_empty_feed(self, tmp_path, monkeypatch):
        """DST: ingest_feed handles an empty feed deterministically."""
        import fyi_system.monitor as monitor_module

        db_path = tmp_path / "test_dst_empty.db"
        from fyi_system.db import init_db
        init_db(str(db_path))

        def mock_parse(_url):
            return SimpleNamespace(entries=[])

        monkeypatch.setattr(monitor_module.feedparser, "parse", mock_parse)

        result = ingest_feed("https://example.test/empty-feed", str(db_path))
        assert result == 0

    def test_ingest_feed_dst_unavailable_feed(self, tmp_path, monkeypatch):
        """DST: ingest_feed handles an unavailable feed gracefully (best-effort)."""
        import fyi_system.monitor as monitor_module

        db_path = tmp_path / "test_dst_unavailable.db"
        from fyi_system.db import init_db
        init_db(str(db_path))

        error_msg = "feed unavailable"

        def mock_parse(_url):
            raise OSError(error_msg)

        monkeypatch.setattr(monitor_module.feedparser, "parse", mock_parse)

        result = ingest_feed("https://example.test/unavailable", str(db_path))
        assert result == 0


# Note: latest_snapshot_summary is tested in test_fetch.py


class TestReconcileEvents:
    """Test event reconciliation functionality."""

    def test_reconcile_events_exists(self):
        """Test that reconcile_events function exists."""
        assert callable(reconcile_events)

    def test_reconcile_events_returns_int(self, tmp_path):
        """Test that reconcile_events returns an integer."""
        db_path = tmp_path / "test.db"
        # Initialize DB first to create tables
        from fyi_system.db import init_db
        init_db(str(db_path))
        result = reconcile_events(str(db_path))
        assert isinstance(result, int)
        assert result >= 0

    def test_reconcile_events_with_path_object(self, tmp_path):
        """Test reconcile_events accepts Path objects."""
        db_path = tmp_path / "test.db"
        # Initialize DB first to create tables
        from fyi_system.db import init_db
        init_db(str(db_path))
        result = reconcile_events(db_path)
        assert isinstance(result, int)

    def test_reconcile_events_with_feed_events(self, tmp_path):
        """Test reconcile_events matches feed events to requests."""
        from fyi_system.db import init_db, insert_tracked_request
        db_path = tmp_path / "test.db"
        init_db(str(db_path))

        # Create a tracked request with fyi_request_id
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug="test",
            title="Test",
            body="Test",
            fyi_request_id=12345,
        )

        # Insert a feed event that should match
        conn = connect(str(db_path))
        conn.execute(
            """INSERT INTO feed_events(feed_url, event_id, title, link, request_id_guess)
               VALUES (?, ?, ?, ?, ?)""",
            ("https://example.com/feed", "event1", "Test Event", "https://fyi.org.nz/request/12345", 12345),
        )
        conn.commit()
        conn.close()

        # Reconcile should match them
        result = reconcile_events(str(db_path))
        assert result >= 1
