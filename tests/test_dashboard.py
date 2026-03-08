from pathlib import Path
from fyi_system.db import init_db
from fyi_system.dashboard import dashboard_payload

def test_dashboard_payload(tmp_path: Path):
    db = tmp_path / 'test.db'
    init_db(db)
    payload = dashboard_payload(db)
    assert payload['summary']['total'] == 0
