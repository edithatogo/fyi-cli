"""Tests for security middleware - CSRF, input validation, security headers."""
import pytest
from http.server import BaseHTTPRequestHandler
from io import BytesIO
from fyi_system.security_middleware import (
    CSRFProtection,
    InputValidator,
    SecurityHeaders,
    generate_csrf_token,
    validate_csrf_token,
    sanitize_input,
    require_csrf,
    validate_request_data,
    CSRF_HEADER_NAME,
    CSRF_FORM_FIELD,
)


class MockHTTPRequestHandler(BaseHTTPRequestHandler):
    """Mock HTTP handler for testing."""
    
    def __init__(self):
        self.response_code = None
        self.headers_sent = {}
        self.body_written = b""
        self.csrf_token = None
        self.post_data = {}
    
    def send_response(self, code, message=None):
        self.response_code = code
    
    def send_header(self, keyword, value):
        self.headers_sent[keyword] = value
    
    def end_headers(self):
        pass
    
    def wfile_write(self, data):
        self.body_written = data
    
    # Alias for compatibility
    wfile = property(lambda self: type('obj', (object,), {'write': self.wfile_write})())


class TestCSRFProtection:
    """Test CSRF protection."""
    
    def test_generate_token(self):
        """Test CSRF token generation."""
        csrf = CSRFProtection()
        token1 = csrf.generate_token()
        token2 = csrf.generate_token()
        
        # Tokens should be non-empty
        assert token1
        assert token2
        
        # Tokens should be unique
        assert token1 != token2
        
        # Tokens should be URL-safe
        assert all(c.isalnum() or c in '-_' for c in token1)
    
    def test_validate_token_success(self):
        """Test successful token validation."""
        csrf = CSRFProtection()
        token = csrf.generate_token()
        
        result = csrf.validate_token(token, token)
        assert result is True
    
    def test_validate_token_failure(self):
        """Test failed token validation."""
        csrf = CSRFProtection()
        token1 = csrf.generate_token()
        token2 = csrf.generate_token()
        
        result = csrf.validate_token(token1, token2)
        assert result is False
    
    def test_validate_token_empty(self):
        """Test validation with empty tokens."""
        csrf = CSRFProtection()
        
        assert csrf.validate_token("", "token") is False
        assert csrf.validate_token("token", "") is False
        assert csrf.validate_token("", "") is False
    
    def test_validate_token_none(self):
        """Test validation with None tokens."""
        csrf = CSRFProtection()
        
        assert csrf.validate_token(None, "token") is False
        assert csrf.validate_token("token", None) is False
    
    def test_get_token_from_header(self):
        """Test extracting token from header."""
        csrf = CSRFProtection()
        handler = MockHTTPRequestHandler()
        handler.headers = {CSRF_HEADER_NAME: "test-token"}
        
        token = csrf.get_token_from_request(handler)
        assert token == "test-token"
    
    def test_generate_token_length(self):
        """Test token has correct length."""
        csrf = CSRFProtection()
        token = csrf.generate_token()
        
        # URL-safe tokens are longer due to encoding
        assert len(token) >= 32


