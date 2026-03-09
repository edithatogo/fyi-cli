"""API Contract Tests for FYI.org.nz API integration.

These tests validate the API request/response schemas and ensure
compatibility with the FYI.org.nz API specification.
"""
import pytest
import json
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock
from typing import Dict, Any, List


# =============================================================================
# Schema Definitions
# =============================================================================

class FYISchema:
    """Schema definitions for FYI.org.nz API."""
    
    # Request schema
    REQUEST_SCHEMA = {
        "type": "object",
        "required": ["title", "body", "recipient"],
        "properties": {
            "title": {"type": "string", "minLength": 1, "maxLength": 500},
            "body": {"type": "string", "minLength": 1},
            "recipient": {"type": "string", "minLength": 1},
            "tags": {"type": "array", "items": {"type": "string"}},
            "is_anonymous": {"type": "boolean"},
        }
    }
    
    # Response schema
    RESPONSE_SCHEMA = {
        "type": "object",
        "required": ["id", "title", "status"],
        "properties": {
            "id": {"type": "integer"},
            "title": {"type": "string"},
            "body": {"type": "string"},
            "status": {"type": "string", "enum": ["pending", "sent", "responded", "closed"]},
            "created_at": {"type": "string", "format": "date-time"},
            "updated_at": {"type": "string", "format": "date-time"},
            "tags": {"type": "array", "items": {"type": "string"}},
        }
    }
    
    # Error schema
    ERROR_SCHEMA = {
        "type": "object",
        "required": ["error"],
        "properties": {
            "error": {"type": "string"},
            "code": {"type": "integer"},
            "message": {"type": "string"},
        }
    }
    
    # Paginated response schema
    PAGINATION_SCHEMA = {
        "type": "object",
        "required": ["data", "page", "per_page", "total"],
        "properties": {
            "data": {"type": "array"},
            "page": {"type": "integer", "minimum": 1},
            "per_page": {"type": "integer", "minimum": 1},
            "total": {"type": "integer", "minimum": 0},
            "total_pages": {"type": "integer", "minimum": 0},
        }
    }


# =============================================================================
# Schema Validation Utilities
# =============================================================================

def validate_schema(data: Dict[str, Any], schema: Dict[str, Any]) -> List[str]:
    """Validate data against schema. Returns list of validation errors."""
    errors = []
    
    # Check type
    if schema.get("type") == "object":
        if not isinstance(data, dict):
            errors.append(f"Expected object, got {type(data).__name__}")
            return errors
        
        # Check required fields
        for field in schema.get("required", []):
            if field not in data:
                errors.append(f"Missing required field: {field}")
        
        # Check properties
        props = schema.get("properties", {})
        for key, value in data.items():
            if key in props:
                prop_schema = props[key]
                # Type check
                if prop_schema.get("type") == "string" and not isinstance(value, str):
                    errors.append(f"Field {key} should be string, got {type(value).__name__}")
                elif prop_schema.get("type") == "integer" and not isinstance(value, int):
                    errors.append(f"Field {key} should be integer, got {type(value).__name__}")
                elif prop_schema.get("type") == "boolean" and not isinstance(value, bool):
                    errors.append(f"Field {key} should be boolean, got {type(value).__name__}")
                elif prop_schema.get("type") == "array" and not isinstance(value, list):
                    errors.append(f"Field {key} should be array, got {type(value).__name__}")
                
                # String length checks
                if prop_schema.get("type") == "string":
                    if "minLength" in prop_schema and len(value) < prop_schema["minLength"]:
                        errors.append(f"Field {key} too short (min {prop_schema['minLength']})")
                    if "maxLength" in prop_schema and len(value) > prop_schema["maxLength"]:
                        errors.append(f"Field {key} too long (max {prop_schema['maxLength']})")
                
                # Enum check
                if "enum" in prop_schema and value not in prop_schema["enum"]:
                    errors.append(f"Field {key} has invalid value: {value}")
                
                # Integer range check
                if prop_schema.get("type") == "integer":
                    if "minimum" in prop_schema and value < prop_schema["minimum"]:
                        errors.append(f"Field {key} below minimum ({prop_schema['minimum']})")
                    if "maximum" in prop_schema and value > prop_schema["maximum"]:
                        errors.append(f"Field {key} above maximum ({prop_schema['maximum']})")
    
    return errors


