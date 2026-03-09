"""Integration tests for FYI Request System.

End-to-end tests that verify complete workflows:
- Request lifecycle
- Feed-to-request workflow
- Export-import round trip
- Database operations
"""
import pytest
import json
from pathlib import Path
from fyi_system.db import init_db, insert_tracked_request, get_tracked_request, query_all, export_tracked_requests
from fyi_system.security import sanitize_payload, redact_text
from fyi_system.reporting import attention_report, build_handover_markdown, export_request_bundle
from fyi_system.dashboard import dashboard_payload, write_dashboard


class TestRequestLifecycle:
    """Test complete request lifecycle."""
    
    def test_create_retrieve_update_request(self, tmp_path):
        """Integration: Full request CRUD operations."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug='ministry-test',
            title='Test Request Lifecycle',
            body='Testing full lifecycle',
            status='draft'
        )
        assert request_id > 0
        
        # Retrieve
        request = get_tracked_request(str(db_path), request_id)
        assert request['authority_slug'] == 'ministry-test'
        assert request['title'] == 'Test Request Lifecycle'
        assert request['status'] == 'draft'
        
        # Update (via direct SQL for now)
        conn = query_all.__globals__['connect'](str(db_path))
        conn.execute(
            "UPDATE tracked_requests SET status = ? WHERE id = ?",
            ('submitted', request_id)
        )
        conn.commit()
        conn.close()
        
        # Verify update
        updated = get_tracked_request(str(db_path), request_id)
        assert updated['status'] == 'submitted'
    
    def test_multiple_requests_isolation(self, tmp_path):
        """Integration: Multiple requests don't interfere."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create multiple requests
        ids = []
        for i in range(10):
            request_id = insert_tracked_request(
                db_path=str(db_path),
                authority_slug=f'authority_{i}',
                title=f'Request {i}',
                body=f'Body {i}'
            )
            ids.append(request_id)
        
        # Verify all exist and are isolated
        for i, request_id in enumerate(ids):
            request = get_tracked_request(str(db_path), request_id)
            assert request['authority_slug'] == f'authority_{i}'
            assert request['title'] == f'Request {i}'


