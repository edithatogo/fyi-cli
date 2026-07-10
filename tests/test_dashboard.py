"""Tests for dashboard module - HTML dashboard generation."""
import json
import tempfile
from pathlib import Path
import pytest
from fyi_system.dashboard import dashboard_payload, write_dashboard


class TestDashboardPayload:
    """Test dashboard payload generation functionality."""
    
    def test_dashboard_payload_returns_dict(self, tmp_path):
        """Test that dashboard_payload returns a dictionary."""
        db_path = tmp_path / "test.db"
        # Initialize DB first
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        result = dashboard_payload(str(db_path))
        assert isinstance(result, dict)
        assert 'summary' in result
        assert 'action_now' in result
    
    def test_dashboard_payload_summary_keys(self, tmp_path):
        """Test that dashboard summary has required keys."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        result = dashboard_payload(str(db_path))
        summary = result['summary']
        assert 'total' in summary
        assert 'attention' in summary
        assert 'action_now' in summary
        assert 'authorities' in summary
        assert 'recent_updates' in summary
    
    def test_dashboard_payload_with_path_object(self, tmp_path):
        """Test dashboard_payload accepts Path objects."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        result = dashboard_payload(db_path)
        assert isinstance(result, dict)
    
    def test_dashboard_payload_empty_db(self, tmp_path):
        """Test dashboard with empty database."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        result = dashboard_payload(str(db_path))
        assert result['summary']['total'] == 0
        assert result['summary']['attention'] == 0
        assert result['action_now'] == []
    
    def test_dashboard_payload_with_data(self, tmp_path):
        """Test dashboard with sample data."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db, insert_tracked_request
        init_db(str(db_path))
        
        # Insert a test request
        insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test Request',
            body='Test body'
        )
        
        result = dashboard_payload(str(db_path))
        # Total should be at least 1 (may include other test data)
        assert result['summary']['total'] >= 1


class TestWriteDashboard:
    """Test dashboard HTML writing."""
    
    def test_write_dashboard_returns_path(self, tmp_path):
        """Test that write_dashboard returns a Path."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        result = write_dashboard(html_output, str(db_path))
        assert isinstance(result, Path)
        assert result.exists()
    
    def test_write_dashboard_creates_html_file(self, tmp_path):
        """Test that write_dashboard creates HTML file."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        assert html_output.exists()
        content = html_output.read_text()
        assert '<html>' in content
        assert '</html>' in content
    
    def test_write_dashboard_contains_title(self, tmp_path):
        """Test that written dashboard contains title."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        content = html_output.read_text()
        assert 'FYI Request System Dashboard' in content
    
    def test_write_dashboard_contains_summary(self, tmp_path):
        """Test that written dashboard contains summary section."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        content = html_output.read_text()
        assert 'Total tracked' in content
        assert 'Needs attention' in content
    
    def test_write_dashboard_with_path_object(self, tmp_path):
        """Test write_dashboard accepts Path objects."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        result = write_dashboard(html_output, db_path)
        assert isinstance(result, Path)
        content = html_output.read_text().strip()
        assert content.startswith('<!doctype html>')
    
    def test_write_dashboard_contains_table(self, tmp_path):
        """Test that written dashboard contains data table."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        content = html_output.read_text()
        assert '<table>' in content
        assert '</table>' in content
    
    def test_write_dashboard_contains_table_headers(self, tmp_path):
        """Test that table has correct headers."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        content = html_output.read_text()
        assert '<th>ID</th>' in content
        assert '<th>Title</th>' in content
        assert '<th>Status</th>' in content
        assert '<th>State</th>' in content
    
    def test_write_dashboard_contains_cards(self, tmp_path):
        """Test that dashboard contains summary cards."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        content = html_output.read_text()
        assert 'class="card"' in content or 'class=\'card\'' in content
    
    def test_write_dashboard_contains_styles(self, tmp_path):
        """Test that dashboard contains CSS styles."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        content = html_output.read_text()
        assert '<style>' in content
        assert '</style>' in content
    
    def test_write_dashboard_contains_charset(self, tmp_path):
        """Test that dashboard has proper charset."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db
        init_db(str(db_path))
        
        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        content = html_output.read_text()
        assert 'charset' in content
        assert 'utf-8' in content.lower()

    def test_write_dashboard_escapes_untrusted_html(self, tmp_path):
        """Dashboard fields must be HTML-escaped before rendering."""
        db_path = tmp_path / "test.db"
        from fyi_system.db import init_db, insert_tracked_request

        init_db(str(db_path))
        insert_tracked_request(
            db_path=str(db_path),
            authority_slug="<img src=x onerror=alert(1)>",
            title="<script>alert(1)</script>",
            body="test",
        )

        html_output = tmp_path / "dashboard.html"
        write_dashboard(html_output, str(db_path))
        content = html_output.read_text()

        assert "<script>alert(1)</script>" not in content
        assert "&lt;script&gt;alert(1)&lt;/script&gt;" in content