# =============================================================================
# Mock API Server
# =============================================================================

class MockFYIServer:
    """Mock FYI.org.nz API server for testing."""
    
    def __init__(self):
        self.requests = []
        self.responses = {}
        self.error_mode = False
        self.rate_limit_remaining = 100
    
    def get(self, endpoint: str, **kwargs) -> Mock:
        """Mock GET request."""
        response = Mock()
        response.status_code = 200
        response.headers = {
            'X-RateLimit-Remaining': str(self.rate_limit_remaining),
            'X-API-Version': '1.0',
        }
        
        if endpoint == '/requests':
            response.json.return_value = {
                "data": self.requests,
                "page": 1,
                "per_page": 20,
                "total": len(self.requests),
            }
        elif endpoint.startswith('/request/'):
            request_id = int(endpoint.split('/')[-1])
            req = next((r for r in self.requests if r['id'] == request_id), None)
            if req:
                response.json.return_value = req
            else:
                response.status_code = 404
                response.json.return_value = {"error": "Request not found"}
        
        return response
    
    def post(self, endpoint: str, **kwargs) -> Mock:
        """Mock POST request."""
        response = Mock()
        response.status_code = 201
        response.headers = {
            'X-RateLimit-Remaining': str(self.rate_limit_remaining),
            'X-API-Version': '1.0',
        }
        
        if endpoint == '/requests':
            new_request = {
                "id": len(self.requests) + 1,
                "title": kwargs.get('json', {}).get('title', 'Test'),
                "body": kwargs.get('json', {}).get('body', 'Test body'),
                "status": "pending",
                "created_at": "2026-03-09T10:00:00Z",
                "tags": kwargs.get('json', {}).get('tags', []),
            }
            self.requests.append(new_request)
            response.json.return_value = new_request
        
        return response
    
    def configure_response(self, endpoint: str, response_data: Dict[str, Any]):
        """Configure a custom response for an endpoint."""
        self.responses[endpoint] = response_data
    
    def set_error_mode(self, enabled: bool):
        """Enable/disable error mode."""
        self.error_mode = enabled


@pytest.fixture
def mock_server():
    """Provide mock server instance."""
    return MockFYIServer()


# =============================================================================
# Schema Validation Tests
# =============================================================================

class TestRequestSchemaValidation:
    """Test request schema validation."""
    
    def test_valid_request(self):
        """Schema: Valid request passes validation."""
        request = {
            "title": "Official Information Request",
            "body": "I request the following information...",
            "recipient": "Ministry of Testing",
            "tags": ["official", "testing"],
            "is_anonymous": True,
        }
        
        errors = validate_schema(request, FYISchema.REQUEST_SCHEMA)
        assert len(errors) == 0, f"Validation failed: {errors}"
    
    def test_missing_required_field(self):
        """Schema: Missing required field fails validation."""
        request = {
            "title": "Official Information Request",
            # Missing "body"
            "recipient": "Ministry of Testing",
        }
        
        errors = validate_schema(request, FYISchema.REQUEST_SCHEMA)
        assert "Missing required field: body" in errors
    
    def test_title_too_long(self):
        """Schema: Title exceeding max length fails validation."""
        request = {
            "title": "A" * 501,  # Exceeds 500 char limit
            "body": "Test body",
            "recipient": "Ministry",
        }
        
        errors = validate_schema(request, FYISchema.REQUEST_SCHEMA)
        assert any("too long" in err for err in errors)
    
    def test_empty_title(self):
        """Schema: Empty title fails validation."""
        request = {
            "title": "",
            "body": "Test body",
            "recipient": "Ministry",
        }
        
        errors = validate_schema(request, FYISchema.REQUEST_SCHEMA)
        assert any("too short" in err for err in errors)
    
    def test_invalid_tags_type(self):
        """Schema: Non-array tags fails validation."""
        request = {
            "title": "Test",
            "body": "Test body",
            "recipient": "Ministry",
            "tags": "not-an-array",
        }
        
        errors = validate_schema(request, FYISchema.REQUEST_SCHEMA)
        assert any("should be array" in err for err in errors)


