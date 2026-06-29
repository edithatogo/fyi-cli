"""Tests for authority import helpers."""

from __future__ import annotations

from typing import TYPE_CHECKING

import httpx

from fyi_system.db import init_db, query_all
from fyi_system.importers import (
    import_authorities_csv,
    import_authorities_rows,
    import_authorities_url,
    parse_authorities_csv,
)

if TYPE_CHECKING:
    from pathlib import Path


def test_parse_authorities_csv_accepts_fyi_columns() -> None:
    rows = parse_authorities_csv(
        "url_name,name,url\nagency,Agency,https://fyi.example/body/agency\n",
    )

    assert rows == [
        {
            "url_name": "agency",
            "name": "Agency",
            "url": "https://fyi.example/body/agency",
        },
    ]


def test_import_authorities_rows_upserts_idempotently(tmp_path: Path) -> None:
    db = tmp_path / "fyi.db"
    init_db(db)

    count = import_authorities_rows(
        [
            {
                "url_name": "agency",
                "name": "Agency",
                "url": "https://fyi.example/body/agency",
            },
        ],
        db_path=db,
    )
    updated = import_authorities_rows(
        [
            {
                "url_name": "agency",
                "name": "Agency Renamed",
                "url": "https://fyi.example/body/agency-renamed",
            },
        ],
        db_path=db,
    )

    rows = query_all(db, "SELECT slug, name, url FROM authorities")
    assert count == 1
    assert updated == 1
    assert [dict(row) for row in rows] == [
        {
            "slug": "agency",
            "name": "Agency Renamed",
            "url": "https://fyi.example/body/agency-renamed",
        },
    ]


def test_import_authorities_csv_uses_local_file(tmp_path: Path) -> None:
    db = tmp_path / "fyi.db"
    csv_path = tmp_path / "authorities.csv"
    init_db(db)
    csv_path.write_text(
        "authority_slug,authority_name,authority_url\n"
        "local,Local Agency,https://fyi.example/body/local\n",
        encoding="utf-8",
    )

    count = import_authorities_csv(csv_path, db_path=db)

    rows = query_all(db, "SELECT slug, name, url FROM authorities")
    assert count == 1
    assert rows[0]["slug"] == "local"


def test_import_authorities_url_fetches_official_csv(tmp_path: Path) -> None:
    db = tmp_path / "fyi.db"
    init_db(db)

    def handler(request: httpx.Request) -> httpx.Response:
        assert request.url == "https://fyi.example/body/all-authorities.csv"
        assert request.headers["user-agent"].startswith("fyi-cli authority-import")
        return httpx.Response(
            200,
            text="url_name,name,url\nremote,Remote Agency,https://fyi.example/body/remote\n",
            request=request,
        )

    count = import_authorities_url(
        "https://fyi.example/body/all-authorities.csv",
        db_path=db,
        transport=httpx.MockTransport(handler),
    )

    rows = query_all(db, "SELECT slug, name, url FROM authorities")
    assert count == 1
    assert rows[0]["name"] == "Remote Agency"
