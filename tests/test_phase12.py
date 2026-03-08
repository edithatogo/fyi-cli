from pathlib import Path

from fyi_system.db import init_db, query_all, insert_tracked_request, connect
from fyi_system.reporting import next_best_action, correspondence_pack, write_correspondence_pack_markdown
from fyi_system.webapp import make_handler


def seed_snapshot(db_path: Path, request_id: int, payload: str):
    conn = connect(db_path)
    try:
        conn.execute(
            "INSERT INTO request_snapshots(fyi_request_id, source_url, raw_json) VALUES (?, ?, ?)",
            (request_id, f"https://fyi.org.nz/request/{request_id}.json", payload),
        )
        conn.commit()
    finally:
        conn.close()


def test_next_best_action_and_correspondence_pack(tmp_path: Path):
    db = tmp_path / "test.db"
    init_db(db)
    conn = connect(db)
    conn.execute("INSERT INTO authorities(slug, name, url) VALUES ('test_auth', 'Test Authority', 'https://example.com')")
    conn.commit()
    conn.close()
    rid = insert_tracked_request(db, 'test_auth', 'Title', 'Body', status='submitted', fyi_request_id=55)
    seed_snapshot(db, 55, '{"title":"Title","described_state":"partially_successful","attachments":[{"name":"doc.pdf","url":"https://example.com/doc.pdf"}]}')

    nba = next_best_action(db, rid)
    assert nba['action_bucket'] == 'review_release'
    assert nba['recommended_strategy'] == 'review_released_material'

    pack = correspondence_pack(db, rid)
    assert 'review_released_material' in pack['strategies']

    out = tmp_path / 'pack.md'
    write_correspondence_pack_markdown(out, db, rid)
    text = out.read_text(encoding='utf-8')
    assert 'Correspondence pack' in text
    assert 'review_released_material' in text


def test_detail_and_correspondence_routes_render(tmp_path: Path):
    db = tmp_path / "web.db"
    init_db(db)
    conn = connect(db)
    conn.execute("INSERT INTO authorities(slug, name, url) VALUES ('test_auth', 'Test Authority', 'https://example.com')")
    conn.commit()
    conn.close()
    rid = insert_tracked_request(db, 'test_auth', 'Title', 'Body', status='draft', fyi_request_id=77)

    Handler = make_handler(str(db))
    assert hasattr(Handler, 'do_GET')
    # route smoke checks via render helpers are enough to catch the former detail/edit shadowing issue
    from fyi_system.webapp import _render_request_detail, _render_request_form, _render_correspondence
    assert 'Next best action' in _render_request_detail(str(db), rid)
    assert 'Edit tracked request' in _render_request_form(str(db), query_all(db, 'SELECT * FROM tracked_requests WHERE id=?', (rid,))[0])
    assert 'Correspondence pack' in _render_correspondence(str(db), rid)
