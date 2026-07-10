from __future__ import annotations
import json
import socket
from pathlib import Path
import feedparser  # type: ignore[import-untyped]
from .db import connect
from .fyi import extract_request_id

def ingest_feed(feed_url: str, db_path: str | Path = 'fyi_system.db') -> int:
    previous_timeout = socket.getdefaulttimeout()
    socket.setdefaulttimeout(2.0)
    try:
        parsed = feedparser.parse(feed_url)
    finally:
        socket.setdefaulttimeout(previous_timeout)
    conn = connect(db_path)
    count = 0
    try:
        for entry in parsed.entries:
            link = getattr(entry, 'link', None)
            request_id = extract_request_id(link or '')
            raw = json.dumps({k: entry.get(k) for k in entry.keys()}, ensure_ascii=False)
            conn.execute(
                '''INSERT INTO feed_events(feed_url, event_id, title, link, published, summary, request_id_guess, raw_json)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)''',
                (feed_url, entry.get('id'), entry.get('title'), link, entry.get('published'), entry.get('summary'), request_id, raw),
            )
            count += 1
        conn.commit()
        return count
    finally:
        conn.close()

def reconcile_events(db_path: str | Path = 'fyi_system.db') -> int:
    conn = connect(db_path)
    matched = 0
    try:
        rows = conn.execute(
            'SELECT id, request_id_guess, title FROM feed_events WHERE tracked_request_id IS NULL AND request_id_guess IS NOT NULL'
        ).fetchall()
        for row in rows:
            tr = conn.execute('SELECT id FROM tracked_requests WHERE fyi_request_id = ?', (row['request_id_guess'],)).fetchone()
            if tr:
                conn.execute('UPDATE feed_events SET tracked_request_id = ? WHERE id = ?', (tr['id'], row['id']))
                conn.execute('UPDATE tracked_requests SET last_event_title = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?', (row['title'], tr['id']))
                matched += 1
        conn.commit()
        return matched
    finally:
        conn.close()