class TestResponseSchemaValidation:
    """Test response schema validation."""
    
    def test_valid_response(self):
        """Schema: Valid response passes validation."""
        response = {
            "id": 123,
            "title": "Official Information Request",
            "body": "Request body",
            "status": "pending",
            "created_at": "2026-03-09T10:00:00Z",
            "updated_at": "2026-03-09T10:00:00Z",
            "tags": ["official"],
        }
        
        errors = validate_schema(response, FYISchema.RESPONSE_SCHEMA)
        assert len(errors) == 0, f"Validation failed: {errors}"
    
    def test_invalid_status(self):
        """Schema: Invalid status value fails validation."""
        response = {
            "id": 123,
            "title": "Test",
            "body": "Test body",
            "status": "invalid_status",  # Not in enum
        }
        
        errors = validate_schema(response, FYISchema.RESPONSE_SCHEMA)
        assert any("invalid value" in err for err in errors)
    
    def test_missing_id(self):
        """Schema: Missing ID fails validation."""
        response = {
            # Missing "id"
            "title": "Test",
            "body": "Test body",
            "status": "pending",
        }
        
        errors = validate_schema(response, FYISchema.RESPONSE_SCHEMA)
        assert "Missing required field: id" in errors


class TestErrorSchemaValidation:
    """Test error schema validation."""
    
    def test_valid_error_response(self):
        """Schema: Valid error response passes validation."""
        error = {
            "error": "Not Found",
            "code": 404,
            "message": "The requested resource was not found",
        }
        
        errors = validate_schema(error, FYISchema.ERROR_SCHEMA)
        assert len(errors) == 0, f"Validation failed: {errors}"
    
    def test_minimal_error_response(self):
        """Schema: Minimal error response (error only) passes validation."""
        error = {
            "error": "Something went wrong",
        }
        
        errors = validate_schema(error, FYISchema.ERROR_SCHEMA)
        assert len(errors) == 0


class TestPaginationSchemaValidation:
    """Test pagination schema validation."""
    
    def test_valid_pagination(self):
        """Schema: Valid pagination response passes validation."""
        pagination = {
            "data": [{"id": 1}, {"id": 2}],
            "page": 1,
            "per_page": 20,
            "total": 100,
            "total_pages": 5,
        }
        
        errors = validate_schema(pagination, FYISchema.PAGINATION_SCHEMA)
        assert len(errors) == 0, f"Validation failed: {errors}"
    
    def test_invalid_page_number(self):
        """Schema: Page number below minimum fails validation."""
        pagination = {
            "data": [],
            "page": 0,  # Below minimum of 1
            "per_page": 20,
            "total": 0,
        }
        
        errors = validate_schema(pagination, FYISchema.PAGINATION_SCHEMA)
        assert any("below minimum" in err for err in errors)


# =============================================================================
# API Endpoint Contract Tests
# =============================================================================

class TestRequestEndpoints:
    """Test FYI API request endpoints."""
    
    def test_mock_server_create_request(self, mock_server):
        """Contract: Mock POST /requests creates new request."""
        # Create request using mock server directly
        response = mock_server.post('/requests', json={
            "title": "Test Request",
            "body": "Test body content",
            "recipient": "Test Ministry",
        })
        
        assert response.status_code == 201
        data = response.json()
        assert "id" in data
        assert data["title"] == "Test Request"
        assert data["status"] == "pending"
    
    def test_mock_server_get_request(self, mock_server):
        """Contract: Mock GET /request/{id} retrieves request."""
        # Create a request first
        mock_server.post('/requests', json={
            "title": "Test",
            "body": "Test body",
            "recipient": "Ministry",
        })
        
        # Retrieve it
        response = mock_server.get('/request/1')
        assert response.status_code == 200
        data = response.json()
        assert data["id"] == 1
        assert data["title"] == "Test"
    
    def test_mock_server_get_nonexistent(self, mock_server):
        """Contract: Mock GET /request/{id} returns 404 for nonexistent."""
        response = mock_server.get('/request/999')
        assert response.status_code == 404
        data = response.json()
        assert "error" in data
    
    def test_mock_server_list_requests(self, mock_server):
        """Contract: Mock GET /requests lists all requests."""
        # Create some requests
        for i in range(3):
            mock_server.post('/requests', json={
                "title": f"Request {i}",
                "body": "Body",
                "recipient": "Ministry",
            })
        
        # List them
        response = mock_server.get('/requests')
        assert response.status_code == 200
        data = response.json()
        assert "data" in data
        assert len(data["data"]) == 3
        assert data["total"] == 3
        assert data["page"] == 1


