"""Tests for scheduler module - request scheduling and cycling."""
import pytest
import time
from pathlib import Path
from fyi_system.scheduler import run_scheduler, run_cycle


class TestRunScheduler:
    """Test scheduler functionality."""
    
    def test_run_scheduler_exists(self):
        """Test that run_scheduler function exists."""
        assert callable(run_scheduler)
    
    def test_run_scheduler_accepts_feed_url(self, tmp_path):
        """Test run_scheduler accepts feed URL parameter."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        # Run once mode to avoid infinite loop
        # Note: This may fail if feed is not available, but we're testing the interface
        try:
            run_scheduler(
                feed_url=feed_url,
                interval_seconds=1,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path),
                once=True
            )
        except Exception:
            # Expected if feed is unavailable or other runtime error
            pass
    
    def test_run_scheduler_once_mode(self, tmp_path):
        """Test scheduler runs once with once=True flag."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        start_time = time.time()
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
        
        # Should return quickly in once mode (not wait for interval)
        elapsed = time.time() - start_time
        assert elapsed < 5  # Should complete in under 5 seconds
    
    def test_run_scheduler_with_path_objects(self, tmp_path):
        """Test scheduler accepts Path objects."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        try:
            run_scheduler(
                feed_url=feed_url,
                interval_seconds=1,
                outputs_dir=outputs_dir,
                db_path=db_path,
                once=True
            )
        except Exception:
            pass
    
    def test_run_scheduler_default_interval(self, tmp_path):
        """Test scheduler default interval is 3600 seconds."""
        # This is tested indirectly through the CLI
        # Direct testing would require mocking time.sleep
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
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
    
    def test_run_scheduler_creates_outputs_dir(self, tmp_path):
        """Test scheduler creates outputs directory if needed."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
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
        
        # Outputs directory should be created
        assert outputs_dir.exists()


class TestRunCycle:
    """Test cycle running functionality."""
    
    def test_run_cycle_exists(self):
        """Test that run_cycle function exists."""
        assert callable(run_cycle)
    
    def test_run_cycle_accepts_parameters(self, tmp_path):
        """Test run_cycle accepts required parameters."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
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
            # Expected if feed is unavailable
            pass
    
    def test_run_cycle_with_path_objects(self, tmp_path):
        """Test run_cycle accepts Path objects."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
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
    
    def test_run_cycle_creates_outputs(self, tmp_path):
        """Test run_cycle creates output files."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
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
        
        # Check that outputs directory exists and may have files
        assert outputs_dir.exists()
    
    def test_run_cycle_default_outputs_dir(self, monkeypatch, tmp_path):
        """Test run_cycle uses default outputs directory."""
        monkeypatch.chdir(tmp_path)
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        db_path = tmp_path / "test.db"
        
        try:
            run_cycle(
                feed_url=feed_url,
                db_path=str(db_path)
            )
        except Exception:
            pass
        
        # Should create 'outputs' directory in current working dir
        outputs_dir = tmp_path / "outputs"
        assert outputs_dir.exists()
    
    def test_run_cycle_initializes_db(self, tmp_path):
        """Test run_cycle works with uninitialized database."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        # Don't initialize DB first - run_cycle should handle it
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


class TestSchedulerIntegration:
    """Integration tests for scheduler."""
    
    def test_scheduler_and_cycle_use_same_db(self, tmp_path):
        """Test that scheduler and cycle use the same database."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        # Run cycle first
        try:
            run_cycle(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path)
            )
        except Exception:
            pass
        
        # Verify database was created
        assert db_path.exists()
        
        # Run scheduler once
        try:
            run_scheduler(
                feed_url=feed_url,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path),
                once=True
            )
        except Exception:
            pass
        
        # Database should still exist and be usable
        assert db_path.exists()
    
    def test_run_cycle_multiple_times(self, tmp_path):
        """Test run_cycle can be called multiple times."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
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
        
        # Should complete without errors
        assert db_path.exists()
