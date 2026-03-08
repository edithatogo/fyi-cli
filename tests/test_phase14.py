
import json
from pathlib import Path
from fyi_system.db import init_db, insert_tracked_request, connect
from fyi_system.reporting import export_request_bundle
from fyi_system.security import redact_text, privacy_audit


def test_redact_text_hides_email_and_query_secret():
    value = 'Contact me at alice@example.org and see https://example.org/x?token=abc123&ok=1'
    redacted = redact_text(value)
    assert '[redacted-email]' in redacted
    assert 'token=%5Bredacted%5D' in redacted
    assert 'ok=1' in redacted


def test_export_bundle_sanitizes_body_in_strict_mode(tmp_path: Path):
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
    request_id = insert_tracked_request(db, 'ministry', 'Sample', 'Please provide secret@example.org and token=abc', tags='topic:alpha', status='submitted', fyi_request_id=1)
    out_dir = export_request_bundle(tmp_path / 'bundle', db, request_id, sanitize=True, profile='strict')
    detail = json.loads((out_dir / 'request-detail.json').read_text(encoding='utf-8'))
    assert detail['correspondence_pack']['title'] == 'Sample'
    assert detail['correspondence_pack']['title'] == 'Sample'
    assert detail['correspondence_pack']['tracked_status'] == 'submitted'


def test_privacy_audit_flags_non_local_host(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    audit = privacy_audit(db, host='0.0.0.0', outputs_dir=tmp_path / 'outputs', profile='strict')
    host_check = next(item for item in audit['checks'] if item['name'] == 'bind_host_local_only')
    assert host_check['ok'] is False