class TestInputValidator:
    """Test input validation."""
    
    def test_validate_email_valid(self):
        """Test valid email addresses."""
        valid_emails = [
            "test@example.com",
            "user.name@domain.org",
            "user+tag@example.co.nz",
        ]
        
        for email in valid_emails:
            assert InputValidator.validate_email(email) is True
    
    def test_validate_email_invalid(self):
        """Test invalid email addresses."""
        invalid_emails = [
            "",
            "not-an-email",
            "@example.com",
            "test@",
            "test@example",
            "a" * 255 + "@example.com",  # Too long
        ]
        
        for email in invalid_emails:
            assert InputValidator.validate_email(email) is False
    
    def test_validate_url_valid(self):
        """Test valid URLs."""
        valid_urls = [
            "http://example.com",
            "https://fyi.org.nz",
            "https://example.com/path?query=value",
        ]
        
        for url in valid_urls:
            assert InputValidator.validate_url(url) is True
    
    def test_validate_url_invalid(self):
        """Test invalid URLs."""
        invalid_urls = [
            "",
            "not-a-url",
            "ftp://example.com",  # Wrong scheme
            "javascript:alert(1)",
        ]
        
        for url in invalid_urls:
            assert InputValidator.validate_url(url) is False
    
    def test_validate_slug_valid(self):
        """Test valid slugs."""
        valid_slugs = [
            "test",
            "test-slug",
            "multi-word-slug",
            "slug-123",
        ]
        
        for slug in valid_slugs:
            assert InputValidator.validate_slug(slug) is True
    
    def test_validate_slug_invalid(self):
        """Test invalid slugs."""
        invalid_slugs = [
            "",
            "Test",  # Uppercase
            "test_slug",  # Underscore
            "test slug",  # Space
            "test--slug",  # Double hyphen
        ]
        
        for slug in invalid_slugs:
            assert InputValidator.validate_slug(slug) is False
    
    def test_validate_title_valid(self):
        """Test valid titles."""
        assert InputValidator.validate_title("Test Request") is True
        assert InputValidator.validate_title("A" * 500) is True
    
    def test_validate_title_invalid(self):
        """Test invalid titles."""
        assert InputValidator.validate_title("") is False
        assert InputValidator.validate_title("A" * 501) is False
    
    def test_validate_body_valid(self):
        """Test valid body."""
        assert InputValidator.validate_body("Test body") is True
        assert InputValidator.validate_body("A" * 100000) is True
    
    def test_validate_body_invalid(self):
        """Test invalid body."""
        assert InputValidator.validate_body("") is False
        assert InputValidator.validate_body("A" * 100001) is False
    
    def test_sanitize_html(self):
        """Test HTML sanitization."""
        # Test that dangerous characters are escaped
        result = InputValidator.sanitize_html("<script>alert(1)</script>")
        assert "<script>" not in result
        assert "&lt;" in result
        assert "&gt;" in result
        
        result = InputValidator.sanitize_html("Hello & Goodbye")
        assert "&amp;" in result
        
        result = InputValidator.sanitize_html('Quote "test"')
        assert "&quot;" in result
        
        result = InputValidator.sanitize_html("Normal text")
        assert result == "Normal text"
    
    def test_sanitize_filename(self):
        """Test filename sanitization."""
        # Basic sanitization
        result = InputValidator.sanitize_filename("test.txt")
        assert result == "test.txt"
        
        # Path traversal removed
        result = InputValidator.sanitize_filename("../etc/passwd")
        assert ".." not in result
        assert "/" not in result
        
        # Dangerous chars removed
        result = InputValidator.sanitize_filename("file:name.txt")
        assert ":" not in result
        
        # Null byte removed
        result = InputValidator.sanitize_filename("file\x00name.txt")
        assert "\x00" not in result
        
        # Length limited
        result = InputValidator.sanitize_filename("a" * 300 + ".txt")
        assert len(result) <= 255
    
    def test_validate_integer_valid(self):
        """Test valid integer validation."""
        assert InputValidator.validate_integer("123") == 123
        assert InputValidator.validate_integer(123) == 123
        assert InputValidator.validate_integer("0") == 0
        assert InputValidator.validate_integer("-10") == -10
    
    def test_validate_integer_with_range(self):
        """Test integer validation with range."""
        assert InputValidator.validate_integer("5", min_val=0, max_val=10) == 5
        assert InputValidator.validate_integer("0", min_val=0, max_val=10) == 0
        assert InputValidator.validate_integer("10", min_val=0, max_val=10) == 10
        
        # Out of range
        assert InputValidator.validate_integer("-1", min_val=0, max_val=10) is None
        assert InputValidator.validate_integer("11", min_val=0, max_val=10) is None
    
    def test_validate_integer_invalid(self):
        """Test invalid integer validation."""
        assert InputValidator.validate_integer("abc") is None
        assert InputValidator.validate_integer("") is None
        assert InputValidator.validate_integer(None) is None


class TestSecurityHeaders:
    """Test security headers."""
    
    def test_get_all_headers(self):
        """Test getting all security headers."""
        headers = SecurityHeaders.get_all_headers()
        
        assert "Content-Security-Policy" in headers
        assert "X-Frame-Options" in headers
        assert "X-Content-Type-Options" in headers
        assert "Strict-Transport-Security" in headers
    
    def test_apply_headers(self):
        """Test applying headers to response."""
        handler = MockHTTPRequestHandler()
        SecurityHeaders.apply_headers(handler)
        
        # Check some key headers were applied
        assert "Content-Security-Policy" in handler.headers_sent
        assert "X-Frame-Options" in handler.headers_sent
    
    def test_add_header(self):
        """Test adding custom header."""
        SecurityHeaders.add_header("X-Custom-Header", "custom-value")
        headers = SecurityHeaders.get_all_headers()
        
        assert "X-Custom-Header" in headers
        assert headers["X-Custom-Header"] == "custom-value"
        
        # Clean up
        SecurityHeaders.remove_header("X-Custom-Header")
    
    def test_remove_header(self):
        """Test removing header."""
        # Add then remove
        SecurityHeaders.add_header("X-Temp", "temp")
        SecurityHeaders.remove_header("X-Temp")
        
        headers = SecurityHeaders.get_all_headers()
        assert "X-Temp" not in headers
    
    def test_csp_header_content(self):
        """Test CSP header has required directives."""
        headers = SecurityHeaders.get_all_headers()
        csp = headers.get("Content-Security-Policy", "")
        
        assert "default-src" in csp
        assert "'self'" in csp
        assert "frame-ancestors" in csp


