"""Tests for Alaveteli API client."""
import pytest
from unittest.mock import Mock, patch, MagicMock
import json
from datetime import datetime
from pathlib import Path

from fyi_system.alaveteli_client import (
    AlaveteliClient,
    AlaveteliAPIError,
    AlaveteliRequest,
    AlaveteliCorrespondence,
    create_alaveteli_client,
    create_fyi_client,
)


@pytest.fixture
def client():
    """Create test client without API key (Read API only)."""
    return AlaveteliClient(base_url='https://test.alaveteli.org', timeout=10)


@pytest.fixture
def client_with_auth():
    """Create test client with API key (Write API enabled)."""
    return AlaveteliClient(
        base_url='https://test.alaveteli.org',
        api_key='test-api-key-123',
        timeout=10
    )


class TestAlaveteliRequest:
    """Test AlaveteliRequest dataclass."""
    
    def test_create_request(self):
        """Test creating request object."""
        req = AlaveteliRequest(
            id=12345,
            title='Test Request',
            body='Test body',
            user_name='Test User',
            status='successful'
        )
        
        assert req.id == 12345
        assert req.title == 'Test Request'
        assert req.body == 'Test body'
        assert req.user_name == 'Test User'
        assert req.status == 'successful'
    
    def test_request_to_dict(self):
        """Test request serialization."""
        req = AlaveteliRequest(
            id=123,
            title='Test',
            body='Body',
            tags=['tag1', 'tag2']
        )
        
        data = req.to_dict()
        
        assert data['id'] == 123
        assert data['title'] == 'Test'
        assert data['tags'] == ['tag1', 'tag2']


class TestAlaveteliCorrespondence:
    """Test AlaveteliCorrespondence dataclass."""
    
    def test_create_correspondence(self):
        """Test creating correspondence object."""
        corr = AlaveteliCorrespondence(
            direction='response',
            body='Response body',
            sent_at='2026-03-09T10:30:00Z',
            state='successful'
        )
        
        assert corr.direction == 'response'
        assert corr.body == 'Response body'
        assert corr.sent_at == '2026-03-09T10:30:00Z'
        assert corr.state == 'successful'
    
    def test_correspondence_to_dict(self):
        """Test correspondence serialization."""
        corr = AlaveteliCorrespondence(
            direction='request',
            body='Request body',
            sent_at='2026-03-09T10:30:00Z'
        )
        
        data = corr.to_dict()
        
        assert data['direction'] == 'request'
        assert data['body'] == 'Request body'


class TestAlaveteliClientInit:
    """Test client initialization."""
    
    def test_init_without_api_key(self):
        """Test client without API key."""
        client = AlaveteliClient(base_url='https://test.org')
        
        assert client.base_url == 'https://test.org'
        assert client.api_key is None
        assert client.timeout == 30
    
    def test_init_with_api_key(self):
        """Test client with API key."""
        client = AlaveteliClient(
            base_url='https://test.org',
            api_key='test-key',
            timeout=60
        )
        
        assert client.api_key == 'test-key'
        assert client.timeout == 60
        # API key should be in session params
        assert client.session.params == {'k': 'test-key'}
    
    def test_init_strips_trailing_slash(self):
        """Test base URL trailing slash is stripped."""
        client = AlaveteliClient(base_url='https://test.org/')
        assert client.base_url == 'https://test.org'


