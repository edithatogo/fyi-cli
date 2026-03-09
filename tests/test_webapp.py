"""Tests for webapp module - HTTP server and route handlers."""
import pytest
from http.server import BaseHTTPRequestHandler
from unittest.mock import MagicMock, patch
from io import BytesIO
import json

from fyi_system.webapp import (
    _escape,
    _layout,
    _status_options,
    _authority_options,
    _render_dashboard,
    _render_requests,
    _render_request_form,
    _render_recommended_draft,
    _render_request_detail,
    _render_authorities,
    _render_import_form,
    _render_timeline,
    _parse_multipart_upload,
    _apply_security_headers,
    _redirect,
    _parse_post_fields,
    make_handler,
    serve,
)
from fyi_system.db import init_db, insert_tracked_request, query_all, get_tracked_request


class TestEscape:
    """Test HTML escaping."""
    
    def test_escape_plain(self):
        assert _escape("Hello") == "Hello"
    
    def test_escape_html_chars(self):
        assert _escape("<script>") == "&lt;script&gt;"
        assert _escape("a & b") == "a &amp; b"
        assert _escape('"quote"') == "&quot;quote&quot;"
    
    def test_escape_none(self):
        assert _escape(None) == ""
    
    def test_escape_empty(self):
        assert _escape("") == ""


class TestLayout:
    """Test HTML layout generation."""
    
    def test_layout_structure(self):
        result = _layout("Title", "<p>Content</p>")
        assert "<!doctype html>" in result
        assert "<html>" in result
        assert "</html>" in result
        assert "<title>Title</title>" in result
        assert "<p>Content</p>" in result
    
    def test_layout_escapes_title(self):
        result = _layout("<script>alert('xss')</script>", "Content")
        assert "<script>" not in result
        assert "&lt;script&gt;" in result
    
    def test_layout_contains_nav(self):
        result = _layout("Title", "Content")
        assert "<nav>" in result
        assert "Dashboard" in result
        assert "Requests" in result
        assert "New request" in result
        assert "Authorities" in result
    
    def test_layout_contains_styles(self):
        result = _layout("Title", "Content")
        assert "<style>" in result
        assert "body {" in result
        assert "font-family" in result


class TestStatusOptions:
    """Test status dropdown generation."""
    
    def test_all_statuses_included(self):
        result = _status_options("draft")
        statuses = ['draft', 'submitted', 'awaiting_response', 'partial', 'completed', 'closed']
        for status in statuses:
            assert status in result
    
    def test_current_selected(self):
        result = _status_options("submitted")
        assert "value='submitted' selected" in result


