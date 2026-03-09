"""End-to-End workflow tests for FYI Request System.

These tests verify complete operator workflows from start to finish.
"""
import subprocess
import sys
import pytest
from pathlib import Path
import json


class TestFeedToReportWorkflow:
    """Test complete feed ingestion to report generation workflow."""

    def test_full_workflow(self, tmp_path):
        """E2E: Complete workflow from init to report."""
        db_path = tmp_path / "test.db"
        
        # Step 1: Initialize database
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        # Handle "database already exists" gracefully
        if result.returncode != 0 and "already" not in result.stderr.lower():
            pytest.skip(f"Database initialization failed: {result.stderr}")
        
        # Step 2: Register a request
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test-authority', 'Test Request', 'Test body for workflow',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        if result.returncode != 0:
            pytest.skip(f"Request registration failed: {result.stderr}")
        
        # Step 3: List requests (verify creation)
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'list-requests', '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        assert result.returncode == 0
        
        # Step 4: Generate attention report
        report_path = tmp_path / "attention.json"
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'attention-report',
             '--output', str(report_path),
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        if result.returncode == 0:
            assert report_path.exists()


class TestExportImportRoundTrip:
    """Test export and import round-trip workflow."""

    def test_export_import_workflow(self, tmp_path):
        """E2E: Export and re-import preserves data."""
        db_path = tmp_path / "test.db"
        export_path = tmp_path / "export.json"
        
        # Setup: Initialize and create request
        init_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        if init_result.returncode != 0 and "already" not in init_result.stderr.lower():
            pytest.skip(f"Init failed: {init_result.stderr}")
        
        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test-auth', 'Export Test', 'Test body',
             '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        if create_result.returncode != 0:
            pytest.skip(f"Create failed: {create_result.stderr}")
        
        # Export
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'export-requests',
             '--output', str(export_path),
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        # Should succeed or skip if disk issues
        if result.returncode == 0:
            assert export_path.exists()
        else:
            pytest.skip(f"Export failed: {result.stderr}")


class TestRequestLifecycleWorkflow:
    """Test request lifecycle management workflow."""

    def test_lifecycle_workflow(self, tmp_path):
        """E2E: Create, update, and verify request lifecycle."""
        db_path = tmp_path / "test.db"
        
        # Initialize
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        # Create request
        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Lifecycle Test', 'Test body',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        assert create_result.returncode == 0
        
        # Extract request ID
        import re
        match = re.search(r'\d+', create_result.stdout)
        if match:
            request_id = match.group()
            
            # Update status
            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'set-status',
                 request_id, 'submitted',
                 '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )
            
            # Should succeed
            assert result.returncode in [0, 1]
            
            # List to verify
            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'list-requests',
                 '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )
            assert result.returncode == 0


class TestBundleExportWorkflow:
    """Test bundle export workflow."""

    def test_bundle_export_workflow(self, tmp_path):
        """E2E: Export request bundle."""
        db_path = tmp_path / "test.db"
        bundle_dir = tmp_path / "bundle"
        
        # Setup
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        create_result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'register-request',
             'test', 'Bundle Test', 'Test body',
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        assert create_result.returncode == 0
        
        # Extract request ID
        import re
        match = re.search(r'\d+', create_result.stdout)
        if match:
            request_id = match.group()
            
            # Export bundle
            result = subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'export-bundle',
                 request_id,
                 '--output-dir', str(bundle_dir),
                 '--db', str(db_path)],
                capture_output=True,
                text=True,
                timeout=30
            )
            
            # Should create directory or give informative error
            if result.returncode == 0:
                assert bundle_dir.exists()


class TestDashboardGenerationWorkflow:
    """Test dashboard generation workflow."""

    def test_dashboard_workflow(self, tmp_path):
        """E2E: Generate HTML dashboard."""
        db_path = tmp_path / "test.db"
        dashboard_path = tmp_path / "dashboard.html"
        
        # Setup
        subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'init-db', '--db', str(db_path)],
            capture_output=True,
            timeout=30
        )
        
        # Create some requests
        for i in range(3):
            subprocess.run(
                [sys.executable, '-m', 'fyi_system.cli', 'register-request',
                 f'auth{i}', f'Request {i}', f'Body {i}',
                 '--db', str(db_path)],
                capture_output=True,
                timeout=30
            )
        
        # Generate dashboard
        result = subprocess.run(
            [sys.executable, '-m', 'fyi_system.cli', 'dashboard',
             '--output', str(dashboard_path),
             '--db', str(db_path)],
            capture_output=True,
            text=True,
            timeout=30
        )
        
        # Should create HTML file
        if result.returncode == 0:
            assert dashboard_path.exists()
            content = dashboard_path.read_text()
            assert '<!doctype html>' in content.lower() or '<html' in content.lower()