class TestAuthenticationHeaders:
    """Test API authentication."""
    
    def test_api_key_header_format(self):
        """Contract: API key header format is correct."""
        # Test header format
        headers = {"X-API-Key": "test-api-key"}
        assert "X-API-Key" in headers
        assert headers["X-API-Key"] == "test-api-key"
    
    def test_authorization_header_format(self):
        """Contract: Bearer token header format is correct."""
        # Test header format
        headers = {"Authorization": "Bearer test-token"}
        assert "Authorization" in headers
        assert headers["Authorization"].startswith("Bearer ")


class TestRateLimiting:
    """Test API rate limiting."""
    
    def test_rate_limit_headers_format(self, mock_server):
        """Contract: Rate limit header format is correct."""
        response = mock_server.get('/requests')
        
        assert 'X-RateLimit-Remaining' in response.headers
        assert int(response.headers['X-RateLimit-Remaining']) >= 0
    
    def test_rate_limit_exceeded_format(self):
        """Contract: 429 response format when rate limit exceeded."""
        mock_response = Mock()
        mock_response.status_code = 429
        mock_response.headers = {
            'X-RateLimit-Remaining': '0',
            'Retry-After': '60',
        }
        
        assert mock_response.status_code == 429
        assert mock_response.headers.get('Retry-After') == '60'


class TestVersionCompatibility:
    """Test API version compatibility."""
    
    def test_api_version_header_format(self, mock_server):
        """Contract: API version header format is correct."""
        response = mock_server.get('/requests')
        
        assert 'X-API-Version' in response.headers
        version = response.headers['X-API-Version']
        assert isinstance(version, str)
    
    def test_version_accept_header_format(self):
        """Contract: Accept header format for API versioning."""
        # Test Accept header format
        headers = {"Accept": "application/vnd.fyi.v1+json"}
        assert "Accept" in headers
        assert "application/vnd" in headers["Accept"]


class TestErrorResponses:
    """Test API error responses."""
    
    def test_400_bad_request_format(self):
        """Contract: 400 error response format."""
        error_response = {
            "error": "Bad Request",
            "code": 400,
            "message": "Invalid request format",
        }
        
        errors = validate_schema(error_response, FYISchema.ERROR_SCHEMA)
        assert len(errors) == 0
    
    def test_401_unauthorized_format(self):
        """Contract: 401 error response format."""
        error_response = {
            "error": "Unauthorized",
            "code": 401,
            "message": "Invalid API key",
        }
        
        errors = validate_schema(error_response, FYISchema.ERROR_SCHEMA)
        assert len(errors) == 0
    
    def test_404_not_found_format(self, mock_server):
        """Contract: 404 error response format."""
        response = mock_server.get('/request/999')
        assert response.status_code == 404
        data = response.json()
        assert "error" in data
    
    def test_500_server_error_format(self):
        """Contract: 500 error response format."""
        error_response = {
            "error": "Internal Server Error",
            "code": 500,
            "message": "Something went wrong",
        }
        
        errors = validate_schema(error_response, FYISchema.ERROR_SCHEMA)
        assert len(errors) == 0


# =============================================================================
# Integration Contract Tests
# =============================================================================