class TestConvenienceFunctions:
    """Test module-level convenience functions."""
    
    def test_generate_csrf_token(self):
        """Test generate_csrf_token function."""
        token = generate_csrf_token()
        
        assert token
        assert len(token) >= 32
    
    def test_validate_csrf_token(self):
        """Test validate_csrf_token function."""
        token = generate_csrf_token()
        
        assert validate_csrf_token(token, token) is True
        assert validate_csrf_token(token, "wrong") is False
    
    def test_sanitize_input(self):
        """Test sanitize_input function."""
        result = sanitize_input("<script>alert(1)</script>")
        
        assert "<script>" not in result
        assert "&lt;script&gt;" in result


class TestDecorators:
    """Test request decorators."""
    
    def test_require_csrf_valid(self):
        """Test require_csrf with valid token."""
        handler = MockHTTPRequestHandler()
        token = generate_csrf_token()
        handler.csrf_token = token
        handler.headers = {CSRF_HEADER_NAME: token}
        handler.post_data = {}
        
        @require_csrf
        def test_method(self):
            return "success"
        
        result = test_method(handler)
        assert result == "success"
    
    def test_require_csrf_invalid(self):
        """Test require_csrf with invalid token."""
        handler = MockHTTPRequestHandler()
        handler.csrf_token = "valid-token"
        handler.headers = {CSRF_HEADER_NAME: "wrong-token"}
        handler.post_data = {}
        
        @require_csrf
        def test_method(self):
            return "success"
        
        result = test_method(handler)
        
        # Should return 403 error
        assert handler.response_code == 403
    
    def test_require_csrf_missing(self):
        """Test require_csrf with missing token."""
        handler = MockHTTPRequestHandler()
        handler.csrf_token = "valid-token"
        handler.headers = {}
        handler.post_data = {}
        
        @require_csrf
        def test_method(self):
            return "success"
        
        result = test_method(handler)
        
        # Should return 403 error
        assert handler.response_code == 403
    
    def test_validate_request_data_valid(self):
        """Test validate_request_data with valid data."""
        handler = MockHTTPRequestHandler()
        handler.post_data = {
            'title': 'Valid Title',
            'body': 'Valid body content',
        }
        
        @validate_request_data({
            'title': InputValidator.validate_title,
            'body': InputValidator.validate_body,
        })
        def test_method(self):
            return "success"
        
        result = test_method(handler)
        assert result == "success"
    
    def test_validate_request_data_invalid(self):
        """Test validate_request_data with invalid data."""
        handler = MockHTTPRequestHandler()
        handler.post_data = {
            'title': '',  # Invalid
            'body': 'Valid body',
        }
        
        @validate_request_data({
            'title': InputValidator.validate_title,
            'body': InputValidator.validate_body,
        })
        def test_method(self):
            return "success"
        
        result = test_method(handler)
        
        # Should return 400 error
        assert handler.response_code == 400


class TestSecurityIntegration:
    """Test security integration."""
    
    def test_csrf_token_round_trip(self):
        """Test CSRF token generation and validation."""
        token = generate_csrf_token()
        assert validate_csrf_token(token, token) is True
    
    def test_sanitize_then_validate(self):
        """Test sanitization followed by validation."""
        malicious = "<script>alert('xss')</script>"
        sanitized = sanitize_input(malicious)
        
        # Should be sanitized
        assert "<script>" not in sanitized
        
        # Should still be valid as text
        assert len(sanitized) > 0
    
    def test_all_headers_present(self):
        """Test all required security headers are present."""
        headers = SecurityHeaders.get_all_headers()
        
        required_headers = [
            "Content-Security-Policy",
            "X-Content-Type-Options",
            "X-Frame-Options",
            "X-XSS-Protection",
            "Referrer-Policy",
            "Cache-Control",
            "Strict-Transport-Security",
        ]
        
        for header in required_headers:
            assert header in headers, f"Missing required header: {header}"
