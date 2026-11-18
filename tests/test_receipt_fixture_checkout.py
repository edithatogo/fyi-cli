"""Byte evidence must survive Git check-in/out with Windows-style conversion."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]


@pytest.mark.parametrize(
    ("name", "payload"),
    [
        ("receipt.json", b'{"synthetic":true}\n'),
        ("source.json", b'{"synthetic":true}\r\n'),
        ("response.bin", b"synthetic\r\n\x00\xff\n"),
    ],
)
@pytest.mark.parametrize(
    "fixture_directory",
    [
        "conformance/acquisition",
        "conformance/internet-archive-replay",
        "tests/fixtures/internet_archive_replay",
    ],
)
def test_git_preserves_acquisition_fixture_bytes(
    tmp_path: Path,
    name: str,
    payload: bytes,
    fixture_directory: str,
) -> None:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
    executable = shutil.which("git")
    assert executable is not None

    def git(*args: str) -> bytes:
        return subprocess.run(  # noqa: S603 - Resolved Git, fixed test arguments, no shell.
            [executable, "-c", f"core.attributesFile={os.devnull}", *args],
            cwd=tmp_path,
            env=env,
            capture_output=True,
            check=True,
            timeout=15,
        ).stdout

    git("init", "--quiet")
    git("config", "core.autocrlf", "true")
    git("config", "core.safecrlf", "false")
    # Use repository policy, not an attribute synthesized by the test.
    (tmp_path / ".gitattributes").write_bytes((ROOT / ".gitattributes").read_bytes())
    relative = f"{fixture_directory}/{name}"
    path = tmp_path / relative
    path.parent.mkdir(parents=True)
    path.write_bytes(payload)
    git("add", "--", ".gitattributes", relative)
    assert git("check-attr", "-z", "text", "--", relative).endswith(b"\x00unset\x00")
    assert git("show", f":{relative}") == payload
    path.unlink()
    git("checkout-index", "--force", "--", relative)
    assert path.read_bytes() == payload
