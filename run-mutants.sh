#!/usr/bin/env bash
# run-mutants.sh
# Runs cargo-mutants for mutation testing audits

set -euo pipefail

echo "Running cargo-mutants mutation audit..."

# Execute cargo-mutants on the workspace
cargo mutants --workspace --all-features
