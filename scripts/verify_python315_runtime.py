"""Fail closed unless a Python CI operation uses the pinned repo-local 3.15.1."""

from __future__ import annotations

import sys
from pathlib import Path

REQUIRED_VERSION = (3, 15, 1)


def main() -> int:
    if sys.version_info[:3] != REQUIRED_VERSION:
        raise SystemExit("Python runtime must be CPython 3.15.1")
    if sys.implementation.name != "cpython":
        raise SystemExit("Unsupported Python implementation")
    repository = Path(__file__).resolve().parents[1]
    expected = (repository / ".venv").resolve()
    if sys.prefix == sys.base_prefix or Path(sys.prefix).resolve() != expected:
        raise SystemExit("Python runtime must use repository-local .venv")
    print("Verified repository-local CPython 3.15.1")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
