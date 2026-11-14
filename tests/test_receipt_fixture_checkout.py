"""Byte evidence must survive Git check-in/out with Windows-style conversion."""
from __future__ import annotations

import os
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
def test_git_preserves_acquisition_fixture_bytes(
    tmp_path: Path, name: str, payload: bytes
) -> None:
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)

    def git(*args: str) -> bytes:
        return subprocess.run(
            ["git", "-c", f"core.attributesFile={os.devnull}", *args],
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
    relative = f"conformance/acquisition/{name}"
    path = tmp_path / relative
    path.parent.mkdir(parents=True)
    path.write_bytes(payload)
    git("add", "--", ".gitattributes", relative)
    assert git("check-attr", "-z", "text", "--", relative).endswith(b"\x00unset\x00")
    assert git("show", f":{relative}") == payload
    path.unlink()
    git("checkout-index", "--force", "--", relative)
    assert path.read_bytes() == payload
