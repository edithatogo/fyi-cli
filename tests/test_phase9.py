from pathlib import Path
import json

from fyi_system.db import init_db, insert_tracked_request, connect
from fyi_system.importers import import_authorities_csv
from fyi_system.reporting import suggest_follow_up, attachment_manifest
from fyi_system.webapp import _render_request_detail


def _seed_authority(db: Path):
    csv_path = db.parent / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)


def test_follow_up_draft_changes_with_snapshot(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='submitted', fyi_request_id=42, fyi_url='https://fyi.org.nz/request/42')
    payload = {
        "title": "Title",
        "described_state": "successful",
        "attachments": [{"name": "file.pdf", "url": "https://example.com/file.pdf"}],
    }
    conn = connect(db)
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', json.dumps(payload)))
    conn.commit(); conn.close()
    draft = suggest_follow_up(db, rid)
    assert draft['stage'] == 'review_response'
    assert 'full response' in draft['body']


def test_attachment_manifest_and_detail_page(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='submitted', fyi_request_id=42, fyi_url='https://fyi.org.nz/request/42')
    payload = {
        "title": "Title",
        "described_state": "successful",
        "attachments": [{"name": "file.pdf", "url": "https://example.com/file.pdf"}],
        "events": [{"event_type": "response", "created_at": "2026-03-08T12:00:00", "title": "Response received"}]
    }
    conn = connect(db)
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', json.dumps(payload)))
    conn.commit(); conn.close()
    manifest = attachment_manifest(db, rid)
    assert manifest['attachments_count'] == 1
    html = _render_request_detail(str(db), rid)
    assert 'Suggested follow-up draft' in html
    assert 'file.pdf' in html
