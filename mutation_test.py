#!/usr/bin/env python3
"""Mutation testing script for FYI Request System.

Performs mutation testing on Python source files by introducing simple
mutations (boolean, comparison, None) and running the test suite to
determine if mutations are caught.

Run with:
    python mutation_test.py [--threshold SCORE]

Exit codes:
    0 - Mutation score meets or exceeds threshold
    1 - Mutation score below threshold or tests fail on original code
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import List, Tuple


# Simple mutations to try
MUTATIONS: List[Tuple[str, str, str]] = [
    # Boolean mutations
    (r"\bTrue\b", "False", "bool_true_to_false"),
    (r"\bFalse\b", "True", "bool_false_to_true"),
    # Comparison mutations
    (r"==", "!=", "eq_to_neq"),
    (r"!=", "==", "neq_to_eq"),
    # None mutations
    (r"\bis None\b", "is not None", "is_none_to_is_not_none"),
    (r"\bis not None\b", "is None", "is_not_none_to_is_none"),
]

DEFAULT_THRESHOLD = 70.0

# Tests to exclude from mutation test runs (slow, network, or flaky)
EXCLUDED_TESTS = [
    "--ignore=tests/test_scheduler_internals.py",
    "--ignore=tests/test_e2e_cli.py",
    "--ignore=tests/test_e2e_workflows.py",
    "--ignore=tests/test_api_contract.py",
]
EXCLUDED_KEYWORDS = "not (SecureFileOperations or DataIntegrity)"


def find_python_files(src_dir: Path) -> List[Path]:
    """Find all Python files in source directory."""
    return sorted(src_dir.glob("**/*.py"))


def read_file(path: Path) -> str:
    """Read file content."""
    return path.read_text(encoding="utf-8")


def write_file(path: Path, content: str) -> None:
    """Write file content."""
    path.write_text(content, encoding="utf-8")


def apply_mutation(
    content: str, pattern: str, replacement: str
) -> Tuple[str, int]:
    """Apply a single mutation to content. Returns mutated content and count."""
    mutated, count = re.subn(pattern, replacement, content, count=1)
    return mutated, count


def run_tests() -> bool:
    """Run test suite. Returns True if tests pass."""
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "pytest",
            "tests/",
            *EXCLUDED_TESTS,
            "-x",
            "-q",
            "--tb=short",
            "-k",
            EXCLUDED_KEYWORDS,
        ],
        capture_output=True,
        text=True,
        timeout=300,
    )
    if result.returncode != 0:
        # Keep the failing baseline or mutant's diagnostics in the CI evidence.
        # The return-code classification and mutation threshold are unchanged.
        if result.stdout:
            print(result.stdout, end="", flush=True)
        if result.stderr:
            print(result.stderr, end="", file=sys.stderr, flush=True)
    return result.returncode == 0


def mutate_and_test(
    file_path: Path,
    original_content: str,
    pattern: str,
    replacement: str,
    mutation_name: str,
) -> bool:
    """Apply mutation and run tests. Returns True if mutation was caught."""
    try:
        mutated, count = apply_mutation(original_content, pattern, replacement)
        if count == 0:
            return False

        write_file(file_path, mutated)

        try:
            tests_passed = run_tests()
        finally:
            write_file(file_path, original_content)

        return not tests_passed
    except subprocess.TimeoutExpired:
        write_file(file_path, original_content)
        return False
    except Exception:
        write_file(file_path, original_content)
        return False


def main() -> int:
    """Run mutation testing."""
    parser = argparse.ArgumentParser(
        description="Run mutation testing on Python source files"
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=DEFAULT_THRESHOLD,
        help=f"Minimum mutation score percentage (default: {DEFAULT_THRESHOLD})",
    )
    args = parser.parse_args()

    src_dir = Path("src/fyi_system")
    test_dir = Path("tests")

    if not src_dir.exists():
        print(f"Error: Source directory {src_dir} not found")
        return 1
    if not test_dir.exists():
        print(f"Error: Test directory {test_dir} not found")
        return 1

    print("Verifying tests pass on original code...")
    if not run_tests():
        print("ERROR: Tests fail on original code! Fix tests first.")
        return 1
    print("[OK] Original tests pass\n")

    python_files = find_python_files(src_dir)
    print(f"Found {len(python_files)} Python files to mutate\n")

    total_mutations = 0
    caught_mutations = 0
    survived_mutations = 0
    per_file_results: list[dict] = []

    for file_path in python_files:
        original_content = read_file(file_path)
        file_caught = 0
        file_total = 0

        for pattern, replacement, mutation_name in MUTATIONS:
            if not re.search(pattern, original_content):
                continue

            file_total += 1
            total_mutations += 1

            caught = mutate_and_test(
                file_path, original_content, pattern, replacement, mutation_name
            )

            if caught:
                file_caught += 1
                caught_mutations += 1
                print(f"  [CAUGHT] {file_path.name}:{mutation_name}")
            else:
                survived_mutations += 1
                print(f"  [SURVIVED] {file_path.name}:{mutation_name}")

        if file_total > 0:
            file_score = (file_caught / file_total) * 100
            per_file_results.append(
                {
                    "file": str(file_path),
                    "mutations": file_total,
                    "caught": file_caught,
                    "survived": file_total - file_caught,
                    "score": file_score,
                }
            )
        print()

    print("=" * 60)
    print("MUTATION TESTING RESULTS")
    print("=" * 60)
    print(f"Total mutations attempted: {total_mutations}")
    print(f"Mutations caught: {caught_mutations}")
    print(f"Mutations survived: {survived_mutations}")

    if total_mutations > 0:
        mutation_score = (caught_mutations / total_mutations) * 100
        print(f"\nMutation score: {mutation_score:.1f}% (threshold: {args.threshold}%)")

        if mutation_score >= args.threshold:
            print("[PASS] Mutation score meets threshold.")
        else:
            print(
                f"[FAIL] Mutation score {mutation_score:.1f}% below threshold "
                f"{args.threshold}%."
            )
            return 1

        if per_file_results:
            print("\nPer-file results:")
            for result in sorted(
                per_file_results, key=lambda r: r["score"]
            ):
                flag = "  " if result["score"] >= args.threshold else "!!"
                print(
                    f"{flag} {result['file']}: "
                    f"{result['score']:.1f}% "
                    f"({result['caught']}/{result['mutations']})"
                )
    else:
        print("\nNo mutations could be applied.")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