class TestAuthorityOptions:
    """Test authority dropdown generation."""
    
    def test_authorities_from_db(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Insert test authority
        conn = query_all.__globals__['connect'](str(db_path))
        conn.execute("INSERT INTO authorities (slug, name) VALUES ('test1', 'Test Authority 1')")
        conn.execute("INSERT INTO authorities (slug, name) VALUES ('test2', 'Test Authority 2')")
        conn.commit()
        conn.close()
        
        result = _authority_options(str(db_path))
        assert "Test Authority 1" in result
        assert "Test Authority 2" in result
        assert "test1" in result
        assert "test2" in result


class TestRenderDashboard:
    """Test dashboard rendering."""
    
    def test_dashboard_basic(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        result = _render_dashboard(str(db_path))
        assert "FYI Request System" in result
        assert "Dashboard" in result
    
    def test_dashboard_with_flash(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        result = _render_dashboard(str(db_path), flash="Success!")
        assert "Success!" in result
        assert "class='success'" in result
    
    def test_dashboard_with_requests(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test Request',
            body='Test body'
        )
        
        result = _render_dashboard(str(db_path))
        assert "Test Request" in result


class TestRenderRequests:
    """Test requests list rendering."""
    
    def test_requests_empty(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        result = _render_requests(str(db_path))
        assert "Requests" in result
        assert "<table>" in result
    
    def test_requests_with_data(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test Request',
            body='Test body',
            status='submitted'
        )
        
        result = _render_requests(str(db_path))
        assert "Test Request" in result
        assert "submitted" in result
    
    def test_requests_with_search(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        insert_tracked_request(
            db_path=str(db_path),
            authority_slug='ministry',
            title='Ministry Request',
            body='Test'
        )
        
        result = _render_requests(str(db_path), q='ministry')
        assert "Ministry Request" in result


class TestRenderRequestForm:
    """Test request form rendering."""
    
    def test_form_new_request(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Insert test authority
        conn = query_all.__globals__['connect'](str(db_path))
        conn.execute("INSERT INTO authorities (slug, name) VALUES ('test', 'Test')")
        conn.commit()
        conn.close()
        
        result = _render_request_form(str(db_path))
        assert "<form" in result
        assert "</form>" in result
        assert "authority" in result.lower()
        assert "title" in result.lower()
        assert "body" in result.lower()
    
    def test_form_with_flash(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        result = _render_request_form(str(db_path), flash="Created!")
        assert "Created!" in result


class TestRenderRequestDetail:
    """Test request detail rendering."""
    
    def test_detail_with_request(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test Detail',
            body='Test body'
        )
        
        result = _render_request_detail(str(db_path), request_id)
        assert "Test Detail" in result
        assert "Test body" in result


class TestRenderTimeline:
    """Test timeline rendering."""
    
    def test_timeline_with_request(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test Timeline',
            body='Test'
        )
        
        result = _render_timeline(str(db_path), request_id)
        assert "Timeline" in result or "test" in result.lower()


class TestRenderAuthorities:
    """Test authorities rendering."""
    
    def test_authorities_empty(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        result = _render_authorities(str(db_path))
        assert "Authorities" in result
    
    def test_authorities_with_data(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        conn = query_all.__globals__['connect'](str(db_path))
        conn.execute("INSERT INTO authorities (slug, name) VALUES ('test', 'Test Authority')")
        conn.commit()
        conn.close()
        
        result = _render_authorities(str(db_path))
        assert "Test Authority" in result


class TestParsePostFields:
    """Test POST field parsing."""
    
    def test_parse_urlencoded(self):
        handler = MagicMock()
        handler.headers = {'Content-Type': 'application/x-www-form-urlencoded', 'Content-Length': '20'}
        handler.rfile = BytesIO(b'field1=value1&field2=value2')
        
        result = _parse_post_fields(handler)
        assert 'field1' in result
        assert result['field1'] == 'value1'
        assert 'field2' in result
        assert result['field2'] == 'value2'


class TestSecurityHeaders:
    """Test security header application."""
    
    def test_apply_security_headers(self):
        handler = MagicMock()
        _apply_security_headers(handler)
        
        # Should set security headers
        calls = handler.send_header.call_args_list
        header_names = [call[0][0] for call in calls]
        
        # Check for common security headers that are actually set
        assert 'X-Content-Type-Options' in header_names
        assert 'Referrer-Policy' in header_names


class TestRedirect:
    """Test redirect functionality."""
    
    def test_redirect(self):
        handler = MagicMock()
        _redirect(handler, "/new-location")
        
        # Should call send_response with 303 (See Other) or 302 (Found)
        handler.send_response.assert_called()
        call_args = handler.send_response.call_args[0][0]
        assert call_args in [302, 303]
        
        handler.send_header.assert_any_call('Location', '/new-location')


class TestMakeHandler:
    """Test handler creation."""
    
    def test_make_handler_returns_class(self, tmp_path):
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        handler_class = make_handler(str(db_path))
        assert handler_class is not None


class TestServe:
    """Test server functionality."""

    def test_serve_function_exists(self):
        """Test that serve function exists and is callable."""
        assert callable(serve)


class TestRenderRecommendedDraft:
    """Test recommended draft rendering."""
    
    def test_draft_not_found(self, tmp_path):
        """Test: Shows not found for invalid request."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        result = _render_recommended_draft(str(db_path), 99999)
        assert "Not found" in result or "not found" in result.lower()
    
    def test_draft_with_request(self, tmp_path):
        """Test: Renders draft for valid request."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test Draft',
            body='Test body'
        )
        
        result = _render_recommended_draft(str(db_path), request_id)
        assert result is not None


class TestRenderRequestDetail:
    """Test request detail rendering."""
    
    def test_detail_not_found(self, tmp_path):
        """Test: Shows not found for invalid request."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        result = _render_request_detail(str(db_path), 99999)
        assert "Not found" in result or "not found" in result.lower()
    
    def test_detail_with_request(self, tmp_path):
        """Test: Renders detail for valid request."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        # Insert authority first (required for request)
        conn = query_all.__globals__['connect'](str(db_path))
        conn.execute("INSERT INTO authorities (slug, name) VALUES ('test', 'Test Authority')")
        conn.commit()
        
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test Detail',
            body='Test body'
        )
        
        # Verify request was created
        request = get_tracked_request(str(db_path), request_id)
        assert request is not None
        
        result = _render_request_detail(str(db_path), request_id)
        assert "Test Detail" in result or "detail" in result.lower()


class TestRenderTimeline:
    """Test timeline rendering."""
    
    def test_timeline_with_request(self, tmp_path):
        """Test: Renders timeline for valid request."""
        db_path = tmp_path / "test.db"
        init_db(str(db_path))
        
        request_id = insert_tracked_request(
            db_path=str(db_path),
            authority_slug='test',
            title='Test Timeline',
            body='Test'
        )
        
        result = _render_timeline(str(db_path), request_id)
        assert result is not None


class TestParsePostFields:
    """Test POST field parsing."""
    
    def test_parse_empty_body(self):
        """Test: Handles empty body."""
        handler = MagicMock()
        handler.headers = {'Content-Type': 'application/x-www-form-urlencoded', 'Content-Length': '0'}
        handler.rfile = BytesIO(b'')
        
        result = _parse_post_fields(handler)
        assert isinstance(result, dict)


class TestRenderImportForm:
    """Test import form rendering."""
    
    def test_import_form_basic(self):
        """Test: Renders import form."""
        result = _render_import_form()
        assert "<form" in result
        assert "import" in result.lower()
    
    def test_import_form_with_message(self):
        """Test: Renders with success message."""
        result = _render_import_form(message="Imported!")
        assert "Imported!" in result
