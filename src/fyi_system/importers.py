from __future__ import annotations

import csv
import io
from pathlib import Path
from typing import TYPE_CHECKING

import httpx

from .db import connect
from .discovery import (
    PoliteRateLimiter,
    SharedRateLimiter,
    client,
    get_with_backoff,
    load_robots_disallow,
)

if TYPE_CHECKING:
    from collections.abc import Mapping

DEFAULT_AUTHORITIES_URL = "https://fyi.org.nz/body/all-authorities.csv"
USER_AGENT = "fyi-cli authority-import/1.0 (+https://github.com/edithatogo/fyi-cli)"


def _value(row: Mapping[str, object], *keys: str) -> str:
    for key in keys:
        value = row.get(key)
        if value is not None:
            value = str(value).strip()
            if value:
                return value
    return ""


def authorities_url(base_url: str) -> str:
    """Build the public Alaveteli authority catalogue URL."""
    return f"{base_url.rstrip('/')}/body/all-authorities.csv"


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
    shared_rate_limit_name: str = "archive-discovery",
    transport: httpx.BaseTransport | None = None,
) -> list[dict[str, str]]:
    """Discover public bodies without mutating the local database."""
    with client(base_url, transport=transport) as http:
        disallows = load_robots_disallow(http)
        shared_rate_limiter = (
            SharedRateLimiter(shared_rate_limit_db_path, name=shared_rate_limit_name)
            if shared_rate_limit_db_path is not None
            else None
        )
        rate_limiter = PoliteRateLimiter(delay_seconds)
        response = get_with_backoff(
            http,
            authorities_url(base_url),
            disallows=disallows,
            shared_rate_limiter=shared_rate_limiter,
            rate_limiter=rate_limiter,
            backoff_seconds=delay_seconds,
        )
        response.raise_for_status()
        return parse_authorities_csv(response.text)
