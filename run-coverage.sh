#!/usr/bin/env bash
# run-coverage.sh
# Runs cargo-llvm-cov to ensure code coverage meets the >90% threshold.

set -euo pipefail

echo "Running Cargo LLVM Cov to audit coverage (>90% threshold)..."

# Run coverage check with fail threshold
cargo llvm-cov --all-features --workspace --fail-under-lines 90 --html

echo "Coverage check passed! Coverage is above 90%."
