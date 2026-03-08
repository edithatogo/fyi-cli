"""Tests for monitor module - feed ingestion and snapshot monitoring."""
import pytest
import sqlite3
from pathlib import Path
from fyi_system.monitor import ingest_feed, reconcile_events
from fyi_system.fetch import latest_snapshot_summary
from fyi_system.db import connect


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
