# Alaveteli API Client Usage Guide

## Overview

The `alaveteli_client` module provides a comprehensive Python client for the Alaveteli API, supporting both Read API (public data access) and Write API (programmatic operations).

**Compatible with:**
- FYI.org.nz (New Zealand)
- WhatDoTheyKnow.com (United Kingdom)
- FragDenStaat.de (Germany)
- Any Alaveteli v0.39+ deployment

---

## Quick Start

### Read API (No Authentication Required)

```python
from fyi_system.alaveteli_client import create_fyi_client

# Create client for FYI.org.nz
client = create_fyi_client()

# Get request details
request = client.get_request(12345)
print(f"Request: {request.title}")
print(f"Status: {request.status}")

# Get latest feed
feed = client.get_feed('latest', format='json')
print(f"Latest entries: {len(feed.get('entries', []))}")

# Search requests
results = client.search_requests(
    query='climate change',
    authority='ministry',
    status='successful'
)
print(f"Found {results.get('count', 0)} results")
```

### Write API (Authentication Required)

```python
from fyi_system.alaveteli_client import create_fyi_client

# Create client with API key
client = create_fyi_client(api_key='your-api-key-here')

# Create new request
result = client.create_request(
    title='Official Information Request',
    body='I request the following information...',
    external_user_name='John Doe',
    external_url='https://example.com/request/123',
    tags=['official-information', 'climate']
)
print(f"Created request {result['id']}: {result['url']}")

# Add correspondence (response from authority)
client.add_correspondence(
    request_id=12345,
    direction='response',
    body='Thank you for your request. Here is the information...',
    state='successful',
    sent_at='2026-03-09T10:30:00Z'
)

# Update request state (user feedback)
client.update_request_state(
    request_id=12345,
    state='successful'
)
```

---

## API Reference

### Client Initialization

```python
from fyi_system.alaveteli_client import AlaveteliClient

# Basic client (Read API only)
client = AlaveteliClient(
    base_url='https://fyi.org.nz',
    timeout=30
)

# Client with authentication (Write API enabled)
client = AlaveteliClient(
    base_url='https://fyi.org.nz',
    api_key='your-api-key',
    timeout=30
)

# Custom Alaveteli instance
client = AlaveteliClient(
    base_url='https://www.whatdotheyknow.com',
    api_key='uk-api-key'
)
```

### Read API Methods

#### `get_request(request_id: int) -> AlaveteliRequest`

Get full information about a request.

```python
request = client.get_request(12345)
print(request.title)
print(request.body)
print(request.status)
print(request.created_at)
```

#### `get_feed(feed_type: str, format: str) -> Dict`

Get RSS/Atom feed data.

```python
# Latest requests (JSON)
feed = client.get_feed('latest', format='json')

# Latest requests (RSS/Atom)
rss = client.get_feed('latest', format='rss')

# Authority-specific feed
feed = client.get_feed('authority/ministry-of-justice', format='json')
```

#### `search_requests(query: str, ...) -> Dict`

Search requests with filters.

```python
results = client.search_requests(
    query='spending',
    authority='treasury',
    status='successful',
    tags=['2026', 'quarterly']
)

for result in results.get('results', []):
    print(f"{result['title']} - {result['status']}")
```

### Write API Methods

#### `create_request(...) -> Dict`

Create a new request programmatically.

```python
result = client.create_request(
    title='Request for Departmental Spending Data',
    body='I request the following information under the Official Information Act...',
    external_user_name='Jane Smith',
    external_url='https://example.org/request/456',
    tags=['spending', 'official-information']
)

print(f"Request created: {result['url']}")
print(f"Request ID: {result['id']}")
```

**Parameters:**
- `title` (required): Request title
- `body` (required): Request body content
- `external_user_name` (required): Name of person originating request
- `external_url` (required): URL where canonical copy can be found
- `tags` (optional): List of tags (space-separated in API)

#### `add_correspondence(...) -> Dict`

Add correspondence to a request.

```python
# Add response from authority
client.add_correspondence(
    request_id=12345,
    direction='response',
    body='Thank you for your Official Information request...',
    state='successful',
    sent_at='2026-03-09T10:30:00Z'
)

# Add response with attachment
client.add_correspondence(
    request_id=12345,
    direction='response',
    body='Please see the attached document',
    state='successful',
    attachment_paths=['/path/to/response.pdf']
)

# Add request from user
client.add_correspondence(
    request_id=12345,
    direction='request',
    body='Thank you for your response. I have a follow-up question...'
)
```

**Parameters:**
- `request_id` (required): Request ID
- `direction` (required): 'request' or 'response'
- `body` (required): Message content
- `sent_at` (optional): ISO-8601 timestamp (default: now)
- `state` (optional): State for responses only
  - 'waiting_response'
  - 'rejected'
  - 'successful'
  - 'partially_successful'
- `attachment_paths` (optional): List of file paths (responses only)

#### `update_request_state(request_id: int, state: str) -> Dict`

Update request state (user feedback).

```python
client.update_request_state(
    request_id=12345,
    state='successful'
)
```

**Note:** Only for user feedback. Authorities should use `add_correspondence()` with state parameter instead.

### Helper Methods

#### `build_prefilled_url(...) -> str`

Generate prefilled URL for manual submission.

```python
url = client.build_prefilled_url(
    authority_slug='ministry-of-justice',
    title='Official Information Request',
    body='I request the following information...',
    tags=['official-information']
)

# Open in browser
import webbrowser
webbrowser.open(url)
```

**Parameters:**
- `authority_slug` (required): Authority URL slug
- `title` (required): Request title
- `body` (required): Request body
- `tags` (optional): List of tags
- `use_body_param` (optional): Use 'body' instead of 'default_letter'

