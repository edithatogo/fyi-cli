"""Regression guard: credential tests must never share a persistent keyring."""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import keyring
import pytest
from keyrings.alt.file import PlaintextKeyring

if TYPE_CHECKING:
    from pathlib import Path as PathType


@pytest.mark.parametrize("case", [0, 1])
def test_each_test_has_private_keyring_storage(tmp_path: PathType, case: int) -> None:
    backend = keyring.get_keyring()
    assert isinstance(backend, PlaintextKeyring)
    assert Path(backend.file_path) == tmp_path / "keyring.cfg"
    # The same service/user pair deliberately repeats across test invocations.
    # Assert the private path before any write so a missing fixture cannot touch
    # the developer's ordinary keyring during the red regression run.
    service = "fyi-test-isolation"
    username = "repeated-user"
    assert keyring.get_password(service, username) is None
    keyring.set_password(service, username, str(case))
    assert keyring.get_password(service, username) == str(case)
    assert (tmp_path / "keyring.cfg").is_file()
