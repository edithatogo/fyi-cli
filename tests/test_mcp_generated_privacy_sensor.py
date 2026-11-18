"""Negative controls prevent escaped private strings from defeating the sensors."""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import pytest

from tests.test_mcp_generated_contracts import PRIVATE, _assert_omits, _assert_private_diagnostics

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.parametrize("marker", ["folder\\private.txt", "private\0file", 'private\nquote"'])
def test_decoded_result_sensor_rejects_escaped_second_content_leaks(marker: str) -> None:
    serialized = json.dumps(
        {"content": [{"text": "safe error"}, {"text": marker}], "isError": True},
    )
    # The decoded sensor must detect values the old raw-substring check missed.
    assert marker not in serialized
    with pytest.raises(AssertionError, match="result disclosed"):
        _assert_omits(json.loads(serialized), marker)
    _assert_omits({"content": [{"text": "safe error"}]}, marker)


def test_diagnostics_sensor_rejects_json_escaped_body_leaks(tmp_path: Path) -> None:
    marker = PRIVATE + 'body\nwith\rquotes"and\\slashes'
    diagnostics = json.dumps({"body": marker})
    (tmp_path / "upstream-server.stderr").write_text(diagnostics, encoding="utf-8")
    assert marker not in diagnostics
    with pytest.raises(AssertionError, match="diagnostics disclosed"):
        _assert_private_diagnostics(tmp_path, PRIVATE)
