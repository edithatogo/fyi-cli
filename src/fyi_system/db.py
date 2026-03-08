from __future__ import annotations
import sqlite3
from pathlib import Path
from typing import Iterable, Any
import json
from .security import ensure_private_path

SCHEMA = """
PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS authorities (
  slug TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT
);
CREATE TABLE IF NOT EXISTS tracked_requests (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  authority_slug TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  tags TEXT DEFAULT '',
  fyi_url TEXT,
  fyi_request_id INTEGER,
  status TEXT DEFAULT 'draft',
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
  last_seen_at TEXT,
  last_event_title TEXT,
  FOREIGN KEY(authority_slug) REFERENCES authorities(slug)
);
CREATE TABLE IF NOT EXISTS feed_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  feed_url TEXT NOT NULL,
  event_id TEXT,
  title TEXT,
  link TEXT,
  published TEXT,
  summary TEXT,
  request_id_guess INTEGER,
  tracked_request_id INTEGER,
  raw_json TEXT,
  seen_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS request_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  fyi_request_id INTEGER NOT NULL,
  source_url TEXT NOT NULL,
  raw_json TEXT NOT NULL,
  fetched_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS run_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_name TEXT NOT NULL,
  status TEXT NOT NULL,
  detail TEXT,
  ran_at TEXT DEFAULT CURRENT_TIMESTAMP
);
"""


def connect(db_path: str | Path = 'fyi_system.db') -> sqlite3.Connection:
    ensure_private_path(Path(db_path).parent, is_dir=True)
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    return conn


def init_db(db_path: str | Path = 'fyi_system.db') -> None:
    conn = connect(db_path)
    try:
        conn.executescript(SCHEMA)
        conn.commit()
        ensure_private_path(db_path, is_dir=False)
    finally:
        conn.close()


def query_all(db_path: str | Path, sql: str, params: Iterable[Any] = ()):
    conn = connect(db_path)
    try:
        return conn.execute(sql, tuple(params)).fetchall()
    finally:
        conn.close()


def insert_tracked_request(db_path: str | Path, authority_slug: str, title: str, body: str, tags: str = '', status: str = 'draft', fyi_request_id: int | None = None, fyi_url: str | None = None) -> int:
    conn = connect(db_path)
    try:
        cur = conn.execute(
            'INSERT INTO tracked_requests(authority_slug, title, body, tags, status, fyi_request_id, fyi_url) VALUES (?, ?, ?, ?, ?, ?, ?)',
            (authority_slug, title, body, tags, status, fyi_request_id, fyi_url),
        )
        conn.commit()
        return int(cur.lastrowid)
    finally:
        conn.close()


def update_tracked_request(
    db_path: str | Path,
    request_id: int,
    *,
    authority_slug: str,
    title: str,
    body: str,
    tags: str = '',
    status: str = 'draft',
    fyi_request_id: int | None = None,
    fyi_url: str | None = None,
) -> None:
    conn = connect(db_path)
    try:
        conn.execute(
            """
            UPDATE tracked_requests
            SET authority_slug=?, title=?, body=?, tags=?, status=?, fyi_request_id=?, fyi_url=?, updated_at=CURRENT_TIMESTAMP
            WHERE id=?
            """,
            (authority_slug, title, body, tags, status, fyi_request_id, fyi_url, request_id),
        )
        conn.commit()
    finally:
        conn.close()


def get_tracked_request(db_path: str | Path, request_id: int):
    rows = query_all(db_path, 'SELECT * FROM tracked_requests WHERE id=?', (request_id,))
    return rows[0] if rows else None


def update_request_status(db_path: str | Path, request_id: int, status: str) -> None:
    conn = connect(db_path)
    try:
        conn.execute(
            "UPDATE tracked_requests SET status=?, updated_at=CURRENT_TIMESTAMP WHERE id=?",
            (status, request_id),
        )
        conn.commit()
    finally:
        conn.close()


def request_timeline(db_path: str | Path, tracked_request_id: int):
    tracked = get_tracked_request(db_path, tracked_request_id)
    if not tracked:
        return []
    items: list[dict[str, Any]] = []
    if tracked['created_at']:
        items.append({
            'ts': tracked['created_at'],
            'kind': 'tracked_request',
            'title': 'Tracked request created',
            'detail': tracked['title'],
        })
    if tracked['updated_at'] and tracked['updated_at'] != tracked['created_at']:
        items.append({
            'ts': tracked['updated_at'],
            'kind': 'tracked_request',
            'title': 'Tracked request updated',
            'detail': f"Status: {tracked['status']}",
        })
    events = query_all(db_path, 'SELECT seen_at, published, title, summary, link FROM feed_events WHERE tracked_request_id=? ORDER BY COALESCE(published, seen_at) DESC, id DESC', (tracked_request_id,))
    for row in events:
        items.append({
            'ts': row['published'] or row['seen_at'],
            'kind': 'feed_event',
            'title': row['title'] or 'Feed event',
            'detail': row['summary'] or row['link'] or '',
        })
    if tracked['fyi_request_id'] is not None:
        snaps = query_all(db_path, 'SELECT fetched_at, raw_json FROM request_snapshots WHERE fyi_request_id=? ORDER BY fetched_at DESC, id DESC', (tracked['fyi_request_id'],))
        for row in snaps:
            try:
                payload = json.loads(row['raw_json'])
            except Exception:
                payload = {}
            title = payload.get('title') or payload.get('info_request', {}).get('title') or 'Request snapshot'
            state = payload.get('described_state') or payload.get('info_request', {}).get('described_state') or ''
            items.append({
                'ts': row['fetched_at'],
                'kind': 'request_snapshot',
                'title': title,
                'detail': state,
            })
    items.sort(key=lambda x: (x['ts'] or ''), reverse=True)
    return items


def export_tracked_requests(db_path: str | Path, output_path: str | Path) -> Path:
    rows = [dict(r) for r in query_all(db_path, 'SELECT * FROM tracked_requests ORDER BY id')]
    out = Path(output_path)
    ensure_private_path(out.parent, is_dir=True)
    out.write_text(json.dumps(rows, indent=2, ensure_ascii=False), encoding='utf-8')
    ensure_private_path(out, is_dir=False)
    return out


def import_tracked_requests(db_path: str | Path, input_path: str | Path, *, replace: bool = False) -> int:
    payload = json.loads(Path(input_path).read_text(encoding='utf-8'))
    if not isinstance(payload, list):
        raise ValueError('Expected a JSON list of tracked requests')
    conn = connect(db_path)
    try:
        if replace:
            conn.execute('DELETE FROM tracked_requests')
        count = 0
        for item in payload:
            conn.execute(
                'INSERT INTO tracked_requests(authority_slug, title, body, tags, fyi_url, fyi_request_id, status, created_at, updated_at, last_seen_at, last_event_title) VALUES (?, ?, ?, ?, ?, ?, ?, COALESCE(?, CURRENT_TIMESTAMP), COALESCE(?, CURRENT_TIMESTAMP), ?, ?)',
                (
                    item.get('authority_slug'), item.get('title'), item.get('body'), item.get('tags', ''), item.get('fyi_url'), item.get('fyi_request_id'), item.get('status', 'draft'),
                    item.get('created_at'), item.get('updated_at'), item.get('last_seen_at'), item.get('last_event_title')
                )
            )
            count += 1
        conn.commit()
        return count
    finally:
        conn.close()
