from __future__ import annotations
from urllib.parse import urlencode, urlparse
import re
from typing import Iterable

FYI_BASE = "https://fyi.org.nz"

def build_prefilled_url(authority_slug: str, title: str, body: str, tags: Iterable[str] | None = None, base_url: str = FYI_BASE) -> str:
    params = {"title": title, "body": body}
    joined = ",".join(tags or [])
    if joined:
        params["tags"] = joined
    return f"{base_url.rstrip('/')}/new/{authority_slug}?{urlencode(params)}"

def extract_request_id(url: str) -> int | None:
    parsed = urlparse(url)
    match = re.search(r"/request/(\d+)", parsed.path)
    return int(match.group(1)) if match else None
