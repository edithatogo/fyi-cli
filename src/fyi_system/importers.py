from __future__ import annotations

import csv
import hashlib
import io
import os
from datetime import UTC, datetime
from pathlib import Path
from urllib.parse import urlsplit

import httpx

from .agent_runtime import build_user_agent
from .db import connect
from .discovery import (
    PoliteRateLimiter,
    SharedRateLimiter,
    client,
    get_with_backoff,
    load_robots_disallow,
)

DEFAULT_AUTHORITIES_URL = "https://fyi.org.nz/body/all-authorities.csv"

# Cryptographic-aligned identity; set FYI_ADMIN_CONTACT for opt-in operator contact.
USER_AGENT = build_user_agent(
    os.environ.get("FYI_ADMIN_CONTACT"), component="authority-import",
)


def authorities_url(base_url: str) -> str:
    """Build the public Alaveteli authorities CSV URL."""
    return f"{base_url.rstrip('/')}/body/all-authorities.csv"


def _value(row: dict, *keys: str) -> str:
    for key in keys:
        value = row.get(key)
        if value is not None:
            value = str(value).strip()
            if value:
                return value
    return ""


def import_authorities_rows(
    rows: list[dict[str, str]],
    db_path: str | Path = "fyi_system.db",
) -> int:
    """Upsert authority rows into the local database."""
    conn = connect(db_path)
    count = 0
    try:
        for row in rows:
            slug = _value(row, "url_name", "slug", "authority_slug")
            name = _value(row, "name", "authority_name", "public_body_name")
            url = _value(row, "url", "authority_url", "request_url") or None
            if not slug or not name:
                continue
            conn.execute(
                """
                INSERT INTO authorities(slug, name, url)
                VALUES (?, ?, ?)
                ON CONFLICT(slug) DO UPDATE SET
                    name=excluded.name,
                    url=excluded.url
                """,
                (slug, name, url),
            )
            count += 1
        conn.commit()
        return count
    finally:
        conn.close()


def parse_authorities_csv(csv_text: str) -> list[dict[str, str]]:
    """Parse an FYI authorities CSV payload."""
    return list(csv.DictReader(io.StringIO(csv_text.lstrip("\ufeff"))))


def import_authorities_csv(csv_path: str | Path, db_path: str | Path = "fyi_system.db") -> int:
    with Path(csv_path).open(newline="", encoding="utf-8-sig") as f:
        return import_authorities_rows(list(csv.DictReader(f)), db_path=db_path)


def import_authorities_url(
    source_url: str = DEFAULT_AUTHORITIES_URL,
    db_path: str | Path = "fyi_system.db",
    *,
    transport: httpx.BaseTransport | None = None,
) -> int:
    """Fetch and import authorities from the official FYI authorities CSV."""
    with httpx.Client(
        headers={"User-Agent": USER_AGENT},
        timeout=30,
        transport=transport,
    ) as client:
        response = client.get(source_url)
        response.raise_for_status()
    return import_authorities_rows(parse_authorities_csv(response.text), db_path=db_path)


def discover_bodies(
    *,
    base_url: str = "https://fyi.org.nz",
    delay_seconds: float = 1.0,
    shared_rate_limit_db_path: str | Path | None = None,
    shared_rate_limit_name: str = "authority-discovery",
    transport: httpx.BaseTransport | None = None,
) -> list[dict[str, str]]:
    """Discover public authorities without mutating the local database."""
    rows, _ = discover_bodies_with_provenance(
        base_url=base_url,
        catalog_url=None,
        delay_seconds=delay_seconds,
        shared_rate_limit_db_path=shared_rate_limit_db_path,
        shared_rate_limit_name=shared_rate_limit_name,
        transport=transport,
    )
    return rows


def discover_bodies_with_provenance(
    *,
    base_url: str = "https://fyi.org.nz",
    catalog_url: str | None = None,
    delay_seconds: float = 1.0,
    shared_rate_limit_db_path: str | Path | None = None,
    shared_rate_limit_name: str = "authority-discovery",
    transport: httpx.BaseTransport | None = None,
) -> tuple[list[dict[str, str]], dict[str, str | int]]:
    """Discover bodies and return auditable HTTP/payload provenance.

    ``base_url`` remains the capture/instance URL.  ``catalog_url`` is an exact
    URL override and therefore uses its own origin for robots and rate limiting.
    """
    effective_catalog_url = catalog_url or authorities_url(base_url)
    parsed = urlsplit(effective_catalog_url)
    if parsed.scheme not in {"http", "https"} or not parsed.netloc:
        message = "catalog_url must be an absolute http(s) URL"
        raise ValueError(message)
    catalog_origin = f"{parsed.scheme}://{parsed.netloc}"
    with client(catalog_origin, transport=transport) as http:
        disallows = load_robots_disallow(http)
        shared_limiter = (
            SharedRateLimiter(shared_rate_limit_db_path, name=shared_rate_limit_name)
            if shared_rate_limit_db_path is not None
            else None
        )
        response = get_with_backoff(
            http,
            effective_catalog_url,
            disallows=disallows,
            shared_rate_limiter=shared_limiter,
            rate_limiter=PoliteRateLimiter(delay_seconds),
            backoff_seconds=delay_seconds,
        )
        response.raise_for_status()
        payload = response.content
        rows = parse_authorities_csv(payload.decode("utf-8-sig"))
        provenance: dict[str, str | int] = {
            "catalog_url": effective_catalog_url,
            "retrieval_mode": "override" if catalog_url else "default",
            "retrieved_at": datetime.now(UTC).isoformat(),
            "response_status": response.status_code,
            "payload_sha256": hashlib.sha256(payload).hexdigest(),
            "row_count": len(rows),
        }
        return rows, provenance
