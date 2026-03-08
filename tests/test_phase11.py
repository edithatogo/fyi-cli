from pathlib import Path
import json

from fyi_system.db import init_db, insert_tracked_request, connect
from fyi_system.importers import import_authorities_csv
from fyi_system.reporting import follow_up_pack, triage_report, response_analysis
from fyi_system.dashboard import dashboard_payload
from fyi_system.webapp import _render_request_detail, _render_requests


def _seed_authority(db: Path):
    csv_path = db.parent / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)


def test_follow_up_pack_and_triage(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='submitted', fyi_request_id=42)
    payload = {
        'title': 'Title',
        'described_state': 'successful',
        'attachments': [{'name': 'file.pdf', 'url': 'https://example.com/file.pdf'}],
    }
    conn = connect(db)
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', json.dumps(payload)))
    conn.commit(); conn.close()

    pack = follow_up_pack(db, rid)
    assert len(pack['items']) == 9
    assert {item['tone'] for item in pack['items']} == {'neutral', 'warm', 'firm'}

    analysis = response_analysis(db, rid)
    assert analysis['normalized_snapshot_state'] == 'responded_full'
    assert analysis['priority'] == 'now'

    triage = triage_report(db)
    assert triage['summary']['action_now'] == 1
    dash = dashboard_payload(db)
    assert dash['summary']['action_now'] == 1


def test_webapp_shows_pack_and_priority_filter(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    _seed_authority(db)
    rid = insert_tracked_request(db, 'alpha', 'Title', 'Body', status='submitted', fyi_request_id=42)
    payload = {'title': 'Title', 'described_state': 'rejected'}
    conn = connect(db)
    conn.execute("INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)", (42, 'https://fyi.org.nz/request/42.json', json.dumps(payload)))
    conn.commit(); conn.close()

    detail = _render_request_detail(str(db), rid)
    assert 'Strategy and tone pack' in detail
    requests_now = _render_requests(str(db), priority='now')
    assert 'Title' in requests_now