class TestReadAPI:
    """Test Read API methods (no authentication required)."""
    
    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_get_request(self, mock_get, client):
        """Test getting request by ID."""
        # Mock response
        mock_response = Mock()
        mock_response.json.return_value = {
            'id': 12345,
            'title': 'Test Request',
            'body': 'Test body',
            'status': 'successful',
            'url': 'https://test.org/request/12345'
        }
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response
        
        # Call method
        request = client.get_request(12345)
        
        # Verify
        assert request.id == 12345
        assert request.title == 'Test Request'
        mock_get.assert_called_once()
    
    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_get_feed_json(self, mock_get, client):
        """Test getting feed in JSON format."""
        mock_response = Mock()
        mock_response.json.return_value = {
            'entries': [
                {'id': 1, 'title': 'Entry 1'},
                {'id': 2, 'title': 'Entry 2'}
            ]
        }
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response
        
        feed = client.get_feed('latest', format='json')
        
        assert 'entries' in feed
        assert len(feed['entries']) == 2
        mock_get.assert_called_once()
    
    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_search_requests(self, mock_get, client):
        """Test searching requests."""
        mock_response = Mock()
        mock_response.json.return_value = {
            'results': [
                {'id': 1, 'title': 'Result 1'}
            ],
            'count': 1
        }
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response
        
        results = client.search_requests(
            query='official information',
            authority='ministry',
            status='successful',
            tags=['test']
        )
        
        assert 'results' in results
        mock_get.assert_called_once()
    
    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_get_request_error(self, mock_get, client):
        """Test error handling for Get Request."""
        mock_get.side_effect = AlaveteliAPIError("404 Not Found", 404)
        
        with pytest.raises(AlaveteliAPIError) as exc_info:
            client.get_request(99999)
        
        assert "404" in str(exc_info.value)


class TestWriteAPI:
    """Test Write API methods (authentication required)."""
    
    @patch('fyi_system.alaveteli_client.requests.Session.post')
    def test_create_request(self, mock_post, client_with_auth):
        """Test creating new request."""
        mock_response = Mock()
        mock_response.json.return_value = {
            'id': 12345,
            'url': 'https://test.org/request/12345'
        }
        mock_response.raise_for_status = Mock()
        mock_post.return_value = mock_response
        
        result = client_with_auth.create_request(
            title='Test Request',
            body='Test body content',
            external_user_name='John Doe',
            external_url='https://example.com/test',
            tags=['test', 'official-information']
        )
        
        assert result['id'] == 12345
        assert result['url'] == 'https://test.org/request/12345'
        
        # Verify JSON was sent correctly
        call_args = mock_post.call_args
        form_data = call_args[1]['data']
        json_data = json.loads(form_data['json'])
        
        assert json_data['title'] == 'Test Request'
        assert json_data['tags'] == 'test official-information'
    
    def test_create_request_without_api_key(self, client):
        """Test creating request without API key fails."""
        with pytest.raises(AlaveteliAPIError) as exc_info:
            client.create_request(
                title='Test',
                body='Body',
                external_user_name='User',
                external_url='https://example.com'
            )
        
        assert 'API key required' in str(exc_info.value)
    
    @patch('fyi_system.alaveteli_client.requests.Session.post')
    def test_add_correspondence_response(self, mock_post, client_with_auth):
        """Test adding response correspondence."""
        mock_response = Mock()
        mock_response.json.return_value = {'success': True}
        mock_response.raise_for_status = Mock()
        mock_post.return_value = mock_response
        
        result = client_with_auth.add_correspondence(
            request_id=12345,
            direction='response',
            body='Thank you for your request...',
            state='successful',
            sent_at='2026-03-09T10:30:00Z'
        )
        
        assert result['success'] is True
        
        # Verify data sent
        call_args = mock_post.call_args
        form_data = call_args[1]['data']
        json_data = json.loads(form_data['json'])
        
        assert json_data['direction'] == 'response'
        assert json_data['state'] == 'successful'
    
    @patch('fyi_system.alaveteli_client.requests.Session.post')
    def test_add_correspondence_with_attachments(self, mock_post, client_with_auth, tmp_path):
        """Test adding correspondence with file attachments."""
        # Create test file
        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"PDF content")
        
        mock_response = Mock()
        mock_response.json.return_value = {'success': True}
        mock_response.raise_for_status = Mock()
        mock_post.return_value = mock_response
        
        result = client_with_auth.add_correspondence(
            request_id=12345,
            direction='response',
            body='See attached',
            state='successful',
            attachment_paths=[str(test_file)]
        )
        
        # Verify files were sent
        call_args = mock_post.call_args
        assert 'files' in call_args[1]
    
    @patch('fyi_system.alaveteli_client.requests.Session.post')
    def test_update_request_state(self, mock_post, client_with_auth):
        """Test updating request state."""
        mock_response = Mock()
        mock_response.json.return_value = {'updated': True}
        mock_response.raise_for_status = Mock()
        mock_post.return_value = mock_response
        
        result = client_with_auth.update_request_state(
            request_id=12345,
            state='successful'
        )
        
        assert result['updated'] is True
        
        # Verify data
        call_args = mock_post.call_args
        form_data = call_args[1]['data']
        json_data = json.loads(form_data['json'])
        
        assert json_data['state'] == 'successful'
    
    def test_update_state_without_api_key(self, client):
        """Test updating state without API key fails."""
        with pytest.raises(AlaveteliAPIError) as exc_info:
            client.update_request_state(request_id=123, state='successful')
        
        assert 'API key required' in str(exc_info.value)


