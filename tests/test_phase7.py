from pathlib import Path
import json

from fyi_system.db import init_db, insert_tracked_request, query_all, export_tracked_requests, import_tracked_requests, request_timeline, connect
from fyi_system.importers import import_authorities_csv
from fyi_system.dashboard import dashboard_payload
from fyi_system.webapp import _render_requests, _render_timeline


def _seed_authority(db: Path):
    csv_path = db.parent / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)


def test_bulk_export_import_requests(tmp_path: Path):
    db = tmp_path / 'a.db'
    init_db(db)
    _seed_authority(db)
    insert_tracked_request(db, 'alpha', 'One', 'Body one', tags='t:1', status='draft')
    insert_tracked_request(db, 'alpha', 'Two', 'Body two', tags='t:2', status='submitted')
    out = tmp_path / 'requests.json'
    export_tracked_requests(db, out)
    payload = json.loads(out.read_text(encoding='utf-8'))
    assert len(payload) == 2

    db2 = tmp_path / 'b.db'
    init_db(db2)
    _seed_authority(db2)
    count = import_tracked_requests(db2, out)
    assert count == 2
    rows = query_all(db2, 'SELECT title, status FROM tracked_requests ORDER BY id')
    assert rows[1]['title'] == 'Two'
    assert rows[1]['status'] == 'submitted'


def test_request_timeline_includes_feed_and_snapshot(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='open', fyi_request_id=42)
    conn = connect(db)
    conn.execute("INSERT INTO feed_events(feed_url, event_id, title, link, published, summary, request_id_guess, tracked_request_id, raw_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)", ('https://example.com/feed', 'e1', 'Agency replied', 'https://fyi.org.nz/request/42', '2026-03-08T10:00:00', 'summary', 42, rid, '{}'))
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', '{"title":"Title","described_state":"successful"}'))
    conn.commit(); conn.close()
    tl = request_timeline(db, rid)
    titles = [x['title'] for x in tl]
    assert 'Agency replied' in titles
    assert 'Title' in titles


def test_render_requests_has_inline_status_and_timeline(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    insert_tracked_request(db, 'alpha', 'Title', 'Body', status='draft')
    html = _render_requests(str(db))
    assert '/status' in html
    assert 'Timeline' in html


def test_render_timeline_page(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='draft')
    html = _render_timeline(str(db), rid)
    assert 'Timeline for request' in html


def test_dashboard_items_include_updated_at(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    insert_tracked_request(db, 'alpha', 'Title', 'Body')
    payload = dashboard_payload(db)
    assert 'updated_at' in payload['items'][0]
