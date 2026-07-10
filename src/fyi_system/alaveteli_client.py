"""Alaveteli API Client for FYI.org.nz and compatible platforms.

This module provides a comprehensive client for the Alaveteli API,
supporting both Read API (public data) and Write API (programmatic operations).

Compatible with:
- FYI.org.nz (New Zealand)
- WhatDoTheyKnow.com (UK)
- FragDenStaat.de (Germany)
- Any Alaveteli v0.39+ deployment

API Documentation: https://alaveteli.org/docs/developers/api/
"""
from __future__ import annotations
import json
import os
import requests
from .agent_runtime import build_user_agent, retry_delay_seconds
import time
from pathlib import Path
from typing import Optional, Dict, Any, List
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from . import db


@dataclass
class AlaveteliRequest:
    """Represents an Alaveteli request object."""
    id: int
    title: str
    body: str
    user_name: Optional[str] = None
    status: Optional[str] = None
    created_at: Optional[str] = None
    updated_at: Optional[str] = None
    url: Optional[str] = None
    tags: Optional[List[str]] = None
    
    def to_dict(self) -> dict:
        """Convert to dictionary."""
        return asdict(self)


@dataclass
class AlaveteliCorrespondence:
    """Represents correspondence (message) on a request."""
    direction: str  # 'request' or 'response'
    body: str
    sent_at: str
    state: Optional[str] = None  # 'waiting_response', 'rejected', 'successful', 'partially_successful'
    attachments: Optional[List[str]] = None  # File paths for attachments
    
    def to_dict(self) -> dict:
        """Convert to dictionary."""
        return asdict(self)


class AlaveteliAPIError(Exception):
    """Base exception for Alaveteli API errors."""
    def __init__(self, message: str, status_code: Optional[int] = None):
        super().__init__(message)
        self.status_code = status_code