#### `check_api_health() -> Dict`

Check API connectivity and health.

```python
health = client.check_api_health()
print(f"Status: {health['status']}")
print(f"API Key Configured: {health['api_key_configured']}")
```

**Returns:**
```json
{
  "status": "healthy",
  "status_code": 200,
  "base_url": "https://fyi.org.nz",
  "api_key_configured": true
}
```

#### `get_api_version() -> Optional[str]`

Attempt to detect Alaveteli version.

```python
version = client.get_api_version()
if version:
    print(f"Alaveteli version: {version}")
else:
    print("Version not publicly disclosed")
```

---

## Error Handling

```python
from fyi_system.alaveteli_client import AlaveteliAPIError

try:
    request = client.get_request(12345)
except AlaveteliAPIError as e:
    print(f"API Error: {e}")
    print(f"Status Code: {e.status_code}")
except Exception as e:
    print(f"Request failed: {e}")
```

**Common Error Codes:**
- `400` - Bad Request (invalid parameters)
- `401` - Unauthorized (missing/invalid API key)
- `403` - Forbidden (API key lacks permission)
- `404` - Not Found (request doesn't exist)
- `429` - Too Many Requests (rate limited)
- `500` - Internal Server Error
- `503` - Service Unavailable

---

## Machine Tags

Alaveteli supports machine tags with colon notation:

```python
# Standard tags
tags = ['spending', '2026']

# Machine tags (preserve colon)
tags = [
    'spending_id:12345',
    'department:treasury',
    'year:2026',
    'uri:https://example.com/doc/456'
]

result = client.create_request(
    title='Spending Request',
    body='...',
    external_user_name='User',
    external_url='https://example.com',
    tags=tags
)
```

---

## Compatible Instances

```python
# FYI.org.nz (New Zealand)
fyi_client = create_fyi_client(api_key='...')

# WhatDoTheyKnow.com (United Kingdom)
wdtk_client = create_alaveteli_client(
    base_url='https://www.whatdotheyknow.com',
    api_key='...'
)

# FragDenStaat.de (Germany)
fds_client = create_alaveteli_client(
    base_url='https://fragdenstaat.de',
    api_key='...'
)

# Custom instance
custom_client = create_alaveteli_client(
    base_url='https://your-alaveteli-instance.org',
    api_key='...'
)
```

---

## Best Practices

### 1. API Key Security

```python
import os

# Store API key in environment variable
api_key = os.environ.get('ALAVETELI_API_KEY')
client = create_fyi_client(api_key=api_key)

# Or use secure credential storage
from fyi_system.credentials import get_fyi_credentials

creds = get_fyi_credentials('fyi-api')
if creds:
    client = create_fyi_client(api_key=creds.api_token)
```

### 2. Rate Limiting

The client sends the fyi-cli identity User-Agent on every request and captures
`RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`, and
`Retry-After` response headers. A `429` honours `Retry-After` (delta seconds or
HTTP date) when present and otherwise uses bounded exponential backoff. Keep
bulk work paginated, checkpointed, and explicitly scoped to small windows.

The Rust `fyi-core` path additionally provides adaptive pacing, per-instance
load memory, guardrails, a filesystem GET cache, and JSONL traces; the Python
client is retained as a compatibility path for legacy discovery/capture.

### 3. Error Recovery

```python
from requests.exceptions import RetryError

max_retries = 3
for attempt in range(max_retries):
    try:
        request = client.get_request(12345)
        break
    except AlaveteliAPIError as e:
        if e.status_code >= 500:
            time.sleep(2 ** attempt)  # Exponential backoff
        else:
            raise
```

### 4. Attachment Handling

```python
from pathlib import Path

# Validate attachments before sending
attachment_paths = []
for path in ['/doc1.pdf', '/doc2.pdf']:
    if Path(path).exists():
        attachment_paths.append(path)
    else:
        print(f"Warning: {path} not found")

client.add_correspondence(
    request_id=12345,
    direction='response',
    body='See attached',
    attachment_paths=attachment_paths
)
```

---

## Testing

```python
# Run Alaveteli client tests
pytest tests/test_alaveteli_client.py -v

# Test specific functionality
pytest tests/test_alaveteli_client.py::TestReadAPI -v
pytest tests/test_alaveteli_client.py::TestWriteAPI -v
```

---

## API Key Acquisition

To use the Write API, you need an API key from your Alaveteli instance administrator:

1. **FYI.org.nz**: Contact Transparency International New Zealand
2. **WhatDoTheyKnow.com**: Contact mySociety
3. **Other instances**: Check admin interface or contact administrator

API keys are typically provided via:
- Admin interface on authority pages
- Direct request to instance administrator
- Application process for bulk access

---

## Troubleshooting

### "API key required for Write API"

You're trying to use Write API without authentication:

```python
# Fix: Provide API key
client = create_fyi_client(api_key='your-api-key')
```

### "404 Not Found"

Request doesn't exist or wrong endpoint:

```python
# Check request ID
try:
    request = client.get_request(12345)
except AlaveteliAPIError as e:
    if e.status_code == 404:
        print("Request not found - check ID")
```

### "503 Service Unavailable"

Alaveteli instance is temporarily down:

```python
# Retry with backoff
import time
for attempt in range(3):
    try:
        health = client.check_api_health()
        break
    except AlaveteliAPIError:
        time.sleep(2 ** attempt)
```

---

## See Also

- [Alaveteli API Documentation](https://alaveteli.org/docs/developers/api/)
- [FYI.org.nz](https://fyi.org.nz)
- [WhatDoTheyKnow.com](https://www.whatdotheyknow.com)
- [FragDenStaat.de](https://fragdenstaat.de)
