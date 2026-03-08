
import json
from pathlib import Path
from fyi_system.db import init_db, insert_tracked_request, connect
from fyi_system.fetch import summarize_request_json, latest_snapshot_summary
from fyi_system.reporting import next_best_action, export_request_bundle
from fyi_system.webapp import _render_request_detail, _render_recommended_draft


def test_normalize_varied_snapshot_shapes(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    conn = connect(db)
    try:
        conn.execute("INSERT INTO authorities(slug, name) VALUES ('ministry', 'Ministry')")
        conn.execute(
            "INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)",
            (77, 'https://example.test/request/77.json', json.dumps({
                'request': {
                    'id': 77,
                    'title': 'Alt payload',
                    'state': 'response available',
                    'url_title': 'alt-payload',
                },
                'documents': [
                    {'display_name': 'reply.pdf', 'attachment_url': 'https://example.test/reply.pdf', 'mime_type': 'application/pdf'}
                ],
                'history': [
                    {'event_type': 'response', 'details': 'Reply uploaded', 'occurred_at': '2026-03-08T12:00:00'}
                ]
            }))
        )
        conn.commit()
    finally:
        conn.close()
    summary = latest_snapshot_summary(db, 77)
    assert summary['title'] == 'Alt payload'
    assert summary['described_state'] == 'response available'
    assert summary['attachments_count'] == 1
    assert summary['events_count'] == 1


def test_export_bundle(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    conn = connect(db)
    try:
        conn.execute("INSERT INTO authorities(slug, name) VALUES ('ministry', 'Ministry')")
        conn.execute(
            "INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)",
            (1, 'https://example.test/request/1.json', json.dumps({'title':'Sample','described_state':'awaiting response'}))
        )
        conn.commit()
    finally:
        conn.close()
    request_id = insert_tracked_request(db, 'ministry', 'Sample', 'Please provide ...', status='submitted', fyi_request_id=1)
    out_dir = export_request_bundle(tmp_path / 'bundle', db, request_id)
    expected = {
        'correspondence-pack.json', 'correspondence-pack.md', 'attachment-manifest.json', 'attachment-manifest.csv',
        'response-analysis.json', 'next-best-action.json', 'request-detail.json', 'manifest.json'
    }
    assert expected.issubset({p.name for p in out_dir.iterdir()})


def test_request_detail_has_open_recommended_draft(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    conn = connect(db)
    try:
        conn.execute("INSERT INTO authorities(slug, name) VALUES ('ministry', 'Ministry')")
        conn.commit()
    finally:
        conn.close()
    request_id = insert_tracked_request(db, 'ministry', 'Sample', 'Please provide ...', status='submitted', fyi_request_id=None)
    html = _render_request_detail(str(db), request_id)
    assert 'Open recommended draft' in html
    draft_html = _render_recommended_draft(str(db), request_id)
    assert 'Recommended draft for request' in draft_html
