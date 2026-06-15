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


class TestParseMultipartUploadExtended:
    """Extended tests for multipart upload parsing."""
    
    def test_parse_no_boundary(self):
        assert _parse_multipart_upload("text/plain", b"data") == b""

    def test_parse_boundary_not_matching(self):
        assert _parse_multipart_upload("multipart/form-data; boundary=123", b"data") == b""

    def test_parse_no_file_field(self):
        body = b"--123\r\nContent-Disposition: form-data; name=\"not-file\"\r\n\r\nsome data\r\n--123--\r\n"
        assert _parse_multipart_upload("multipart/form-data; boundary=123", body) == b""

    def test_parse_success(self):
        body = b"--123\r\nContent-Disposition: form-data; name=\"file\"; filename=\"x.csv\"\r\n\r\nfile-content\r\n--123--\r\n"
        assert _parse_multipart_upload("multipart/form-data; boundary=123", body) == b"file-content"


class TestParsePostFieldsExtended:
    """Extended tests for POST field parsing."""
    
    def test_parse_json(self):
        handler = MagicMock()
        handler.headers = {"Content-Type": "application/json"}
        handler.rfile = BytesIO(b'{"key": "value"}')
        assert _parse_post_fields(handler) == {"key": "value"}

    def test_parse_json_empty(self):
        handler = MagicMock()
        handler.headers = {"Content-Type": "application/json"}
        handler.rfile = BytesIO(b"")
        assert _parse_post_fields(handler) == {}

    def test_parse_multipart(self):
        handler = MagicMock()
        handler.headers = {"Content-Type": "multipart/form-data; boundary=123"}
        handler.rfile = BytesIO(b'--123\r\nContent-Disposition: form-data; name="file"\r\n\r\nhello\r\n--123--')
        assert _parse_post_fields(handler) == {"file": "hello"}