class TestHelperMethods:
    """Test helper methods."""
    
    def test_build_prefilled_url(self, client):
        """Test building prefilled URL."""
        url = client.build_prefilled_url(
            authority_slug='ministry-of-justice',
            title='Test Request',
            body='Test body',
            tags=['test', 'official']
        )
        
        assert 'ministry-of-justice' in url
        assert 'title=Test+Request' in url or 'title=Test%20Request' in url
        assert 'default_letter=' in url
        assert 'tags=' in url
    
    def test_build_prefilled_url_with_body_param(self, client):
        """Test building prefilled URL with body parameter."""
        url = client.build_prefilled_url(
            authority_slug='ministry',
            title='Title',
            body='Body',
            use_body_param=True
        )
        
        assert 'body=' in url
        assert 'default_letter=' not in url
    
    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_check_api_health_healthy(self, mock_get, client):
        """Test API health check (healthy)."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response
        
        health = client.check_api_health()
        
        assert health['status'] == 'healthy'
        assert health['status_code'] == 200
    
    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_check_api_health_error(self, mock_get, client):
        """Test API health check (error)."""
        mock_get.side_effect = AlaveteliAPIError("Connection failed", 503)
        
        health = client.check_api_health()
        
        assert health['status'] == 'error'
        assert health['status_code'] == 503
    
    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_get_api_version_found(self, mock_get, client):
        """Test getting API version (found)."""
        mock_response = Mock()
        mock_response.text = '<html>Powered by Alaveteli v0.42.1</html>'
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response
        
        version = client.get_api_version()
        
        assert version == '0.42.1'
    
    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_get_api_version_not_found(self, mock_get, client):
        """Test getting API version (not found)."""
        mock_response = Mock()
        mock_response.text = '<html>No version info</html>'
        mock_response.raise_for_status = Mock()
        mock_get.return_value = mock_response
        
        version = client.get_api_version()
        
        assert version is None


class TestConvenienceFunctions:
    """Test module-level convenience functions."""
    
    def test_create_alaveteli_client(self):
        """Test create_alaveteli_client function."""
        client = create_alaveteli_client(
            base_url='https://custom.alaveteli.org',
            api_key='custom-key'
        )
        
        assert client.base_url == 'https://custom.alaveteli.org'
        assert client.api_key == 'custom-key'
    
    def test_create_fyi_client(self):
        """Test create_fyi_client function."""
        client = create_fyi_client(api_key='fyi-key')
        
        assert client.base_url == 'https://fyi.org.nz'
        assert client.api_key == 'fyi-key'


class TestAlaveteliCompatibility:
    """Test compatibility with different Alaveteli instances."""
    
    def test_fyi_org_nz(self):
        """Test FYI.org.nz configuration."""
        client = create_fyi_client()
        assert client.base_url == 'https://fyi.org.nz'
    
    def test_whatdotheyknow_com(self):
        """Test WhatDoTheyKnow.com configuration."""
        client = create_alaveteli_client(base_url='https://www.whatdotheyknow.com')
        assert client.base_url == 'https://www.whatdotheyknow.com'
    
    def test_fragdenstaat_de(self):
        """Test FragDenStaat.de configuration."""
        client = create_alaveteli_client(base_url='https://fragdenstaat.de')
        assert client.base_url == 'https://fragdenstaat.de'
    
    def test_machine_tags(self, client):
        """Test machine tag format (spending_id:12345)."""
        url = client.build_prefilled_url(
            authority_slug='treasury',
            title='Spending Request',
            body='Body',
            tags=['spending_id:12345', 'year:2026']
        )
        
        # Machine tags should preserve colon notation
        assert 'spending_id:12345' in url or 'spending_id%3A12345' in url


class TestSustainabilityExtensions:
    """Test sustainability features like ETags, retries, and bulk exports."""

    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_rate_limit_headers_parsed(self, mock_get, client):
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.raise_for_status = Mock()
        mock_response.headers = {
            'RateLimit-Limit': '10',
            'RateLimit-Remaining': '9',
            'RateLimit-Reset': '45',
            'X-Advisory-Status': 'nominal'
        }
        mock_get.return_value = mock_response

        client._get('/api/v2/request/123.json')
        assert client.last_rate_limit['limit'] == '10'
        assert client.last_rate_limit['remaining'] == '9'
        assert client.last_rate_limit['reset'] == '45'
        assert client.last_rate_limit['advisory_status'] == 'nominal'

    @patch('fyi_system.alaveteli_client.requests.Session.get')
    @patch('fyi_system.alaveteli_client.db.get_cached_response')
    def test_etag_cache_hit_304(self, mock_get_cached, mock_get, client):
        mock_get_cached.return_value = {
            'etag': '"abc"',
            'last_modified': 'Mon, 09 Jul 2026 12:00:00 GMT',
            'response_body': '{"cached": true}'
        }
        mock_response = Mock()
        mock_response.status_code = 304
        mock_response.headers = {'ETag': '"abc"'}
        mock_get.return_value = mock_response

        res = client._get('/api/v2/request/123.json')
        assert res.status_code == 200
        assert res.text == '{"cached": true}'

    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_get_bulk_export(self, mock_get, client):
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.raise_for_status = Mock()
        mock_response.iter_lines.return_value = [
            '{"id": 1, "title": "Req 1"}',
            '{"id": 2, "title": "Req 2"}',
        ]
        mock_get.return_value = mock_response

        results = client.get_bulk_export()
        assert len(results) == 2
        assert results[0]['id'] == 1
        assert results[1]['title'] == 'Req 2'
        mock_get.assert_called_once_with(
            'https://test.alaveteli.org/api/v1/bulk_export',
            headers={},
            timeout=10,
            stream=True,
        )
        mock_response.close.assert_called_once()

    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_bulk_export_enforces_item_limit_before_unbounded_growth(self, mock_get, client):
        mock_response = Mock(status_code=200)
        mock_response.raise_for_status = Mock()
        mock_response.iter_lines.return_value = [
            '{"id": 1}',
            '{"id": 2}',
        ]
        mock_get.return_value = mock_response

        with pytest.raises(AlaveteliAPIError, match='item limit'):
            client.get_bulk_export(max_items=1)
        mock_response.close.assert_called_once()

    @patch('fyi_system.alaveteli_client.requests.Session.get')
    def test_bulk_export_enforces_byte_limit_and_closes_stream(self, mock_get, client):
        mock_response = Mock(status_code=200)
        mock_response.raise_for_status = Mock()
        mock_response.iter_lines.return_value = ['{"id": 12345}']
        mock_get.return_value = mock_response

        with pytest.raises(AlaveteliAPIError, match='response-byte limit'):
            client.get_bulk_export(max_bytes=3)
        mock_response.close.assert_called_once()
