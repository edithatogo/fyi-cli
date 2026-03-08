from __future__ import annotations
import json
import time
from pathlib import Path
from .db import connect
from .monitor import ingest_feed, reconcile_events
from .reporting import write_attention_report, write_handover

def run_cycle(feed_url: str, db_path: str | Path = 'fyi_system.db', outputs_dir: str | Path = 'outputs') -> dict:
    out = Path(outputs_dir)
    out.mkdir(parents=True, exist_ok=True)
    ingested = ingest_feed(feed_url, db_path=db_path)
    matched = reconcile_events(db_path=db_path)
    write_attention_report(out / 'attention-report.json', db_path=db_path)
    write_handover(out / 'handover.md', db_path=db_path)
    result = {'ingested': ingested, 'matched': matched, 'outputs_dir': str(out)}
    conn = connect(db_path)
    try:
        conn.execute('INSERT INTO run_log(job_name, status, detail) VALUES (?, ?, ?)', ('run_cycle', 'ok', json.dumps(result)))
        conn.commit()
    finally:
        conn.close()
    return result

def run_scheduler(feed_url: str, interval_seconds: int = 3600, db_path: str | Path = 'fyi_system.db', outputs_dir: str | Path = 'outputs', once: bool = False) -> None:
    while True:
        run_cycle(feed_url=feed_url, db_path=db_path, outputs_dir=outputs_dir)
        if once:
            break
        time.sleep(interval_seconds)