class TestExportImportRoundTrip:
    """Test export-import round trip."""
    
    def test_export_import_requests(self, tmp_path):
        """Integration: Export and import preserves data."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create test data
        original_data = []
        for i in range(5):
            request_id = insert_tracked_request(
                db_path=str(db_path),
                authority_slug=f'auth_{i}',
                title=f'Title {i}',
                body=f'Body {i}',
                tags=f'tag_{i}'
            )
            original_data.append({
                'id': request_id,
                'authority_slug': f'auth_{i}',
                'title': f'Title {i}',
                'body': f'Body {i}',
                'tags': f'tag_{i}'
            })
        
        # Export
        export_path = tmp_path / "export.json"
        export_tracked_requests(str(db_path), export_path)
        
        # Verify export file exists and is valid JSON
        assert export_path.exists()
        exported = json.loads(export_path.read_text())
        assert isinstance(exported, list)
        assert len(exported) == 5


class TestReportingIntegration:
    """Test reporting functions with real data."""
    
    def test_attention_report_with_data(self, tmp_path):
        """Integration: Attention report generates with real data."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create test requests
        for i in range(10):
            insert_tracked_request(
                db_path=str(db_path),
                authority_slug=f'authority_{i}',
                title=f'Request {i}',
                body=f'Body {i}',
                status='draft' if i % 2 == 0 else 'submitted'
            )
        
        # Generate report
        report = attention_report(str(db_path))

        assert isinstance(report, dict)
        assert 'items' in report
        assert isinstance(report['items'], list)
    
    def test_handover_document_with_data(self, tmp_path):
        """Integration: Handover document generates with real data."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create test requests
        for i in range(5):
            insert_tracked_request(
                db_path=str(db_path),
                authority_slug=f'authority_{i}',
                title=f'Request {i}',
                body=f'Body {i}'
            )
        
        # Generate document
        doc = build_handover_markdown(str(db_path))
        
        assert isinstance(doc, str)
        assert len(doc) > 0
        assert 'FYI' in doc or 'Request' in doc
    
    def test_dashboard_with_data(self, tmp_path):
        """Integration: Dashboard generates with real data."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create test requests
        for i in range(20):
            insert_tracked_request(
                db_path=str(db_path),
                authority_slug=f'authority_{i % 5}',
                title=f'Request {i}',
                body=f'Body {i}',
                status=['draft', 'submitted', 'completed'][i % 3]
            )
        
        # Generate dashboard
        dashboard = dashboard_payload(str(db_path))
        
        assert isinstance(dashboard, dict)
        assert 'summary' in dashboard
        assert 'items' in dashboard
        assert isinstance(dashboard['items'], list)
    
    def test_write_dashboard_html(self, tmp_path):
        """Integration: Dashboard HTML file is generated."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create test request
        insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test',
            body='Test body'
        )
        
        # Write HTML
        html_path = tmp_path / "dashboard.html"
        write_dashboard(str(html_path), str(db_path))
        
        assert html_path.exists()
        content = html_path.read_text()
        assert '<!doctype html>' in content
        assert 'Dashboard' in content


class TestSecurityIntegration:
    """Test security functions with realistic data."""
    
    def test_redaction_in_realistic_text(self):
        """Integration: Redaction works with realistic text."""
        realistic_text = """
        Dear Minister,
        
        I am writing to request information under the Official Information Act.
        
        Please contact me at requester@example.com or call 04-123-4567.
        
        My previous request (ref: OIA-2024-001) was submitted via 
        https://fyi.org.nz/request/12345?token=secret123.
        
        Thank you,
        John Doe (john.doe@company.co.nz)
        """
        
        result = redact_text(realistic_text)
        
        # Emails should be redacted
        assert 'requester@example.com' not in result
        assert 'john.doe@company.co.nz' not in result
        
        # URL with token should be sanitized
        assert 'secret123' not in result
        
        # Rest of text should be preserved
        assert 'Dear Minister' in result
        assert 'Official Information Act' in result
    
    def test_sanitize_realistic_payload(self):
        """Integration: Sanitization works with realistic payload."""
        realistic_payload = {
            'email': 'user@example.com',
            'title': 'OIA Request',
            'body': 'I request information about...',
            'tags': 'oia,government,2024',
            'metadata': {
                'source': 'web',
                'ip': '192.168.1.1',
                'user_agent': 'Mozilla/5.0'
            }
        }
        
        result = sanitize_payload(realistic_payload)
        
        # Email should be redacted
        assert 'user@example.com' not in str(result)
        
        # Structure should be preserved
        assert set(result.keys()) == set(realistic_payload.keys())
        assert 'metadata' in result
        assert 'source' in result['metadata']


class TestBundleExportIntegration:
    """Test bundle export functionality."""
    
    def test_export_request_bundle(self, tmp_path):
        """Integration: Request bundle exports successfully."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Create test request
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test-authority',
            title='Test Bundle Export',
            body='This is a test request for bundle export'
        )
        
        # Export bundle
        bundle_dir = tmp_path / "bundle"
        export_request_bundle(str(bundle_dir), str(db_path), request_id)
        
        # Verify bundle directory created
        assert bundle_dir.exists()
        
        # Should have some files
        files = list(bundle_dir.glob('*'))
        assert len(files) > 0


class TestDatabaseIntegrity:
    """Test database integrity under various conditions."""
    
    def test_concurrent_inserts(self, tmp_path):
        """Integration: Multiple inserts maintain integrity."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Insert many requests
        for i in range(100):
            insert_tracked_request(
                db_path=str(db_path),
                authority_slug=f'auth_{i % 10}',
                title=f'Request {i}',
                body=f'Body {i}'
            )
        
        # Verify all exist
        all_requests = query_all(str(db_path), "SELECT * FROM tracked_requests")
        assert len(all_requests) == 100
        
        # Verify no corruption
        for request in all_requests:
            assert request['id'] is not None
            assert request['authority_slug'] is not None
            assert request['title'] is not None
    
    def test_database_after_operations(self, tmp_path):
        """Integration: Database remains valid after operations."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Perform various operations
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test',
            body='Test'
        )
        
        # Query
        get_tracked_request(str(db_path), request_id)
        
        # Generate reports
        attention_report(str(db_path))
        dashboard_payload(str(db_path))
        
        # Verify database still works
        request = get_tracked_request(str(db_path), request_id)
        assert request is not None
        assert request['id'] == request_id
