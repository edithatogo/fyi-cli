"""Security middleware for web application.

This module provides CSRF protection, security headers, and input validation
for the web application.
"""
from __future__ import annotations
import secrets
import hashlib
import re
from typing import Optional, Dict, Any, Callable
from functools import wraps
from http.server import BaseHTTPRequestHandler


# CSRF Configuration
CSRF_TOKEN_LENGTH = 32
CSRF_HEADER_NAME = "X-CSRF-Token"
CSRF_FORM_FIELD = "csrf_token"

# Security Headers
SECURITY_HEADERS = {
    "Content-Security-Policy": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'",
    "X-Content-Type-Options": "nosniff",
    "X-Frame-Options": "DENY",
    "X-XSS-Protection": "1; mode=block",
    "Referrer-Policy": "strict-origin-when-cross-origin",
    "Cache-Control": "no-store, no-cache, must-revalidate, proxy-revalidate",
    "Pragma": "no-cache",
    "Strict-Transport-Security": "max-age=31536000; includeSubDomains",
}

# Input Validation Patterns
EMAIL_PATTERN = re.compile(r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$')
URL_PATTERN = re.compile(r'^https?://[^\s]+$')
SLUG_PATTERN = re.compile(r'^[a-z0-9]+(?:-[a-z0-9]+)*$')
SAFE_TEXT_PATTERN = re.compile(r'^[\w\s.,!?@#$%&*()\-+=\[\]{}|;:\'"/\\<>]*$')

# Maximum lengths
MAX_TITLE_LENGTH = 500
MAX_BODY_LENGTH = 100000
MAX_TAG_LENGTH = 100
MAX_EMAIL_LENGTH = 254


class CSRFProtection:
    """CSRF token generation and validation."""
    
    def __init__(self, secret_key: Optional[str] = None):
        """Initialize CSRF protection.
        
        Args:
            secret_key: Optional secret key for token generation
        """
        self.secret_key = secret_key or secrets.token_hex(32)
    
    def generate_token(self) -> str:
        """Generate a new CSRF token.
        
        Returns:
            CSRF token string
        """
        return secrets.token_urlsafe(CSRF_TOKEN_LENGTH)
    
    def validate_token(self, token: str, session_token: str) -> bool:
        """Validate a CSRF token.
        
        Args:
            token: Token from request
            session_token: Token stored in session
        
        Returns:
            True if valid
        """
        if not token or not session_token:
            return False
        
        # Use constant-time comparison to prevent timing attacks
        return secrets.compare_digest(token, session_token)
    
    def get_token_from_request(self, handler: BaseHTTPRequestHandler) -> Optional[str]:
        """Extract CSRF token from request.
        
        Checks header first, then form data.
        
        Args:
            handler: HTTP request handler
        
        Returns:
            Token string or None
        """
        # Check header
        token = handler.headers.get(CSRF_HEADER_NAME)
        if token:
            return token
        
        # Token will be in form data for POST requests
        return None


class InputValidator:
    """Input validation utilities."""
    
    @staticmethod
    def validate_email(email: str) -> bool:
        """Validate email address.
        
        Args:
            email: Email to validate
        
        Returns:
            True if valid
        """
        if not email or len(email) > MAX_EMAIL_LENGTH:
            return False
        return bool(EMAIL_PATTERN.match(email))
    
    @staticmethod
    def validate_url(url: str) -> bool:
        """Validate URL.
        
        Args:
            url: URL to validate
        
        Returns:
            True if valid
        """
        if not url or len(url) > 2048:
            return False
        return bool(URL_PATTERN.match(url))
    
    @staticmethod
    def validate_slug(slug: str) -> bool:
        """Validate slug format.
        
        Args:
            slug: Slug to validate
        
        Returns:
            True if valid
        """
        if not slug or len(slug) > 100:
            return False
        return bool(SLUG_PATTERN.match(slug))
    
    @staticmethod
    def validate_title(title: str) -> bool:
        """Validate request title.
        
        Args:
            title: Title to validate
        
        Returns:
            True if valid
        """
        if not title or len(title) > MAX_TITLE_LENGTH:
            return False
        return True
    
    @staticmethod
    def validate_body(body: str) -> bool:
        """Validate request body.
        
        Args:
            body: Body text to validate
        
        Returns:
            True if valid
        """
        if not body or len(body) > MAX_BODY_LENGTH:
            return False
        return True
    
    @staticmethod
    def validate_tag(tag: str) -> bool:
        """Validate tag.
        
        Args:
            tag: Tag to validate
        
        Returns:
            True if valid
        """
        if not tag or len(tag) > MAX_TAG_LENGTH:
            return False
        return bool(SAFE_TEXT_PATTERN.match(tag))
    
    @staticmethod
    def sanitize_html(text: str) -> str:
        """Sanitize HTML by escaping dangerous characters.
        
        Args:
            text: Text to sanitize
        
        Returns:
            Sanitized text
        """
        if not text:
            return ""
        
        # Escape HTML special characters
        replacements = {
            '&': '&amp;',
            '<': '&lt;',
            '>': '&gt;',
            '"': '&quot;',
            "'": '&#x27;',
            '/': '&#x2F;',
        }
        
        result = text
        for char, replacement in replacements.items():
            result = result.replace(char, replacement)
        
        return result
    
    @staticmethod
    def sanitize_filename(filename: str) -> str:
        """Sanitize filename for safe storage.
        
        Args:
            filename: Filename to sanitize
        
        Returns:
            Sanitized filename
        """
        if not filename:
            return ""
        
        # Remove path components
        filename = filename.split('/')[-1].split('\\')[-1]
        
        # Remove dangerous characters
        unsafe_chars = ['..', '/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0']
        for char in unsafe_chars:
            filename = filename.replace(char, '')
        
        # Limit length
        return filename[:255]
    
    @staticmethod
    def validate_integer(value: Any, min_val: Optional[int] = None, 
                        max_val: Optional[int] = None) -> Optional[int]:
        """Validate and convert to integer.
        
        Args:
            value: Value to validate
            min_val: Minimum allowed value
            max_val: Maximum allowed value
        
        Returns:
            Validated integer or None
        """
        try:
            int_val = int(value)
            
            if min_val is not None and int_val < min_val:
                return None
            if max_val is not None and int_val > max_val:
                return None
            
            return int_val
        except (ValueError, TypeError):
            return None


class SecurityHeaders:
    """Security header management."""
    
    @staticmethod
    def get_all_headers() -> Dict[str, str]:
        """Get all security headers.
        
        Returns:
            Dictionary of header name to value
        """
        return SECURITY_HEADERS.copy()
    
    @staticmethod
    def apply_headers(handler: BaseHTTPRequestHandler) -> None:
        """Apply security headers to response.
        
        Args:
            handler: HTTP request handler
        """
        for header, value in SECURITY_HEADERS.items():
            handler.send_header(header, value)
    
    @staticmethod
    def add_header(name: str, value: str) -> None:
        """Add or override a security header.
        
        Args:
            name: Header name
            value: Header value
        """
        SECURITY_HEADERS[name] = value
    
    @staticmethod
    def remove_header(name: str) -> None:
        """Remove a security header.
        
        Args:
            name: Header name
        """
        SECURITY_HEADERS.pop(name, None)


# Decorators for request validation

def require_csrf(f: Callable) -> Callable:
    """Decorator to require CSRF token for POST requests.
    
    Usage:
        @require_csrf
        def handle_post(self):
            ...
    """
    @wraps(f)
    def wrapper(self, *args, **kwargs):
        # Get CSRF protection instance
        csrf = CSRFProtection()
        
        # Get token from request
        token = csrf.get_token_from_request(self)
        
        # For form submissions, token might be in POST data
        if not token and hasattr(self, 'post_data'):
            token = self.post_data.get(CSRF_FORM_FIELD)
        
        # Get session token
        session_token = getattr(self, 'csrf_token', None)
        
        # Validate
        if not csrf.validate_token(token, session_token):
            self.send_response(403)
            self.send_header("Content-Type", "application/json")
            SecurityHeaders.apply_headers(self)
            self.end_headers()
            self.wfile.write(b'{"error": "CSRF token missing or invalid"}')
            return
        
        return f(self, *args, **kwargs)
    
    return wrapper


def validate_request_data(validators: Dict[str, Callable]) -> Callable:
    """Decorator to validate request data.
    
    Usage:
        @validate_request_data({
            'title': InputValidator.validate_title,
            'body': InputValidator.validate_body,
        })
        def create_request(self):
            ...
    """
    def decorator(f: Callable) -> Callable:
        @wraps(f)
        def wrapper(self, *args, **kwargs):
            # Get data to validate
            data = getattr(self, 'post_data', {})
            
            # Validate each field
            errors = []
            for field, validator in validators.items():
                value = data.get(field)
                if value is not None and not validator(value):
                    errors.append(f"Invalid {field}")
            
            if errors:
                self.send_response(400)
                self.send_header("Content-Type", "application/json")
                SecurityHeaders.apply_headers(self)
                self.end_headers()
                error_json = f'{{"error": "Validation failed", "details": {errors}}}'.encode()
                self.wfile.write(error_json)
                return
            
            return f(self, *args, **kwargs)
        
        return wrapper
    return decorator


# Convenience functions

def generate_csrf_token() -> str:
    """Generate a new CSRF token.
    
    Returns:
        CSRF token string
    """
    csrf = CSRFProtection()
    return csrf.generate_token()


def validate_csrf_token(token: str, session_token: str) -> bool:
    """Validate a CSRF token.
    
    Args:
        token: Token from request
        session_token: Token from session
    
    Returns:
        True if valid
    """
    csrf = CSRFProtection()
    return csrf.validate_token(token, session_token)


def sanitize_input(text: str) -> str:
    """Sanitize user input.
    
    Args:
        text: Text to sanitize
    
    Returns:
        Sanitized text
    """
    return InputValidator.sanitize_html(text)
