from pathlib import Path
import json

from fyi_system.db import init_db, insert_tracked_request, connect
from fyi_system.importers import import_authorities_csv
from fyi_system.reporting import follow_up_variants, response_analysis, write_attachment_manifest_csv
from fyi_system.webapp import _render_request_detail


def _seed_authority(db: Path):
    csv_path = db.parent / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)


def test_follow_up_variants_and_analysis(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='submitted', fyi_request_id=42, fyi_url='https://fyi.org.nz/request/42')
    payload = {
        "title": "Title",
        "described_state": "partially_successful",
        "attachments": [{"name": "file.pdf", "url": "https://example.com/file.pdf"}],
        "events": [{"event_type": "response", "created_at": "2026-03-08T12:00:00", "title": "Response received"}]
    }
    conn = connect(db)
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', json.dumps(payload)))
    conn.commit(); conn.close()
    variants = follow_up_variants(db, rid)
    assert len(variants['variants']) >= 3
    assert any(v['strategy'] == 'review_released_material' for v in variants['variants'])
    analysis = response_analysis(db, rid)
    assert analysis['likely_response_received'] is True
    assert analysis['likely_incomplete'] is True
    html = _render_request_detail(str(db), rid)
    assert 'Alternative follow-up variants' in html
    assert 'Response analysis' in html


def test_attachment_manifest_csv(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='submitted', fyi_request_id=42)
    payload = {"title": "Title", "described_state": "successful", "attachments": [{"name": "file.pdf", "url": "https://example.com/file.pdf"}]}
    conn = connect(db)
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', json.dumps(payload)))
    conn.commit(); conn.close()
    out = tmp_path / 'attachments.csv'
    write_attachment_manifest_csv(out, db, rid)
    content = out.read_text(encoding='utf-8')
    assert 'file.pdf' in content
    assert 'tracked_request_id' in content