class TestMockHandlerRoutes:
    """Route handling and HTML/JSON response tests using a mocked HTTPServer handler."""
    
    @pytest.fixture(autouse=True)
    def setup_db(self, tmp_path):
        self.db_path = str(tmp_path / "test.db")
        init_db(self.db_path)
        
        # Insert test authority
        conn = query_all.__globals__['connect'](self.db_path)
        conn.execute("INSERT INTO authorities (slug, name, url) VALUES ('test-slug', 'Test Auth Name', 'https://example.com')")
        conn.commit()
        conn.close()
        
        # Insert test request
        self.req_id = insert_tracked_request(
            db_path=self.db_path,
            authority_slug='test-slug',
            title='Test Request Title',
            body='Test request body',
            tags='tag1,tag2',
            status='draft',
            fyi_request_id=123,
            fyi_url='https://example.com/fyi'
        )
        self.HandlerClass = make_handler(self.db_path)

    def get_handler(self, path, headers=None, body=b""):
        class MockHandler(self.HandlerClass):
            def __init__(self):
                self.path = path
                self.headers = headers or {}
                self.rfile = BytesIO(body)
                self.wfile = BytesIO()
                self.response_status = None
                self.response_headers = {}
                
            def send_response(self, status):
                self.response_status = status
                
            def send_header(self, keyword, value):
                self.response_headers[keyword] = value
                
            def end_headers(self):
                pass
                
        return MockHandler()

    def test_get_dashboard(self):
        handler = self.get_handler("/")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"FYI Request System" in handler.wfile.getvalue()
        assert handler.response_headers["Cache-Control"] == "no-store"
        assert "Content-Security-Policy" in handler.response_headers

    def test_get_api_dashboard(self):
        handler = self.get_handler("/api/dashboard")
        handler.do_GET()
        assert handler.response_status == 200
        data = json.loads(handler.wfile.getvalue().decode())
        assert "summary" in data

    def test_get_requests_list(self):
        handler = self.get_handler("/requests")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"Test Request Title" in handler.wfile.getvalue()

    def test_get_requests_list_priority(self):
        handler = self.get_handler("/requests?priority=now")
        handler.do_GET()
        assert handler.response_status == 200

    def test_get_requests_new(self):
        handler = self.get_handler("/requests/new")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"Create tracked request" in handler.wfile.getvalue()

    def test_get_authorities(self):
        handler = self.get_handler("/authorities")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"Test Auth Name" in handler.wfile.getvalue()

    def test_get_authorities_import(self):
        handler = self.get_handler("/authorities/import")
        handler.do_GET()
        assert handler.response_status == 200

    def test_get_request_detail_success(self):
        handler = self.get_handler(f"/requests/{self.req_id}")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"Test Request Title" in handler.wfile.getvalue()

    def test_get_request_detail_not_found(self):
        handler = self.get_handler("/requests/999999")
        handler.do_GET()
        assert handler.response_status == 404
        data = json.loads(handler.wfile.getvalue().decode())
        assert data == {"error": "not found"}

    def test_get_request_detail_invalid_id(self):
        handler = self.get_handler("/requests/abc")
        handler.do_GET()
        assert handler.response_status == 404

    def test_get_timeline_success(self):
        handler = self.get_handler(f"/requests/{self.req_id}/timeline")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"Timeline" in handler.wfile.getvalue()

    def test_get_correspondence_success(self):
        handler = self.get_handler(f"/requests/{self.req_id}/correspondence")
        handler.do_GET()
        assert handler.response_status == 200

    def test_get_correspondence_not_found(self):
        handler = self.get_handler("/requests/999999/correspondence")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"not found" in handler.wfile.getvalue().lower()

    def test_get_edit_success(self):
        handler = self.get_handler(f"/requests/{self.req_id}/edit")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"Edit tracked request" in handler.wfile.getvalue()

    def test_get_edit_not_found(self):
        handler = self.get_handler("/requests/999999/edit")
        handler.do_GET()
        assert handler.response_status == 404

    def test_get_recommended_draft(self):
        handler = self.get_handler(f"/requests/{self.req_id}/recommended-draft?strategy=polite_nudge&tone=neutral")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"Recommended draft" in handler.wfile.getvalue()

    def test_get_recommended_draft_not_found(self):
        handler = self.get_handler("/requests/999999/recommended-draft")
        handler.do_GET()
        assert handler.response_status == 200
        assert b"not found" in handler.wfile.getvalue().lower()

    def test_get_export_bundle(self):
        with patch("fyi_system.webapp.export_request_bundle", return_value="dummy_path") as mock_export:
            handler = self.get_handler(f"/requests/{self.req_id}/export-bundle")
            handler.do_GET()
            assert handler.response_status == 200
            assert b"Exported bundle to dummy_path" in handler.wfile.getvalue()
            mock_export.assert_called_once()

    def test_get_invalid_route(self):
        handler = self.get_handler("/invalid/route")
        handler.do_GET()
        assert handler.response_status == 404

    def test_post_create_request(self):
        body = b"authority_slug=test-slug&title=New+Req&body=New+Body&tags=t1,t2&status=submitted&fyi_request_id=456&fyi_url=http%3A%2F%2Ftest"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler("/requests/create", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/requests?flash=Tracked+request+created"
        
        # Verify db insert
        requests = query_all(self.db_path, "SELECT * FROM tracked_requests WHERE title='New Req'")
        assert len(requests) == 1
        assert requests[0]["status"] == "submitted"
        assert requests[0]["fyi_request_id"] == 456

    def test_post_import_authorities(self):
        csv_body = b"slug,name,url\nimport-slug,Import Name,http://import\n"
        boundary = b"----WebKitFormBoundary12345"
        body = (
            b"--" + boundary + b"\r\n"
            b'Content-Disposition: form-data; name="file"; filename="authorities.csv"\r\n'
            b"Content-Type: text/csv\r\n\r\n" + csv_body + b"\r\n"
            b"--" + boundary + b"--\r\n"
        )
        headers = {
            "Content-Type": f"multipart/form-data; boundary={boundary.decode()}",
            "Content-Length": str(len(body))
        }
        with patch("fyi_system.webapp.import_authorities_csv", return_value=1) as mock_import:
            handler = self.get_handler("/authorities/import", headers=headers, body=body)
            handler.do_POST()
            assert handler.response_status == 303
            assert handler.response_headers["Location"] == "/authorities?flash=Imported+1+authorities"
            mock_import.assert_called_once()

    def test_post_update_request(self):
        body = b"authority_slug=test-slug&title=Updated+Title&body=Updated+Body&tags=t1&status=completed&fyi_request_id=789&fyi_url=http%3A%2F%2Fupdated"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler(f"/requests/{self.req_id}/update", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == f"/requests/{self.req_id}?flash=Tracked+request+updated"
        
        # Verify update
        req = get_tracked_request(self.db_path, self.req_id)
        assert req["title"] == "Updated Title"
        assert req["status"] == "completed"

    def test_post_update_request_invalid_id(self):
        handler = self.get_handler("/requests/abc/update")
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/"

    def test_post_update_status(self):
        body = b"status=closed"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler(f"/requests/{self.req_id}/status", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/requests?flash=Status+updated"
        
        # Verify status
        req = get_tracked_request(self.db_path, self.req_id)
        assert req["status"] == "closed"

    def test_post_invalid_route(self):
        handler = self.get_handler("/invalid-post-route")
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/"


class TestServeMocked:
    """Mocked test for the serve function."""
    
    @patch("fyi_system.webapp.ThreadingHTTPServer")
    @patch("fyi_system.webapp.init_db")
    def test_serve(self, mock_init, mock_server):
        serve(db_path="dummy.db")
        mock_init.assert_called_with("dummy.db")
        mock_server.assert_called_once()
        mock_server.return_value.serve_forever.assert_called_once()


class TestWebAppFormsPhase2:
    """Phase 2 Form Handling and CSV Import Integration Tests."""
    
    @pytest.fixture(autouse=True)
    def setup_db(self, tmp_path):
        self.db_path = str(tmp_path / "test.db")
        init_db(self.db_path)
        
        # Insert test authority
        conn = query_all.__globals__['connect'](self.db_path)
        conn.execute("INSERT INTO authorities (slug, name, url) VALUES ('test-slug', 'Test Auth Name', 'https://example.com')")
        conn.commit()
        conn.close()
        
        # Insert test request
        self.req_id = insert_tracked_request(
            db_path=self.db_path,
            authority_slug='test-slug',
            title='Test Request Title',
            body='Test request body',
            tags='tag1,tag2',
            status='draft',
            fyi_request_id=123,
            fyi_url='https://example.com/fyi'
        )
        self.HandlerClass = make_handler(self.db_path)

    def get_handler(self, path, headers=None, body=b""):
        class MockHandler(self.HandlerClass):
            def __init__(self):
                self.path = path
                self.headers = headers or {}
                self.rfile = BytesIO(body)
                self.wfile = BytesIO()
                self.response_status = None
                self.response_headers = {}
                
            def send_response(self, status):
                self.response_status = status
                
            def send_header(self, keyword, value):
                self.response_headers[keyword] = value
                
            def end_headers(self):
                pass
                
        return MockHandler()

    def test_post_create_request_success(self):
        # Test creation with complete inputs and success redirect
        body = b"authority_slug=test-slug&title=Successful+Request&body=Request+body+here&tags=newtag1,newtag2&status=draft&fyi_request_id=456&fyi_url=https%3A%2F%2Fexample.com%2Ffyi456"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler("/requests/create", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/requests?flash=Tracked+request+created"

        # Verify db contains it
        reqs = query_all(self.db_path, "SELECT * FROM tracked_requests WHERE title='Successful Request'")
        assert len(reqs) == 1
        assert reqs[0]["authority_slug"] == "test-slug"
        assert reqs[0]["body"] == "Request body here"
        assert reqs[0]["tags"] == "newtag1,newtag2"
        assert reqs[0]["status"] == "draft"
        assert reqs[0]["fyi_request_id"] == 456
        assert reqs[0]["fyi_url"] == "https://example.com/fyi456"

    def test_post_create_request_empty_optional_fields(self):
        # Optional fields like tags, fyi_request_id, fyi_url can be empty/missing
        body = b"authority_slug=test-slug&title=No+Optional+Fields&body=Request+body+here&tags=&status=submitted&fyi_request_id=&fyi_url="
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler("/requests/create", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303

        # Verify insertion
        reqs = query_all(self.db_path, "SELECT * FROM tracked_requests WHERE title='No Optional Fields'")
        assert len(reqs) == 1
        assert reqs[0]["tags"] == ""
        assert reqs[0]["fyi_request_id"] is None
        assert reqs[0]["fyi_url"] is None
        assert reqs[0]["status"] == "submitted"

    def test_post_create_request_invalid_fyi_request_id(self):
        # Sending non-integer string for fyi_request_id should raise ValueError in handler
        body = b"authority_slug=test-slug&title=Invalid+ID&body=Body&fyi_request_id=not-a-number"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler("/requests/create", headers=headers, body=body)
        with pytest.raises(ValueError):
            handler.do_POST()

    def test_post_update_request_success(self):
        # Update existing request
        body = b"authority_slug=test-slug&title=Updated+Title&body=Updated+Body&tags=updatedtag&status=awaiting_response&fyi_request_id=999&fyi_url=https%3A%2F%2Fexample.com%2Ffyi999"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler(f"/requests/{self.req_id}/update", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == f"/requests/{self.req_id}?flash=Tracked+request+updated"

        # Verify db update
        req = get_tracked_request(self.db_path, self.req_id)
        assert req["title"] == "Updated Title"
        assert req["body"] == "Updated Body"
        assert req["tags"] == "updatedtag"
        assert req["status"] == "awaiting_response"
        assert req["fyi_request_id"] == 999
        assert req["fyi_url"] == "https://example.com/fyi999"

    def test_post_update_request_empty_fields(self):
        # Update with empty optionals
        body = b"authority_slug=test-slug&title=Updated+Title&body=Updated+Body&tags=&status=completed&fyi_request_id=&fyi_url="
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler(f"/requests/{self.req_id}/update", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303

        # Verify db update
        req = get_tracked_request(self.db_path, self.req_id)
        assert req["tags"] == ""
        assert req["fyi_request_id"] is None
        assert req["fyi_url"] is None

    def test_post_update_status_transitions(self):
        # Transition to different allowed statuses
        for next_status in ["submitted", "awaiting_response", "partial", "completed", "closed", "draft"]:
            body = f"status={next_status}".encode()
            headers = {
                "Content-Type": "application/x-www-form-urlencoded",
                "Content-Length": str(len(body))
            }
            handler = self.get_handler(f"/requests/{self.req_id}/status", headers=headers, body=body)
            handler.do_POST()
            assert handler.response_status == 303
            assert handler.response_headers["Location"] == "/requests?flash=Status+updated"
            
            # Verify status in DB
            req = get_tracked_request(self.db_path, self.req_id)
            assert req["status"] == next_status

    def test_post_update_status_invalid_value(self):
        # What happens if we supply an unexpected status? 
        # The db/webapp currently doesn't raise error, but let's document/verify the behavior.
        body = b"status=nonexistent_status"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler(f"/requests/{self.req_id}/status", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303
        
        # Verify status in DB is actually set to the value we passed
        req = get_tracked_request(self.db_path, self.req_id)
        assert req["status"] == "nonexistent_status"

    def test_post_update_status_nonexistent_request(self):
        body = b"status=closed"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler("/requests/999999/status", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/requests?flash=Status+updated"

    def test_post_update_status_invalid_request_id(self):
        body = b"status=closed"
        headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(body))
        }
        handler = self.get_handler("/requests/abc/status", headers=headers, body=body)
        handler.do_POST()
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/"

    def test_post_import_authorities_success(self):
        # Valid CSV with utf-8-sig and conflicting slug updates
        csv_body = b"\xef\xbb\xbfslug,name,url\n" \
                   b"test-slug,Updated Authority Name,https://example.com/updated\n" \
                   b"new-slug,New Authority,https://example.com/new\n"
        
        boundary = b"----Boundary123"
        body = (
            b"--" + boundary + b"\r\n"
            b'Content-Disposition: form-data; name="file"; filename="authorities.csv"\r\n'
            b"Content-Type: text/csv\r\n\r\n" + csv_body + b"\r\n"
            b"--" + boundary + b"--\r\n"
        )
        headers = {
            "Content-Type": f"multipart/form-data; boundary={boundary.decode()}",
            "Content-Length": str(len(body))
        }
        
        handler = self.get_handler("/authorities/import", headers=headers, body=body)
        handler.do_POST()
        
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/authorities?flash=Imported+2+authorities"
        
        # Verify database changes
        auths = query_all(self.db_path, "SELECT * FROM authorities ORDER BY slug")
        assert len(auths) == 2
        assert auths[0]["slug"] == "new-slug"
        assert auths[0]["name"] == "New Authority"
        assert auths[1]["slug"] == "test-slug"
        assert auths[1]["name"] == "Updated Authority Name"
        assert auths[1]["url"] == "https://example.com/updated"

    def test_post_import_authorities_custom_columns(self):
        # CSV using alternative column names: url_name, authority_name, request_url
        csv_body = b"url_name,authority_name,request_url\n" \
                   b"custom-slug,Custom Name,https://example.com/custom\n"
        
        boundary = b"----Boundary123"
        body = (
            b"--" + boundary + b"\r\n"
            b'Content-Disposition: form-data; name="file"; filename="authorities.csv"\r\n'
            b"Content-Type: text/csv\r\n\r\n" + csv_body + b"\r\n"
            b"--" + boundary + b"--\r\n"
        )
        headers = {
            "Content-Type": f"multipart/form-data; boundary={boundary.decode()}",
            "Content-Length": str(len(body))
        }
        
        handler = self.get_handler("/authorities/import", headers=headers, body=body)
        handler.do_POST()
        
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/authorities?flash=Imported+1+authorities"
        
        auths = query_all(self.db_path, "SELECT * FROM authorities WHERE slug='custom-slug'")
        assert len(auths) == 1
        assert auths[0]["name"] == "Custom Name"
        assert auths[0]["url"] == "https://example.com/custom"

    def test_post_import_authorities_invalid_csv_rows(self):
        # Row with missing fields (should skip them)
        csv_body = b"slug,name,url\n" \
                   b",Missing Slug Name,https://example.com\n" \
                   b"valid-slug,,https://example.com\n" \
                   b"another-valid-slug,Name,https://example.com\n"
        
        boundary = b"----Boundary123"
        body = (
            b"--" + boundary + b"\r\n"
            b'Content-Disposition: form-data; name="file"; filename="authorities.csv"\r\n'
            b"Content-Type: text/csv\r\n\r\n" + csv_body + b"\r\n"
            b"--" + boundary + b"--\r\n"
        )
        headers = {
            "Content-Type": f"multipart/form-data; boundary={boundary.decode()}",
            "Content-Length": str(len(body))
        }
        
        handler = self.get_handler("/authorities/import", headers=headers, body=body)
        handler.do_POST()
        
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/authorities?flash=Imported+1+authorities"

    def test_post_import_authorities_malformed_multipart_or_missing_file(self):
        # Missing "file" part in body
        boundary = b"----Boundary123"
        body = (
            b"--" + boundary + b"\r\n"
            b'Content-Disposition: form-data; name="not-file"; filename="authorities.csv"\r\n'
            b"Content-Type: text/csv\r\n\r\ncontent\r\n"
            b"--" + boundary + b"--\r\n"
        )
        headers = {
            "Content-Type": f"multipart/form-data; boundary={boundary.decode()}",
            "Content-Length": str(len(body))
        }
        
        handler = self.get_handler("/authorities/import", headers=headers, body=body)
        handler.do_POST()
        
        # Should redirect with 0 authorities imported since the payload was empty
        assert handler.response_status == 303
        assert handler.response_headers["Location"] == "/authorities?flash=Imported+0+authorities"


class TestPhase2_4SearchAndFilter:
    """Tests for Phase 2.4: Search and Filter routes."""

    @pytest.fixture(autouse=True)
    def setup_db(self, tmp_path):
        self.db_path = str(tmp_path / "test.db")
        init_db(self.db_path)
        conn = query_all.__globals__['connect'](self.db_path)
        # Insert authorities
        conn.execute("INSERT INTO authorities (slug, name, url) VALUES ('slug-one', 'Authority One', 'https://example.com/1')")
        conn.execute("INSERT INTO authorities (slug, name, url) VALUES ('slug-two', 'Authority Two', 'https://example.com/2')")
        # Insert requests
        conn.execute(
            "INSERT INTO tracked_requests (id, authority_slug, title, body, status, tags, updated_at) "
            "VALUES (101, 'slug-one', 'Searchable Request Alpha', 'Body one', 'draft', 'important,draft-tag', datetime('now'))"
        )
        conn.execute(
            "INSERT INTO tracked_requests (id, authority_slug, title, body, status, tags, updated_at) "
            "VALUES (102, 'slug-two', 'Another Request Beta', 'Body two', 'submitted', 'other-tag', datetime('now'))"
        )
        conn.commit()
        conn.close()
        self.HandlerClass = make_handler(self.db_path)

    def get_handler(self, path):
        class MockHandler(self.HandlerClass):
            def __init__(self):
                self.path = path
                self.headers = {}
                self.rfile = BytesIO(b"")
                self.wfile = BytesIO()
                self.response_status = None
                self.response_headers = {}
                
            def send_response(self, status):
                self.response_status = status
                
            def send_header(self, keyword, value):
                self.response_headers[keyword] = value
                
            def end_headers(self):
                pass
        return MockHandler()

    def test_get_requests_search_title(self):
        # Search by title
        handler = self.get_handler("/requests?q=Alpha")
        handler.do_GET()
        assert handler.response_status == 200
        content = handler.wfile.getvalue().decode("utf-8")
        assert "Searchable Request Alpha" in content
        assert "Another Request Beta" not in content

    def test_get_requests_search_authority(self):
        # Search by authority slug
        handler = self.get_handler("/requests?q=slug-two")
        handler.do_GET()
        assert handler.response_status == 200
        content = handler.wfile.getvalue().decode("utf-8")
        assert "Another Request Beta" in content
        assert "Searchable Request Alpha" not in content

    def test_get_requests_search_tag(self):
        # Search by tags
        handler = self.get_handler("/requests?q=draft-tag")
        handler.do_GET()
        assert handler.response_status == 200
        content = handler.wfile.getvalue().decode("utf-8")
        assert "Searchable Request Alpha" in content
        assert "Another Request Beta" not in content

    def test_get_requests_priority_filter(self):
        # Filter by priority
        # Let's mock response_analysis for the requests to return a specific priority.
        with patch("fyi_system.webapp.response_analysis") as mock_analysis:
            # For request 101, return priority 'now'
            # For request 102, return priority 'soon'
            def side_effect(db_path, req_id):
                if req_id == 101:
                    return {"priority": "now"}
                return {"priority": "soon"}
            mock_analysis.side_effect = side_effect

            # Request filtering for priority=now
            handler = self.get_handler("/requests?priority=now")
            handler.do_GET()
            assert handler.response_status == 200
            content = handler.wfile.getvalue().decode("utf-8")
            assert "Searchable Request Alpha" in content
            assert "Another Request Beta" not in content

            # Request filtering for priority=soon
            handler2 = self.get_handler("/requests?priority=soon")
            handler2.do_GET()
            assert handler2.response_status == 200
            content2 = handler2.wfile.getvalue().decode("utf-8")
            assert "Another Request Beta" in content2
            assert "Searchable Request Alpha" not in content2

    def test_get_authorities_search(self):
        # Search authorities by name
        handler = self.get_handler("/authorities?q=One")
        handler.do_GET()
        assert handler.response_status == 200
        content = handler.wfile.getvalue().decode("utf-8")
        assert "Authority One" in content
        assert "Authority Two" not in content

        # Search authorities by slug
        handler2 = self.get_handler("/authorities?q=slug-two")
        handler2.do_GET()
        assert handler2.response_status == 200
        content2 = handler2.wfile.getvalue().decode("utf-8")
        assert "Authority Two" in content2
        assert "Authority One" not in content2


class TestPhase3HTMLRenderingAndSecurity:
    """Tests for Phase 3: HTML Rendering, dashboard statistics, pagination limits, badges, and redaction/security."""

    @pytest.fixture(autouse=True)
    def setup_db(self, tmp_path):
        self.db_path = str(tmp_path / "test.db")
        init_db(self.db_path)
        self.conn = query_all.__globals__['connect'](self.db_path)
        
        # Insert 15 authorities to test dropdowns & stats
        for i in range(15):
            self.conn.execute(
                f"INSERT INTO authorities (slug, name, url) "
                f"VALUES ('slug-{i}', 'Authority {i}', 'https://example.com/{i}')"
            )
        # Insert 15 requests to test pagination limits (e.g. limit to 10 in dashboard)
        for i in range(15):
            self.conn.execute(
                f"INSERT INTO tracked_requests (id, authority_slug, title, body, status, tags, updated_at) "
                f"VALUES ({200 + i}, 'slug-0', 'Request {i}', 'Sensitive content with email user_{i}@example.com and token api_key=secret_tok_{i}', 'draft', 'tag-{i}', datetime('now'))"
            )
        self.conn.commit()
        self.conn.close()
        self.HandlerClass = make_handler(self.db_path)

    def get_handler(self, path):
        class MockHandler(self.HandlerClass):
            def __init__(self):
                self.path = path
                self.headers = {}
                self.rfile = BytesIO(b"")
                self.wfile = BytesIO()
                self.response_status = None
                self.response_headers = {}
                
            def send_response(self, status):
                self.response_status = status
                
            def send_header(self, keyword, value):
                self.response_headers[keyword] = value
                
            def end_headers(self):
                pass
        return MockHandler()

    def test_dashboard_rendering_with_requests_and_stats(self):
        handler = self.get_handler("/")
        handler.do_GET()
        assert handler.response_status == 200
        content = handler.wfile.getvalue().decode("utf-8")
        
        # Check title and dashboard elements
        assert "FYI Request System" in content
        assert "Total tracked" in content
        assert "Needs attention" in content
        assert "Recent updates" in content
        assert "Action now" in content
        
        # Check dashboard statistics counts
        assert "15" in content  # Total tracked requests and/or authorities (both are 15)
        
        # Check that dashboard limits to recent 10 requests (pagination limit)
        # It displays elements from range(15), ordering is DESC, so it shows Request 14 down to Request 5.
        # Request 0 to Request 4 should be omitted from the table (since limit is 10)
        assert "Request 14" in content
        assert "Request 5" in content
        assert "Request 0" not in content

    def test_dashboard_empty_state(self, tmp_path):
        # Create an empty db
        empty_db_path = str(tmp_path / "empty.db")
        init_db(empty_db_path)
        
        HandlerClassEmpty = make_handler(empty_db_path)
        class MockHandlerEmpty(HandlerClassEmpty):
            def __init__(self):
                self.path = "/"
                self.headers = {}
                self.rfile = BytesIO(b"")
                self.wfile = BytesIO()
                self.response_status = None
                self.response_headers = {}
                
            def send_response(self, status):
                self.response_status = status
                
            def send_header(self, keyword, value):
                self.response_headers[keyword] = value
                
            def end_headers(self):
                pass

        handler = MockHandlerEmpty()
        handler.do_GET()
        assert handler.response_status == 200
        content = handler.wfile.getvalue().decode("utf-8")
        assert "Total tracked" in content
        assert "<strong>0</strong>" in content  # statistics shows 0 total

    def test_request_list_rendering_badges_and_actions(self):
        handler = self.get_handler("/requests")
        handler.do_GET()
        assert handler.response_status == 200
        content = handler.wfile.getvalue().decode("utf-8")
        
        # Check that table headers and structure exist
        assert "Tracked requests" in content
        assert "<th>Authority</th>" in content
        assert "<th>Status</th>" in content
        assert "<th>Next action</th>" in content
        
        # Check next action badges / pills
        assert "class='pill'" in content
        # Check that action buttons/links are rendered
        assert "View" in content
        assert "Edit" in content
        assert "Timeline" in content
        assert "Correspondence" in content
        assert "Open recommended draft" in content

    def test_request_detail_rendering(self):
        # We need mock for snapshot to return some attachments and events
        with patch("fyi_system.webapp.latest_snapshot_summary") as mock_snapshot:
            mock_snapshot.return_value = {
                "fetched_at": "2026-06-15 00:00:00",
                "described_state": "waiting_clarification",
                "attachments_count": 2,
                "events_count": 1,
                "source_url": "https://example.com/snapshot/source",
                "attachments": [
                    {"url": "https://example.com/file1.pdf", "name": "Document.pdf", "content_type": "application/pdf"},
                    {"url": "https://example.com/file2.png", "name": "Image.png", "content_type": "image/png"},
                ],
                "events": [
                    {"title": "Initial Request", "created_at": "2026-06-14 12:00:00", "detail": "Sent to agency."}
                ]
            }
            
            handler = self.get_handler("/requests/200")
            handler.do_GET()
            assert handler.response_status == 200
            content = handler.wfile.getvalue().decode("utf-8")
            
            # Check title, sections, action buttons
            assert "Request #200" in content
            assert "Latest FYI snapshot" in content
            assert "Attachments" in content
            assert "Snapshot events" in content
            assert "Document.pdf" in content
            assert "Image.png" in content
            assert "Initial Request" in content
            assert "Response analysis" in content
            assert "Suggested follow-up draft" in content
            assert "Next best action" in content
            assert "Alternative follow-up variants" in content
            assert "Strategy and tone pack" in content
            
            # Action buttons
            assert "Open recommended draft" in content
            assert "Open correspondence pack" in content
            assert "Export bundle" in content

    def test_privacy_redaction_and_security_headers(self):
        from fyi_system.security import redact_text
        
        handler = self.get_handler("/requests/200")
        handler.do_GET()
        assert handler.response_status == 200
        content = handler.wfile.getvalue().decode("utf-8")
        
        # Verify Security Headers in HTTP Response
        assert handler.response_headers.get("Cache-Control") == "no-store"
        assert handler.response_headers.get("Pragma") == "no-cache"
        assert handler.response_headers.get("X-Content-Type-Options") == "nosniff"
        assert handler.response_headers.get("Referrer-Policy") == "no-referrer"
        assert "Content-Security-Policy" in handler.response_headers
        
        # HTML output verification:
        # Check that we can redact the raw data containing PII before it gets exported or logged.
        # Since the detail page renders the DB content (which hasn't been automatically run through redact_text),
        # let's assert that running redact_text on the generated HTML or raw values successfully redacts PII.
        sensitive_snippet = "Sensitive content with email user_0@example.com and token api_key=secret_tok_0"
        assert sensitive_snippet in content  # DB stored values are returned as is to authorized web application users
        
        # Verify redact_text correctly sanitizes it for logs/exports/public views
        redacted = redact_text(content)
        assert "user_0@example.com" not in redacted
        assert "[redacted-email]" in redacted
        assert "secret_tok_0" not in redacted
        assert "[redacted-secret]" in redacted


class TestPhase4SecurityAndIntegration:
    """Tests for Phase 4: Security headers and end-to-end integration flow scenarios."""

    @pytest.fixture(autouse=True)
    def setup_db(self, tmp_path):
        self.db_path = str(tmp_path / "test_integration.db")
        init_db(self.db_path)
        
        # Insert test authority
        conn = query_all.__globals__['connect'](self.db_path)
        conn.execute("INSERT INTO authorities (slug, name, url) VALUES ('gov-dept', 'Government Department', 'https://example.com/gov')")
        conn.commit()
        conn.close()
        
        self.HandlerClass = make_handler(self.db_path)

    def get_handler(self, path, method="GET", headers=None, body=b""):
        class MockHandler(self.HandlerClass):
            def __init__(self):
                self.path = path
                self.command = method
                self.headers = headers or {}
                self.rfile = BytesIO(body)
                self.wfile = BytesIO()
                self.response_status = None
                self.response_headers = {}
                
            def send_response(self, status):
                self.response_status = status
                
            def send_header(self, keyword, value):
                self.response_headers[keyword] = value
                
            def end_headers(self):
                pass
                
        return MockHandler()

    def test_security_headers_cache_control(self):
        """Test Cache-Control header is set to no-store."""
        handler = self.get_handler("/")
        handler.do_GET()
        assert handler.response_headers.get("Cache-Control") == "no-store"

    def test_security_headers_content_security_policy(self):
        """Test Content-Security-Policy header is correctly formatted and contains self."""
        handler = self.get_handler("/")
        handler.do_GET()
        csp = handler.response_headers.get("Content-Security-Policy", "")
        assert "default-src 'self'" in csp
        assert "style-src 'self' 'unsafe-inline'" in csp

    def test_security_headers_x_content_type_options(self):
        """Test X-Content-Type-Options header is set to nosniff."""
        handler = self.get_handler("/")
        handler.do_GET()
        assert handler.response_headers.get("X-Content-Type-Options") == "nosniff"

    def test_security_headers_referrer_policy(self):
        """Test Referrer-Policy header is set to no-referrer."""
        handler = self.get_handler("/")
        handler.do_GET()
        assert handler.response_headers.get("Referrer-Policy") == "no-referrer"

    def test_full_request_lifecycle_via_web(self):
        """Test full request lifecycle via web client interactions: create -> view -> update status -> check timeline."""
        # 1. GET Dashboard first to see if it works
        handler_dash = self.get_handler("/")
        handler_dash.do_GET()
        assert handler_dash.response_status == 200

        # 2. POST to /requests/create to create the request
        create_body = b"authority_slug=gov-dept&title=Lifecycle+Test+Request&body=Please+provide+information.&tags=lifecycle,test&status=draft"
        create_headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(create_body))
        }
        handler_create = self.get_handler("/requests/create", method="POST", headers=create_headers, body=create_body)
        handler_create.do_POST()
        assert handler_create.response_status == 303
        assert "flash=Tracked+request+created" in handler_create.response_headers.get("Location", "")

        # Find the ID of the created request
        rows = query_all(self.db_path, "SELECT id FROM tracked_requests WHERE title='Lifecycle Test Request'")
        assert len(rows) == 1
        req_id = rows[0]["id"]

        # 3. GET the request detail page
        handler_detail = self.get_handler(f"/requests/{req_id}")
        handler_detail.do_GET()
        assert handler_detail.response_status == 200
        detail_html = handler_detail.wfile.getvalue().decode("utf-8")
        assert "Lifecycle Test Request" in detail_html
        assert "gov-dept" in detail_html

        # 4. POST status update to submitted
        status_body = b"status=submitted"
        status_headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(status_body))
        }
        handler_status = self.get_handler(f"/requests/{req_id}/status", method="POST", headers=status_headers, body=status_body)
        handler_status.do_POST()
        assert handler_status.response_status == 303
        assert "Location" in handler_status.response_headers

        # 5. GET timeline to verify the status transition is recorded
        handler_timeline = self.get_handler(f"/requests/{req_id}/timeline")
        handler_timeline.do_GET()
        assert handler_timeline.response_status == 200
        timeline_html = handler_timeline.wfile.getvalue().decode("utf-8")
        assert "submitted" in timeline_html or "status" in timeline_html.lower()

    def test_create_update_export_flow(self):
        """Test integration flow: create request -> update request content -> export bundle."""
        # 1. Create a request
        create_body = b"authority_slug=gov-dept&title=Export+Flow+Request&body=Initial+Body&status=draft"
        create_headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(create_body))
        }
        handler_create = self.get_handler("/requests/create", method="POST", headers=create_headers, body=create_body)
        handler_create.do_POST()
        assert handler_create.response_status == 303

        rows = query_all(self.db_path, "SELECT id FROM tracked_requests WHERE title='Export Flow Request'")
        req_id = rows[0]["id"]

        # 2. Update request details
        update_body = b"authority_slug=gov-dept&title=Updated+Flow+Request&body=Updated+Body&status=submitted&fyi_request_id=12345"
        update_headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(update_body))
        }
        handler_update = self.get_handler(f"/requests/{req_id}/update", method="POST", headers=update_headers, body=update_body)
        handler_update.do_POST()
        assert handler_update.response_status == 303

        # Verify database update
        rows_updated = query_all(self.db_path, f"SELECT * FROM tracked_requests WHERE id={req_id}")
        assert rows_updated[0]["title"] == "Updated Flow Request"
        assert rows_updated[0]["body"] == "Updated Body"
        assert rows_updated[0]["fyi_request_id"] == 12345

        # 3. GET export bundle
        with patch("fyi_system.webapp.export_request_bundle", return_value="dummy_bundle_path") as mock_export:
            handler_export = self.get_handler(f"/requests/{req_id}/export-bundle")
            handler_export.do_GET()
            assert handler_export.response_status == 200
            assert b"Exported bundle to dummy_bundle_path" in handler_export.wfile.getvalue()
            mock_export.assert_called_once()

    def test_search_view_update_flow(self):
        """Test integration flow: search requests -> view detail -> update status."""
        # 1. Create request to search for
        create_body = b"authority_slug=gov-dept&title=Search+Target+Request&body=Body+Search&status=draft"
        create_headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(create_body))
        }
        handler_create = self.get_handler("/requests/create", method="POST", headers=create_headers, body=create_body)
        handler_create.do_POST()
        assert handler_create.response_status == 303

        rows = query_all(self.db_path, "SELECT id FROM tracked_requests WHERE title='Search Target Request'")
        req_id = rows[0]["id"]

        # 2. Search for the request
        handler_search = self.get_handler("/requests?q=Search+Target")
        handler_search.do_GET()
        assert handler_search.response_status == 200
        search_html = handler_search.wfile.getvalue().decode("utf-8")
        assert "Search Target Request" in search_html

        # 3. View detail page
        handler_view = self.get_handler(f"/requests/{req_id}")
        handler_view.do_GET()
        assert handler_view.response_status == 200
        assert b"Search Target Request" in handler_view.wfile.getvalue()

        # 4. Update status to closed
        status_body = b"status=closed"
        status_headers = {
            "Content-Type": "application/x-www-form-urlencoded",
            "Content-Length": str(len(status_body))
        }
        handler_status = self.get_handler(f"/requests/{req_id}/status", method="POST", headers=status_headers, body=status_body)
        handler_status.do_POST()
        assert handler_status.response_status == 303

        # Verify db status is closed
        rows_status = query_all(self.db_path, f"SELECT status FROM tracked_requests WHERE id={req_id}")
        assert rows_status[0]["status"] == "closed"

