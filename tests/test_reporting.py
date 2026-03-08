from pathlib import Path
from fyi_system.db import init_db, connect
from fyi_system.reporting import attention_report, build_handover_markdown

def test_attention_report(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    conn = connect(db)
    conn.execute("INSERT INTO authorities(slug, name) VALUES ('a', 'Authority A')")
    conn.execute("INSERT INTO tracked_requests(authority_slug, title, body, status) VALUES ('a', 'T1', 'B1', 'draft')")
    conn.commit(); conn.close()
    report = attention_report(db)
    assert report['count'] == 1
    assert report['items'][0]['needs_attention'] is True
    assert 'FYI Request System Handover' in build_handover_markdown(db)
