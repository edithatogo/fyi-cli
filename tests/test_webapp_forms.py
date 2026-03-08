from pathlib import Path
from fyi_system.db import init_db, query_all, insert_tracked_request, update_tracked_request
from fyi_system.dashboard import dashboard_payload
from fyi_system.importers import import_authorities_csv
from fyi_system.webapp import _render_request_form, _render_authorities


def test_importer_accepts_alternative_headers(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    csv_path = tmp_path / 'authorities.csv'
    csv_path.write_text('authority_slug,authority_name,authority_url\nalpha,Alpha Authority,https://example.com\n', encoding='utf-8')
    count = import_authorities_csv(csv_path, db)
    assert count == 1
    rows = query_all(db, 'SELECT slug, name, url FROM authorities')
    assert rows[0]['slug'] == 'alpha'
    assert rows[0]['name'] == 'Alpha Authority'


def test_insert_and_update_request_helpers(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    csv_path = tmp_path / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)
    request_id = insert_tracked_request(db, 'alpha', 'Title', 'Body', tags='tag:a', status='draft')
    update_tracked_request(db, request_id, authority_slug='alpha', title='Title 2', body='Body 2', tags='tag:b', status='submitted', fyi_request_id=12, fyi_url='https://fyi.org.nz/request/12')
    row = query_all(db, 'SELECT * FROM tracked_requests WHERE id=?', (request_id,))[0]
    assert row['title'] == 'Title 2'
    assert row['status'] == 'submitted'
    assert row['fyi_request_id'] == 12


def test_render_request_form_contains_prefilled_link(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    csv_path = tmp_path / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)
    request_id = insert_tracked_request(db, 'alpha', 'Title', 'Body', tags='tag:a', status='draft')
    row = query_all(db, 'SELECT * FROM tracked_requests WHERE id=?', (request_id,))[0]
    html = _render_request_form(str(db), row)
    assert 'Open FYI prefilled draft' in html
    assert 'alpha' in html


def test_dashboard_payload_recent_updates_key(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    csv_path = tmp_path / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)
    insert_tracked_request(db, 'alpha', 'Title', 'Body')
    payload = dashboard_payload(db)
    assert 'recent_updates' in payload['summary']


def test_render_authorities_search(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    csv_path = tmp_path / 'authorities.csv'
    csv_path.write_text('slug,name\nalpha,Alpha Authority\nbeta,Beta Board\n', encoding='utf-8')
    import_authorities_csv(csv_path, db)
    html = _render_authorities(str(db), q='beta')
    assert 'Beta Board' in html
