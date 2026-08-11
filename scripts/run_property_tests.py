#!/usr/bin/env python3
"""Run the repository's explicit property-based test layer."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMMAND_TIMEOUT_SECONDS = 600


def command_environment(command: tuple[str, ...]) -> dict[str, str]:
    environment = os.environ.copy()
    if os.name == "nt" and command and command[0] == "cargo":
        environment.setdefault("RUSTUP_TOOLCHAIN", "stable-x86_64-pc-windows-gnu")
        environment.setdefault("CARGO_BUILD_TARGET", "x86_64-pc-windows-gnu")
        linker = Path.home() / "scoop/apps/mingw/current/bin/x86_64-w64-mingw32-gcc.exe"
        if linker.exists():
            environment.setdefault("CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER", str(linker))
    return environment


def run(command: tuple[str, ...]) -> int:
    print(f"Running property-test command: {' '.join(command)}", flush=True)
    try:
        completed = subprocess.run(  # noqa: S603
            command,
            cwd=ROOT,
            check=False,
            env=command_environment(command),
            timeout=COMMAND_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        print(
            f"Property-test command timed out after {COMMAND_TIMEOUT_SECONDS} seconds.",
            file=sys.stderr,
        )
        return 1
    return completed.returncode


def main() -> int:
    commands = (
        (sys.executable, "-m", "pytest", "tests/test_hypothesis.py", "tests/test_fuzz.py"),
        ("cargo", "test", "-p", "fyi-core", "--test", "property_tests"),
    )
    for command in commands:
        if run(command):
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