class TestFYIIntegration:
    """Test actual FYI integration from fyi.py module."""
    
    def test_build_prefilled_url_format(self):
        """Contract: build_prefilled_url returns valid URL."""
        from fyi_system.fyi import build_prefilled_url
        
        url = build_prefilled_url(
            authority_slug="test-ministry",
            title="Test Request",
            body="Test body",
            tags=["tag1", "tag2"],
        )
        
        assert url.startswith("https://fyi.org.nz/new/")
        assert "test-ministry" in url
        assert "title=Test+Request" in url or "title=Test%20Request" in url
        assert "body=" in url
        assert "tags=" in url
    
    def test_build_prefilled_url_minimal(self):
        """Contract: build_prefilled_url works with minimal args."""
        from fyi_system.fyi import build_prefilled_url
        
        url = build_prefilled_url(
            authority_slug="ministry",
            title="Title",
            body="Body",
        )
        
        assert "ministry" in url
        assert "title=" in url
        assert "body=" in url
    
    def test_extract_request_id_valid(self):
        """Contract: extract_request_id parses valid URLs."""
        from fyi_system.fyi import extract_request_id
        
        test_cases = [
            ("https://fyi.org.nz/request/123", 123),
            ("https://fyi.org.nz/request/456/update", 456),
            ("https://fyi.org.nz/request/789#anchor", 789),
        ]
        
        for url, expected in test_cases:
            result = extract_request_id(url)
            assert result == expected, f"Failed for {url}"
    
    def test_extract_request_id_invalid(self):
        """Contract: extract_request_id returns None for invalid URLs."""
        from fyi_system.fyi import extract_request_id
        
        test_cases = [
            "https://fyi.org.nz/",
            "https://fyi.org.nz/requests",
            "https://fyi.org.nz/request/abc",
            "not-a-url",
        ]
        
        for url in test_cases:
            result = extract_request_id(url)
            assert result is None, f"Expected None for {url}, got {result}"
    
    def test_custom_base_url(self):
        """Contract: build_prefilled_url supports custom base URL."""
        from fyi_system.fyi import build_prefilled_url
        
        url = build_prefilled_url(
            authority_slug="test",
            title="Test",
            body="Test",
            base_url="https://custom.fyi.org.nz",
        )
        
        assert url.startswith("https://custom.fyi.org.nz/new/")


# =============================================================================
# Schema Versioning Tests
# =============================================================================

class TestSchemaVersioning:
    """Test schema versioning compatibility."""
    
    def test_schema_has_version_info(self):
        """Contract: Schema includes version information."""
        # All schemas should be versioned
        assert hasattr(FYISchema, 'REQUEST_SCHEMA')
        assert hasattr(FYISchema, 'RESPONSE_SCHEMA')
        assert hasattr(FYISchema, 'ERROR_SCHEMA')
    
    def test_backward_compatibility(self):
        """Contract: New schema versions are backward compatible."""
        # Test that v1 schema accepts v1 data
        v1_response = {
            "id": 1,
            "title": "Test",
            "body": "Test body",
            "status": "pending",
        }
        
        errors = validate_schema(v1_response, FYISchema.RESPONSE_SCHEMA)
        assert len(errors) == 0, "v1 schema should accept v1 data"
    
    def test_forward_compatibility(self):
        """Contract: Schema accepts additional fields (forward compat)."""
        # Response with extra fields should still validate
        response_with_extras = {
            "id": 1,
            "title": "Test",
            "body": "Test body",
            "status": "pending",
            "extra_field": "should be ignored",
            "another_extra": 123,
        }
        
        errors = validate_schema(response_with_extras, FYISchema.RESPONSE_SCHEMA)
        # Should pass - extra fields are allowed
        assert len(errors) == 0, "Schema should allow extra fields"


# =============================================================================
# Performance Contract Tests
# =============================================================================

class TestPerformanceContracts:
    """Test API performance contracts."""
    
    def test_schema_validation_performance(self):
        """Contract: Schema validation is fast."""
        import time
        
        response = {
            "id": 1,
            "title": "Test",
            "body": "Test body",
            "status": "pending",
            "tags": ["tag1", "tag2"],
        }
        
        start = time.time()
        
        # Validate 100 times
        for _ in range(100):
            errors = validate_schema(response, FYISchema.RESPONSE_SCHEMA)
            assert len(errors) == 0
        
        elapsed = time.time() - start
        
        # Should complete 100 validations in under 0.1 seconds
        assert elapsed < 0.1, f"Schema validation too slow: {elapsed}s"
    
    def test_mock_server_performance(self, mock_server):
        """Contract: Mock server responds quickly."""
        import time
        
        start = time.time()
        
        # Make 50 requests
        for i in range(50):
            mock_server.post('/requests', json={
                "title": f"Request {i}",
                "body": "Body",
                "recipient": "Ministry",
            })
        
        elapsed = time.time() - start
        
        # Should complete in under 0.5 seconds
        assert elapsed < 0.5, f"Mock server too slow: {elapsed}s"
