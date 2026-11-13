"""Keep credential test storage private without adding deps to metadata-only tests."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

KEYRING_TEST_MODULES = frozenset(
    {"test_credentials.py", "test_encryption.py", "test_keyring_isolation.py"},
)


@pytest.fixture(autouse=True)
def isolated_test_keyring(request: pytest.FixtureRequest, tmp_path: Path) -> Iterator[None]:
    """Use the real file backend at a unique per-test path, also under xdist."""
    if request.node.path.name not in KEYRING_TEST_MODULES:
        yield
        return

    # Metadata-only CI installs pytest alone. Load keyring dependencies only for
    # the credential suites, which install the project's development extra.
    import keyring
    from keyrings.alt.file import PlaintextKeyring

    previous = keyring.get_keyring()
    backend = PlaintextKeyring()
    backend.file_path = str(tmp_path / "keyring.cfg")
    keyring.set_keyring(backend)
    try:
        yield
    finally:
        keyring.set_keyring(previous)
