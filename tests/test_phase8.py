from pathlib import Path

from fyi_system.db import init_db, insert_tracked_request, connect
from fyi_system.importers import import_authorities_csv
from fyi_system.fetch import latest_snapshot_summary
from fyi_system.webapp import _render_request_detail


def _seed_authority(db: Path):
    csv_path = db.parent / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)


def test_latest_snapshot_summary_extracts_attachments_and_events(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    payload = {
        "title": "Example request",
        "described_state": "successful",
        "attachments": [
            {"name": "release.pdf", "url": "https://example.com/release.pdf", "content_type": "application/pdf"}
        ],
        "events": [
            {"event_type": "response", "created_at": "2026-03-08T12:00:00", "title": "Agency responded"}
        ]
    }
    conn = connect(db)
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', __import__('json').dumps(payload)))
    conn.commit(); conn.close()
    summary = latest_snapshot_summary(db, 42)
    assert summary is not None
    assert summary['attachments_count'] == 1
    assert summary['events_count'] == 1
    assert summary['attachments'][0]['name'] == 'release.pdf'


def test_render_request_detail_shows_snapshot_sections(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='submitted', fyi_request_id=42, fyi_url='https://fyi.org.nz/request/42')
    conn = connect(db)
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', '{"title":"Title","described_state":"successful","attachments":[{"name":"file.pdf","url":"https://example.com/file.pdf"}],"events":[{"event_type":"response","created_at":"2026-03-08T12:00:00","title":"Response received"}]}'))
    conn.commit(); conn.close()
    html = _render_request_detail(str(db), rid)
    assert 'Latest FYI snapshot' in html
    assert 'Attachments' in html
    assert 'Snapshot events' in html
    assert 'file.pdf' in html
