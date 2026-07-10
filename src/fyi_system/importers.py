from __future__ import annotations

import csv
import io
import os
from pathlib import Path

import httpx

from .agent_runtime import build_user_agent
from .db import connect

DEFAULT_AUTHORITIES_URL = "https://fyi.org.nz/body/all-authorities.csv"

# Cryptographic-aligned identity; set FYI_ADMIN_CONTACT for opt-in operator contact.
USER_AGENT = build_user_agent(
    os.environ.get("FYI_ADMIN_CONTACT"), component="authority-import"
)


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
