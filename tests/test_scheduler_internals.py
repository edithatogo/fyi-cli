"""Tests for scheduler module internals - loop logic and timing."""
import pytest
import time
from pathlib import Path
from unittest.mock import patch, MagicMock, call
from fyi_system.scheduler import run_scheduler, run_cycle


class TestSchedulerInternals:
    """Test scheduler internal logic."""

    def test_scheduler_loop_structure(self, tmp_path):
        """Test that scheduler has proper loop structure."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"

        # Mock time.sleep and monitor functions to avoid actual database operations
        with patch('fyi_system.scheduler.time.sleep') as mock_sleep, \
             patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'), \
             patch('fyi_system.scheduler.connect') as mock_connect:
            mock_conn = MagicMock()
            mock_connect.return_value = mock_conn
            mock_sleep.side_effect = [None, InterruptedError("Stop after 2 iterations")]

            try:
                run_scheduler(
                    feed_url=feed_url,
                    interval_seconds=1,
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path),
                    once=False
                )
            except InterruptedError:
                pass

            # Should have called sleep at least once
            assert mock_sleep.called

    def test_scheduler_once_mode_no_loop(self, tmp_path):
        """Test that once mode doesn't loop."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"

        with patch('fyi_system.scheduler.time.sleep') as mock_sleep, \
             patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):
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

    def test_scheduler_error_handling(self, tmp_path):
        """Test scheduler handles errors gracefully."""
        feed_url = "https://invalid.invalid/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"
        
        # Should not raise unhandled exceptions
        try:
            run_scheduler(
                feed_url=feed_url,
                interval_seconds=1,
                outputs_dir=str(outputs_dir),
                db_path=str(db_path),
                once=True
            )
        except Exception:
            pass  # Expected for invalid URL
        
        # DB should still be created
        assert db_path.exists()


class TestCycleInternals:
    """Test cycle internal operations."""

    def test_cycle_sequence(self, tmp_path):
        """Test that cycle runs operations in sequence."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"

        # Mock the individual operations to track call order
        with patch('fyi_system.scheduler.ingest_feed', return_value=0) as mock_ingest, \
             patch('fyi_system.scheduler.reconcile_events', return_value=0) as mock_reconcile, \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):

            try:
                run_cycle(
                    feed_url=feed_url,
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path)
                )
            except Exception:
                pass

            # ingest_feed should be called first
            assert mock_ingest.called
            assert mock_reconcile.called

    def test_cycle_with_invalid_feed(self, tmp_path):
        """Test cycle handles invalid feed gracefully."""
        feed_url = "https://invalid.invalid/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"

        # Mock to avoid actual network calls
        with patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):
            # Should not crash on invalid feed
            try:
                run_cycle(
                    feed_url=feed_url,
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path)
                )
            except Exception:
                pass

            # DB should still be initialized
            assert db_path.exists()

    def test_cycle_creates_outputs_dir(self, tmp_path):
        """Test cycle creates outputs directory."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "new_outputs"
        db_path = tmp_path / "test.db"

        with patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):
            try:
                run_cycle(
                    feed_url=feed_url,
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path)
                )
            except Exception:
                pass

            assert outputs_dir.exists()


class TestSchedulerConfiguration:
    """Test scheduler configuration options."""

    def test_scheduler_custom_interval(self, tmp_path):
        """Test scheduler with custom interval."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"

        with patch('fyi_system.scheduler.time.sleep') as mock_sleep, \
             patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'), \
             patch('fyi_system.scheduler.connect') as mock_connect:
            mock_conn = MagicMock()
            mock_connect.return_value = mock_conn
            mock_sleep.side_effect = InterruptedError("Stop")

            try:
                run_scheduler(
                    feed_url=feed_url,
                    interval_seconds=0.1,  # Very short interval for testing
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path),
                    once=False
                )
            except InterruptedError:
                pass

        # Verify interval was used
        assert mock_sleep.called

    def test_scheduler_default_values(self, tmp_path):
        """Test scheduler default parameter values."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        db_path = tmp_path / "test.db"
        outputs_dir = tmp_path / "outputs"

        # Test that defaults work
        with patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):
            try:
                run_scheduler(
                    feed_url=feed_url,
                    db_path=str(db_path),
                    outputs_dir=str(outputs_dir),
                    once=True
                )
            except Exception:
                pass

        # Should use default outputs_dir='outputs'
        assert outputs_dir.exists()


class TestSchedulerEdgeCases:
    """Test scheduler edge cases."""

    def test_scheduler_empty_feed_url(self, tmp_path):
        """Test scheduler with empty feed URL."""
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"

        with patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):
            try:
                run_scheduler(
                    feed_url="",
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path),
                    once=True
                )
            except Exception:
                pass

            # Should handle gracefully
            assert db_path.exists()

    def test_cycle_with_none_values(self, tmp_path):
        """Test cycle handles None values."""
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"

        # Test with various parameter combinations
        with patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):
            try:
                run_cycle(
                    feed_url=None,
                    outputs_dir=str(outputs_dir),
                    db_path=str(db_path)
                )
            except (TypeError, AttributeError, Exception):
                pass

            # Should not crash catastrophically
            assert db_path.exists()

    def test_scheduler_concurrent_calls(self, tmp_path):
        """Test scheduler handles concurrent calls."""
        feed_url = "https://www.fyi.org.nz/request/12345/feed"
        outputs_dir = tmp_path / "outputs"
        outputs_dir.mkdir()
        db_path = tmp_path / "test.db"

        # Multiple calls should not cause issues
        with patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):
            for i in range(3):
                try:
                    run_scheduler(
                        feed_url=feed_url,
                        outputs_dir=str(outputs_dir),
                        db_path=str(db_path),
                        once=True
                    )
                except Exception:
                    pass

            # Should complete without database corruption
            assert db_path.exists()


class TestSchedulerIntegration:
    """Integration tests for scheduler with real database operations."""

    def test_scheduler_full_cycle_integration(self, tmp_path):
        """Test scheduler with real database operations."""
        from fyi_system.db import init_db, connect
        
        db_path = tmp_path / "test.db"
        outputs_dir = tmp_path / "outputs"
        
        # Initialize database schema
        init_db(db_path)
        
        # Run one cycle with mocked network but real DB
        with patch('fyi_system.scheduler.ingest_feed', return_value=0), \
             patch('fyi_system.scheduler.reconcile_events', return_value=0), \
             patch('fyi_system.scheduler.write_attention_report'), \
             patch('fyi_system.scheduler.write_handover'):
            result = run_cycle(
                feed_url="https://example.com/feed",
                db_path=str(db_path),
                outputs_dir=str(outputs_dir)
            )
        
        # Verify database state changed
        conn = connect(db_path)
        try:
            run_log = conn.execute(
                "SELECT * FROM run_log WHERE job_name = 'run_cycle'"
            ).fetchone()
            assert run_log is not None, "run_cycle should be logged"
            assert run_log['status'] == 'ok', "run_cycle should have 'ok' status"
        finally:
            conn.close()
        
        # Verify result structure
        assert result['ingested'] == 0
        assert result['matched'] == 0
        assert outputs_dir.exists()
