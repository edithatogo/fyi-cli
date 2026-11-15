"""Exercise the audit entry point and parser used by the locked security gate."""

from __future__ import annotations

import os
import subprocess
import sys
from importlib import import_module
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.security
def test_locked_audit_cli_starts_on_the_selected_interpreter(tmp_path: Path) -> None:
    """An import crash must fail the required test suite before audit execution."""
    result = subprocess.run(
        [sys.executable, "-I", "-m", "pip_audit", "--version"],
        cwd=tmp_path,
        env={
            **os.environ,
            "PIP_DISABLE_PIP_VERSION_CHECK": "1",
            "PIP_NO_INDEX": "1",
        },
        capture_output=True,
        text=True,
        timeout=15,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip().startswith("pip-audit ")


@pytest.mark.security
def test_audit_parser_preserves_pinned_versions_and_hashes(tmp_path: Path) -> None:
    """Parse a local export-shaped requirement without resolving or downloading it."""
    pip_api = import_module("pip_api")

    requirement = tmp_path / "requirements.txt"
    digest = "0" * 64
    requirement.write_text(
        f"# Synthetic audit input: café, Māori, 日本語\npip-api==0.0.35 --hash=sha256:{digest}\n",
        encoding="utf-8",
    )
    parsed = pip_api.parse_requirements(str(requirement))
    assert set(parsed) == {"pip-api"}
    assert str(parsed["pip-api"].specifier) == "==0.0.35"
    assert parsed["pip-api"].hashes == {"sha256": [digest]}
