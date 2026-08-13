"""Run one named Atheris target with caller-supplied libFuzzer bounds."""

from __future__ import annotations

import argparse
import sys

import atheris

with atheris.instrument_imports():
    from fuzzing.targets import TARGETS


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True, choices=sorted(TARGETS))
    args, libfuzzer_args = parser.parse_known_args()
    sys.argv = [sys.argv[0], *libfuzzer_args]
    atheris.Setup(sys.argv, TARGETS[args.target])
    atheris.Fuzz()


if __name__ == "__main__":
    main()
