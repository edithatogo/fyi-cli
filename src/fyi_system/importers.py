from __future__ import annotations
import csv
from pathlib import Path
from .db import connect


def _value(row: dict, *keys: str) -> str:
    for key in keys:
        value = row.get(key)
        if value is not None:
            value = str(value).strip()
            if value:
                return value
    return ''


def import_authorities_csv(csv_path: str | Path, db_path: str | Path = 'fyi_system.db') -> int:
    conn = connect(db_path)
    count = 0
    try:
        with open(csv_path, newline='', encoding='utf-8-sig') as f:
            reader = csv.DictReader(f)
            for row in reader:
                slug = _value(row, 'url_name', 'slug', 'authority_slug')
                name = _value(row, 'name', 'authority_name', 'public_body_name')
                url = _value(row, 'url', 'authority_url', 'request_url') or None
                if not slug or not name:
                    continue
                conn.execute(
                    'INSERT INTO authorities(slug, name, url) VALUES (?, ?, ?) ON CONFLICT(slug) DO UPDATE SET name=excluded.name, url=excluded.url',
                    (slug, name, url),
                )
                count += 1
        conn.commit()
        return count
    finally:
        conn.close()