class AlaveteliClient:
    """Client for Alaveteli API (Read + Write API).
    
    Usage:
        # Read API (no authentication required)
        client = AlaveteliClient(base_url='https://fyi.org.nz')
        request = client.get_request(12345)
        
        # Write API (authentication required)
        client = AlaveteliClient(
            base_url='https://fyi.org.nz',
            api_key='your-api-key'
        )
        new_request = client.create_request(...)
    """
    
    def __init__(
        self,
        base_url: str = 'https://fyi.org.nz',
        api_key: Optional[str] = None,
        timeout: int = 30,
        db_path: str = 'fyi_system.db'
    ):
        """Initialize Alaveteli API client.
        
        Args:
            base_url: Base URL of Alaveteli instance (e.g., https://fyi.org.nz)
            api_key: API key for Write API operations (optional for Read API)
            timeout: Request timeout in seconds
            db_path: Path to local SQLite database for caching
        """
        self.base_url = base_url.rstrip('/')
        self.api_key = api_key
        self.timeout = timeout
        self.db_path = db_path
        self.session = requests.Session()
        self.session.headers["User-Agent"] = build_user_agent(
            os.environ.get("FYI_ADMIN_CONTACT")
        )
        self.last_rate_limit = {}
        
        if api_key:
            self.session.params = {'k': api_key}
            # Set bot token header for rate-limit bypass
            self.session.headers['X-FYI-Bot-Token'] = api_key
    
    def _get(self, endpoint: str, **kwargs) -> requests.Response:
        """Make GET request with rate-limit handling and ETag caching.
        
        Args:
            endpoint: API endpoint (e.g., '/api/v2/request/123.json')
            **kwargs: Additional requests.get() arguments
        
        Returns:
            requests.Response object
        
        Raises:
            AlaveteliAPIError: On API error
        """
        url = f"{self.base_url}{endpoint}"
        headers = kwargs.pop('headers', {})

        # Load cached ETag/Last-Modified details
        cached = None
        try:
            cached = db.get_cached_response(self.db_path, url)
        except Exception:
            pass

        if cached:
            if cached.get('etag'):
                headers['If-None-Match'] = cached['etag']
            if cached.get('last_modified'):
                headers['If-Modified-Since'] = cached['last_modified']
        
        retries = 0
        max_retries = 3
        while retries < max_retries:
            try:
                response = self.session.get(url, headers=headers, timeout=self.timeout, **kwargs)
                
                response_headers = response.headers if isinstance(response.headers, Mapping) else {}
                header_names = {str(name).lower() for name in response_headers}

                # Capture rate limits while tolerating test doubles without headers.
                if any(key.lower() in header_names for key in (
                    'RateLimit-Limit', 'RateLimit-Remaining', 'RateLimit-Reset', 'Retry-After'
                )):
                    self.last_rate_limit = {
                        'limit': response_headers.get('RateLimit-Limit'),
                        'remaining': response_headers.get('RateLimit-Remaining'),
                        'reset': response_headers.get('RateLimit-Reset'),
                        'retry_after': response_headers.get('Retry-After'),
                        'advisory_status': response_headers.get('X-Advisory-Status', 'nominal')
                    }

                # Handle 429 Too Many Requests
                if response.status_code == 429:
                    retry_after = retry_delay_seconds(
                        response_headers, attempt=retries, max_seconds=300
                    )
                    time.sleep(retry_after)
                    retries += 1
                    continue

                # Handle ETag cache hits (304 Not Modified)
                if response.status_code == 304 and cached:
                    mock_response = requests.Response()
                    mock_response.status_code = 200
                    mock_response._content = cached['response_body'].encode('utf-8')
                    mock_response.headers = response_headers
                    return mock_response

                response.raise_for_status()

                # Cache successful GET responses containing ETags or Last-Modified
                etag = response_headers.get('ETag')
                last_modified = response_headers.get('Last-Modified')
                if response.status_code == 200 and (etag or last_modified):
                    try:
                        db.set_cached_response(self.db_path, url, etag, last_modified, response.text)
                    except Exception:
                        pass

                return response
            except requests.exceptions.HTTPError as e:
                if response.status_code == 429:
                    retries += 1
                    continue
                raise AlaveteliAPIError(f"API error: {e}", response.status_code)
            except requests.exceptions.RequestException as e:
                raise AlaveteliAPIError(f"Request failed: {e}")

        raise AlaveteliAPIError("Request failed: Max retries exceeded due to rate limiting", 429)
    
    def _post(self, endpoint: str, data: Optional[Dict] = None, 
              files: Optional[Dict] = None, **kwargs) -> requests.Response:
        """Make POST request.
        
        Args:
            endpoint: API endpoint
            data: JSON data (will be sent as form variable 'json')
            files: Multipart file attachments
            **kwargs: Additional requests.post() arguments
        
        Returns:
            requests.Response object
        
        Raises:
            AlaveteliAPIError: On API error
        """
        url = f"{self.base_url}{endpoint}"
        
        # Prepare form data with JSON as 'json' variable
        form_data = {}
        if data:
            form_data['json'] = json.dumps(data)
        
        try:
            response = self.session.post(
                url,
                data=form_data,
                files=files,
                timeout=self.timeout,
                **kwargs
            )
            response.raise_for_status()
            return response
        except requests.exceptions.HTTPError as e:
            raise AlaveteliAPIError(f"API error: {e}", response.status_code)
        except requests.exceptions.RequestException as e:
            raise AlaveteliAPIError(f"Request failed: {e}")
    
    # ========== Read API Methods ==========
    
    def get_request(self, request_id: int) -> AlaveteliRequest:
        """Get full information about a request.
        
        Endpoint: GET /api/v2/request/<=id>.json
        
        Args:
            request_id: Request ID
        
        Returns:
            AlaveteliRequest object
        
        Raises:
            AlaveteliAPIError: On API error
        """
        response = self._get(f'/api/v2/request/{request_id}.json')
        data = response.json()
        
        return AlaveteliRequest(
            id=data.get('id'),
            title=data.get('title'),
            body=data.get('body'),
            user_name=data.get('user_name'),
            status=data.get('status'),
            created_at=data.get('created_at'),
            updated_at=data.get('updated_at'),
            url=data.get('url'),
            tags=data.get('tags', [])
        )
    
    def get_feed(self, feed_type: str = 'latest', format: str = 'json') -> Dict[str, Any]:
        """Get RSS/Atom feed data.
        
        Endpoints:
            - /request/latest.rss (or .json)
            - /authority/<slug>.rss (or .json)
            - /search.rss (or .json)
        
        Args:
            feed_type: Type of feed ('latest', 'authority', 'search')
            format: Response format ('json' or 'rss')
        
        Returns:
            Feed data as dictionary (JSON) or text (RSS)
        """
        endpoint = f'/request/{feed_type}.{format}'
        response = self._get(endpoint)
        
        if format == 'json':
            return response.json()
        else:
            return response.text
    
    def search_requests(
        self,
        query: str,
        authority: Optional[str] = None,
        status: Optional[str] = None,
        tags: Optional[List[str]] = None,
        format: str = 'json'
    ) -> Dict[str, Any]:
        """Search requests.
        
        Endpoint: /search.json (or .rss)
        
        Args:
            query: Search query
            authority: Filter by authority slug
            status: Filter by status
            tags: Filter by tags
            format: Response format
        
        Returns:
            Search results
        """
        params = {'query': query}
        
        if authority:
            params['authority'] = authority
        if status:
            params['status'] = status
        if tags:
            params['tags'] = ' '.join(tags)
        
        response = self._get('/search.json', params=params)
        return response.json()
    
    def get_bulk_export(self) -> List[Dict[str, Any]]:
        """Fetch bulk request metadata stream from Alaveteli.
        
        Endpoint: GET /api/v1/bulk_export
        
        Returns:
            List of request metadata dictionaries
        """
        response = self._get('/api/v1/bulk_export')
        results = []
        for line in response.text.strip().split("\n"):
            if line:
                results.append(json.loads(line))
        return results

    # ========== Write API Methods ==========
    
    def create_request(
        self,
        title: str,
        body: str,
        external_user_name: str,
        external_url: str,
        tags: Optional[List[str]] = None
    ) -> Dict[str, Any]:
        """Create a new request via API.
        
        Endpoint: POST /api/v2/request
        
        Args:
            title: Request title
            body: Request body content
            external_user_name: Name of person originating request
            external_url: URL where canonical copy can be found
            tags: Optional tags for the request
        
        Returns:
            Dictionary with 'url' and 'id' of new request
        
        Raises:
            AlaveteliAPIError: If API key not provided or API error
        
        Example:
            result = client.create_request(
                title='Official Information Request',
                body='I request the following information...',
                external_user_name='John Doe',
                external_url='https://example.com/request/123',
                tags=['official-information', 'test']
            )
            print(f"Created request {result['id']}: {result['url']}")
        """
        if not self.api_key:
            raise AlaveteliAPIError("API key required for Write API")
        
        data = {
            'title': title,
            'body': body,
            'external_user_name': external_user_name,
            'external_url': external_url
        }
        
        if tags:
            data['tags'] = ' '.join(tags)
        
        response = self._post('/api/v2/request', data=data)
        return response.json()
    
    def add_correspondence(
        self,
        request_id: int,
        direction: str,
        body: str,
        sent_at: Optional[str] = None,
        state: Optional[str] = None,
        attachment_paths: Optional[List[str]] = None
    ) -> Dict[str, Any]:
        """Add correspondence to a request.
        
        Endpoint: POST /api/v2/request/<id>.json
        
        Args:
            request_id: Request ID
            direction: 'request' (from user) or 'response' (from authority)
            body: Message content
            sent_at: ISO-8601 timestamp (default: now)
            state: State update (only for 'response' direction)
                   Values: 'waiting_response', 'rejected', 'successful', 'partially_successful'
            attachment_paths: List of file paths for attachments (only for 'response')
        
        Returns:
            Response data
        
        Raises:
            AlaveteliAPIError: If API key not provided or API error
        
        Example:
            # Add response from authority
            client.add_correspondence(
                request_id=12345,
                direction='response',
                body='Thank you for your request...',
                state='successful',
                sent_at='2026-03-09T10:30:00Z'
            )
            
            # Add response with attachment
            client.add_correspondence(
                request_id=12345,
                direction='response',
                body='Please see attached document',
                state='successful',
                attachment_paths=['/path/to/document.pdf']
            )
        """
        if not self.api_key:
            raise AlaveteliAPIError("API key required for Write API")
        
        if sent_at is None:
            sent_at = datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
        
        data = {
            'direction': direction,
            'body': body,
            'sent_at': sent_at
        }
        
        if direction == 'response':
            if state:
                data['state'] = state
            
            # Handle attachments
            files = {}
            if attachment_paths:
                for i, path in enumerate(attachment_paths):
                    file_path = Path(path)
                    if file_path.exists():
                        files[f'attachment_{i}'] = (
                            file_path.name,
                            file_path.open('rb'),
                            'application/octet-stream'
                        )
                
                response = self._post(f'/api/v2/request/{request_id}.json', 
                                     data=data, files=files)
            else:
                response = self._post(f'/api/v2/request/{request_id}.json', data=data)
        else:
            response = self._post(f'/api/v2/request/{request_id}.json', data=data)
        
        return response.json()
    
    def update_request_state(
        self,
        request_id: int,
        state: str
    ) -> Dict[str, Any]:
        """Update request state (user feedback).
        
        Endpoint: POST /api/v2/request/<id>/update.json
        
        Args:
            request_id: Request ID
            state: User's assessment of request state
                   Values: 'waiting_response', 'rejected', 'successful', 'partially_successful'
        
        Returns:
            Response data
        
        Raises:
            AlaveteliAPIError: If API key not provided or API error
        
        Note:
            Only for user feedback. Authorities should use add_correspondence() instead.
        """
        if not self.api_key:
            raise AlaveteliAPIError("API key required for Write API")
        
        data = {'state': state}
        response = self._post(f'/api/v2/request/{request_id}/update.json', data=data)
        return response.json()
    
    # ========== Helper Methods ==========
    
    def build_prefilled_url(
        self,
        authority_slug: str,
        title: str,
        body: str,
        tags: Optional[List[str]] = None,
        use_body_param: bool = False
    ) -> str:
        """Build prefilled request URL (Read API alternative).
        
        This is a non-API alternative that opens the web form with prefilled data.
        Useful when Write API is not available or for manual review before submission.
        
        URL Pattern: /new/<authority>?title=&body=&tags=
        
        Args:
            authority_slug: Authority URL slug
            title: Request title
            body: Request body (default_letter or body parameter)
            tags: Optional tags
            use_body_param: If True, use 'body' param instead of 'default_letter'
                           ('body' sets entire text including salutation/signoff)
        
        Returns:
            Prefilled URL string
        
        Example:
            url = client.build_prefilled_url(
                authority_slug='ministry-of-justice',
                title='Official Information Request',
                body='I request the following information...',
                tags=['official-information', 'test']
            )
            # Opens browser with prefilled form
        """
        from urllib.parse import urlencode
        
        params = {
            'title': title
        }
        
        if use_body_param:
            params['body'] = body
        else:
            params['default_letter'] = body
        
        if tags:
            # Machine tags use ':' notation (e.g., spending_id:12345)
            params['tags'] = ' '.join(tags)
        
        return f"{self.base_url}/new/{authority_slug}?{urlencode(params)}"
    
    def check_api_health(self) -> Dict[str, Any]:
        """Check API health and connectivity.
        
        Returns:
            Dictionary with health status
        """
        try:
            response = self._get('/')
            return {
                'status': 'healthy',
                'status_code': response.status_code,
                'base_url': self.base_url,
                'api_key_configured': bool(self.api_key)
            }
        except AlaveteliAPIError as e:
            return {
                'status': 'error',
                'error': str(e),
                'status_code': e.status_code,
                'base_url': self.base_url,
                'api_key_configured': bool(self.api_key)
            }
    
    def get_api_version(self) -> Optional[str]:
        """Attempt to detect Alaveteli version.
        
        Note: Version is not always publicly disclosed for security.
        
        Returns:
            Version string or None if not available
        """
        try:
            # Try to get version from various locations
            response = self._get('/about')
            
            # Look for version in HTML
            import re
            version_match = re.search(
                r'Alaveteli\s+v?(\d+\.\d+(?:\.\d+)?)',
                response.text,
                re.IGNORECASE
            )
            
            if version_match:
                return version_match.group(1)
            
        except AlaveteliAPIError:
            pass
        
        return None


# Convenience functions for simple usage

def create_alaveteli_client(
    base_url: str = 'https://fyi.org.nz',
    api_key: Optional[str] = None
) -> AlaveteliClient:
    """Create Alaveteli API client.
    
    Args:
        base_url: Alaveteli instance URL
        api_key: API key for Write API (optional)
    
    Returns:
        AlaveteliClient instance
    """
    return AlaveteliClient(base_url=base_url, api_key=api_key)


def create_fyi_client(api_key: Optional[str] = None) -> AlaveteliClient:
    """Create FYI.org.nz API client.
    
    Args:
        api_key: API key for Write API (optional)
    
    Returns:
        AlaveteliClient configured for FYI.org.nz
    """
    return create_alaveteli_client(base_url='https://fyi.org.nz', api_key=api_key)
