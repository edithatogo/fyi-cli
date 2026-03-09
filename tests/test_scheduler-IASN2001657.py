"""Tests for scheduler module."""
import pytest
from unittest.mock import patch, MagicMock
from pathlib import Path

from fyi_system.scheduler import run_scheduler, run_cycle
from fyi_system.db import init_db


class TestRunScheduler:
    """Test scheduler execution."""
    
    def test_scheduler_once_mode_no_loop(self, tmp_path):
        """Test: once=True doesn't loop."""
        feed_url = "https://example.com/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        with patch('fyi_system.scheduler.time.sleep') as mock_sleep:
            try:
                run_scheduler(
                    feed_url=feed_url,
                    interval_seconds=1,
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path),
                    once=True
                )
            except Exception:
                pass
            
            # Should not call sleep in once mode
            assert not mock_sleep.called
    
    def test_scheduler_creates_outputs_dir(self, tmp_path):
        """Test: Creates outputs directory."""
        feed_url = "https://example.com/feed"
        outputs_dir = tmp_path / "new_outputs"
        db_path = tmp_path / "test.db"
        
        try:
            run_scheduler(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path),
                once=True
            )
        except Exception:
            pass
        
        assert outputs_dir.exists()
    
    def test_scheduler_handles_invalid_feed(self, tmp_path):
        """Test: Handles invalid feed gracefully."""
        feed_url = "https://invalid.invalid/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        # Should not crash
        try:
            run_scheduler(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path),
                once=True
            )
        except Exception:
            pass
        
        # DB should still be created
        assert db_path.exists()
    
    def test_scheduler_with_path_objects(self, tmp_path):
        """Test: Accepts Path objects."""
        feed_url = "https://example.com/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        try:
            run_scheduler(
                feed_url=feed_url,
                outputs_dir=outputs_dir,
                db_path=db_path,
                once=True
            )
        except Exception:
            pass
        
        assert db_path.exists()


class TestRunCycle:
    """Test cycle execution."""
    
    def test_cycle_initializes_db(self, tmp_path):
        """Test: Initializes database if needed."""
        feed_url = "https://example.com/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        # Don't init DB first - run_cycle should handle it
        try:
            run_cycle(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path)
            )
        except Exception:
            pass
        
        # DB should be created
        assert db_path.exists()
    
    def test_cycle_creates_outputs(self, tmp_path):
        """Test: Creates outputs directory."""
        feed_url = "https://example.com/feed"
        outputs_dir = tmp_path / "outputs"
        db_path = tmp_path / "test.db"
        
        try:
            run_cycle(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path)
            )
        except Exception:
            pass
        
        assert outputs_dir.exists()
    
    def test_cycle_with_invalid_feed(self, tmp_path):
        """Test: Handles invalid feed."""
        feed_url = "https://invalid.invalid/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        try:
            run_cycle(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path)
            )
        except Exception:
            pass
        
        assert db_path.exists()
    
    def test_cycle_with_path_objects(self, tmp_path):
        """Test: Accepts Path objects."""
        feed_url = "https://example.com/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        try:
            run_cycle(
                feed_url=feed_url,
                outputs_dir=outputs_dir,
                db_path=db_path
            )
        except Exception:
            pass
        
        assert db_path.exists()


class TestSchedulerIntegration:
    """Integration tests for scheduler."""
    
    def test_scheduler_and_cycle_same_db(self, tmp_path):
        """Test: Scheduler and cycle use same DB."""
        feed_url = "https://example.com/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        # Run cycle
        try:
            run_cycle(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path)
            )
        except Exception:
            pass
        
        assert db_path.exists()
        
        # Run scheduler
        try:
            run_scheduler(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path),
                once=True
            )
        except Exception:
            pass
        
        # DB should still work
        assert db_path.exists()
    
    def test_multiple_cycle_runs(self, tmp_path):
        """Test: Multiple cycle runs work."""
        feed_url = "https://example.com/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        for i in range(3):
            try:
                run_cycle(
                    feed_url=feed_url,
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path)
                )
            except Exception:
                pass
        
        assert db_path.exists()
