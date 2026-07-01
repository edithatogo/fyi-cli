from __future__ import annotations
from datetime import datetime, timezone
import os
import random
import time
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
CREATE TABLE IF NOT EXISTS shared_rate_limit_state (
  name TEXT PRIMARY KEY,
  next_allowed_at REAL NOT NULL DEFAULT 0,
  last_acquired_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_owner_id TEXT NOT NULL DEFAULT '',
  last_sleep_seconds REAL NOT NULL DEFAULT 0,
  interval_seconds REAL NOT NULL DEFAULT 0,
  jitter_seconds REAL NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS shared_rate_limit_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  owner_id TEXT NOT NULL DEFAULT '',
  status_code INTEGER,
  delay_seconds REAL NOT NULL DEFAULT 0,
  next_allowed_at REAL NOT NULL DEFAULT 0,
  observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"""


def connect(db_path: str | Path = 'fyi_system.db') -> sqlite3.Connection:
    ensure_private_path(Path(db_path).parent, is_dir=True)
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA busy_timeout=5000")
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
        rowid = cur.lastrowid
        return int(rowid) if rowid is not None else -1
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


def _utc_now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _ensure_shared_rate_limit_tables(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS shared_rate_limit_state (
          name TEXT PRIMARY KEY,
          next_allowed_at REAL NOT NULL DEFAULT 0,
          last_acquired_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          last_owner_id TEXT NOT NULL DEFAULT '',
          last_sleep_seconds REAL NOT NULL DEFAULT 0,
          interval_seconds REAL NOT NULL DEFAULT 0,
          jitter_seconds REAL NOT NULL DEFAULT 0
        )
        """,
    )
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS shared_rate_limit_events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          kind TEXT NOT NULL,
          owner_id TEXT NOT NULL DEFAULT '',
          status_code INTEGER,
          delay_seconds REAL NOT NULL DEFAULT 0,
          next_allowed_at REAL NOT NULL DEFAULT 0,
          observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        """,
    )


def _reserve_shared_rate_limit(
    conn: sqlite3.Connection,
    *,
    name: str,
    interval_seconds: float,
    jitter_seconds: float = 0.25,
    owner_id: str | None = None,
    now: float | None = None,
    randomizer: Any = random.random,
    kind: str,
    status_code: int | None = None,
) -> dict[str, Any]:
    interval_seconds = max(float(interval_seconds), 0.0)
    jitter_seconds = max(float(jitter_seconds), 0.0)
    owner_id = owner_id or f"pid:{os.getpid()}"
    current_time = float(now if now is not None else time.time())
    jitter = jitter_seconds * float(randomizer()) if interval_seconds > 0 else 0.0

    conn.execute("BEGIN IMMEDIATE")
    row = conn.execute(
        "SELECT next_allowed_at FROM shared_rate_limit_state WHERE name = ?",
        (name,),
    ).fetchone()
    previous_next_allowed_at = float(row["next_allowed_at"]) if row else 0.0
    scheduled_at = max(current_time, previous_next_allowed_at)
    sleep_seconds = max(0.0, scheduled_at - current_time)
    next_allowed_at = scheduled_at + interval_seconds + jitter
    conn.execute(
        """
        INSERT INTO shared_rate_limit_state(
          name,
          next_allowed_at,
          last_acquired_at,
          last_owner_id,
          last_sleep_seconds,
          interval_seconds,
          jitter_seconds
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(name) DO UPDATE SET
          next_allowed_at=excluded.next_allowed_at,
          last_acquired_at=excluded.last_acquired_at,
          last_owner_id=excluded.last_owner_id,
          last_sleep_seconds=excluded.last_sleep_seconds,
          interval_seconds=excluded.interval_seconds,
          jitter_seconds=excluded.jitter_seconds
        """,
        (
            name,
            next_allowed_at,
            _utc_now_iso(),
            owner_id,
            sleep_seconds,
            interval_seconds,
            jitter_seconds,
        ),
    )
    conn.execute(
        """
        INSERT INTO shared_rate_limit_events(
          name,
          kind,
          owner_id,
          status_code,
          delay_seconds,
          next_allowed_at,
          observed_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        """,
        (
            name,
            kind,
            owner_id,
            status_code,
            sleep_seconds if kind == 'acquired' else interval_seconds,
            next_allowed_at,
            _utc_now_iso(),
        ),
    )
    return {
        "name": name,
        "owner_id": owner_id,
        "sleep_seconds": sleep_seconds,
        "next_allowed_at": next_allowed_at,
        "previous_next_allowed_at": previous_next_allowed_at,
        "interval_seconds": interval_seconds,
        "jitter_seconds": jitter_seconds,
        "kind": kind,
        "status_code": status_code,
    }


def acquire_shared_rate_limit(
    db_path: str | Path,
    *,
    name: str,
    interval_seconds: float,
    jitter_seconds: float = 0.25,
    owner_id: str | None = None,
    now: float | None = None,
    randomizer: Any = random.random,
) -> dict[str, Any]:
    """Atomically reserve the next slot in a shared rate limit."""
    conn = connect(db_path)
    try:
        _ensure_shared_rate_limit_tables(conn)
        conn.commit()
        result = _reserve_shared_rate_limit(
            conn,
            name=name,
            interval_seconds=interval_seconds,
            jitter_seconds=jitter_seconds,
            owner_id=owner_id,
            now=now,
            randomizer=randomizer,
            kind="acquired",
        )
        conn.commit()
        return result
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()


def record_shared_rate_limit_backoff(
    db_path: str | Path,
    *,
    name: str,
    delay_seconds: float,
    jitter_seconds: float = 0.25,
    owner_id: str | None = None,
    now: float | None = None,
    status_code: int | None = None,
    randomizer: Any = random.random,
) -> dict[str, Any]:
    """Advance the shared rate limit after a transient failure."""
    conn = connect(db_path)
    try:
        _ensure_shared_rate_limit_tables(conn)
        conn.commit()
        result = _reserve_shared_rate_limit(
            conn,
            name=name,
            interval_seconds=delay_seconds,
            jitter_seconds=jitter_seconds,
            owner_id=owner_id,
            now=now,
            randomizer=randomizer,
            kind="backoff",
            status_code=status_code,
        )
        conn.commit()
        return result
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()


def read_shared_rate_limit_state(db_path: str | Path, *, name: str) -> dict[str, Any] | None:
    """Read the current shared rate limit state snapshot."""
    conn = connect(db_path)
    try:
        _ensure_shared_rate_limit_tables(conn)
        conn.commit()
        row = conn.execute(
            """
            SELECT
              name,
              next_allowed_at,
              last_acquired_at,
              last_owner_id,
              last_sleep_seconds,
              interval_seconds,
              jitter_seconds
            FROM shared_rate_limit_state
            WHERE name = ?
            """,
            (name,),
        ).fetchone()
        return dict(row) if row else None
    finally:
        conn.close()


def read_shared_rate_limit_events(
    db_path: str | Path,
    *,
    name: str,
    limit: int = 10,
) -> list[dict[str, Any]]:
    """Return the most recent shared limiter events."""
    conn = connect(db_path)
    try:
        _ensure_shared_rate_limit_tables(conn)
        conn.commit()
        rows = conn.execute(
            """
            SELECT
              kind,
              owner_id,
              status_code,
              delay_seconds,
              next_allowed_at,
              observed_at
            FROM shared_rate_limit_events
            WHERE name = ?
            ORDER BY id DESC
            LIMIT ?
            """,
            (name, limit),
        ).fetchall()
        return [dict(row) for row in rows]
    finally:
        conn.close()
