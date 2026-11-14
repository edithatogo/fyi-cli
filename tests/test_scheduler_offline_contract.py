"""Legacy scheduler tests must not use a network, even when errors are swallowed."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

from fyi_system.db import connect, init_db
from fyi_system.scheduler import run_cycle
from tests import scheduler_feed_fixture

local_scheduler_feed = scheduler_feed_fixture.local_scheduler_feed

ROOT = Path(__file__).resolve().parents[1]
SCHEDULER_MODULES = [
    "tests/test_scheduler-IASN2001657.py",
    "tests/test_scheduler.py",
    "tests/test_scheduler_internals.py",
]
PROBE = """
import socket
import sys
import pytest

attempts = []
def deny_network(*args, **kwargs):
    attempts.append(1)
    raise OSError("scheduler offline contract forbids network access")

# Install before test-module imports: block DNS and connections, including
# configured proxies. No address or argument is recorded in diagnostics.
socket.getaddrinfo = deny_network
socket.gethostbyname = deny_network
socket.gethostbyname_ex = deny_network
socket.gethostbyaddr = deny_network
socket.create_connection = deny_network
socket.socket.connect = deny_network
socket.socket.connect_ex = deny_network
socket.socket.sendto = deny_network

result = pytest.main([*sys.argv[1:], "-q"])
if attempts:
    print("scheduler network attempts blocked:", len(attempts))
    raise SystemExit(86)
raise SystemExit(result)
"""


@pytest.mark.security
@pytest.mark.regression
def test_legacy_scheduler_modules_are_offline_even_when_they_swallow_errors() -> None:
    environment = {
        name: os.environ[name]
        for name in ("PATH", "SYSTEMROOT", "WINDIR", "TEMP", "TMP")
        if name in os.environ
    }
    environment["PYTEST_DISABLE_PLUGIN_AUTOLOAD"] = "1"
    result = subprocess.run(  # noqa: S603 - fixed local test probe; all DNS/socket I/O denied.
        [sys.executable, "-c", PROBE, *SCHEDULER_MODULES],
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    assert result.returncode == 0, result.stdout[-5000:] + result.stderr[-1000:]
    assert "scheduler network attempts blocked:" not in result.stdout


@pytest.mark.regression
@pytest.mark.parametrize("url", ["https://example.invalid/feed", "https://invalid.invalid/feed"])
def test_synthetic_feed_preserves_real_local_cycle_outputs(
    tmp_path: Path,
    local_scheduler_feed: list[str],
    url: str,
) -> None:
    database = tmp_path / "scheduler.db"
    outputs = tmp_path / "outputs"
    init_db(database)
    result = run_cycle(url, db_path=database, outputs_dir=outputs)
    assert local_scheduler_feed == [url]
    assert result == {"ingested": 0, "matched": 0, "outputs_dir": str(outputs)}
    assert (outputs / "attention-report.json").is_file()
    assert (outputs / "handover.md").is_file()
    connection = connect(database)
    try:
        row = connection.execute("SELECT job_name, status FROM run_log").fetchone()
        assert tuple(row) == ("run_cycle", "ok")
    finally:
        connection.close()
