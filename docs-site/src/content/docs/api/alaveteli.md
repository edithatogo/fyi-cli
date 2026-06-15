---
title: Alaveteli Client API
description: Usage documentation for the Alaveteli API client.
---

## Overview

The `alaveteli_client` module provides a comprehensive client for the Alaveteli API, supporting both public reading and write-authenticated requests.

---

## 1. Quick Start

### 1.1 Reading Requests

No API key is required to query public request threads:

```python
from fyi_system.alaveteli_client import create_fyi_client

client = create_fyi_client()
request = client.get_request(12345)
print(request.title)
```

### 1.2 Creating Requests

Creating a new request requires providing your API key during initialization:

```python
client = create_fyi_client(api_key="your-api-key")
client.create_request(
    title="OIA request regarding...",
    body="Under the Official Information Act...",
    tags=["official-information"]
)
```
