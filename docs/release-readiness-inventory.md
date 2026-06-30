# Release Readiness Inventory

## Summary

- Surfaces scanned: 11
- Missing surfaces: 0
- Issues found: 17
- High severity: 6
- Medium severity: 11

## Surfaces

- `README.md`: present
- `INSTALL.md`: present
- `QUICKSTART.md`: present
- `RELEASE_PLAN.md`: present
- `GITHUB_SETUP.md`: present
- `CHANGELOG.md`: present
- `Cargo.toml`: present
- `pyproject.toml`: present
- `.github/workflows/ci.yml`: present
- `.github/workflows/release.yml`: present
- `.github/workflows/release-please.yml`: present

## Issues

- `placeholder_repository_url` (high) at `README.md:8`: Release-facing file still references placeholder URL 'github.com/yourusername/fyi-cli'.
- `placeholder_repository_url` (high) at `README.md:9`: Release-facing file still references placeholder URL 'codecov.io/gh/yourusername/fyi-cli'.
- `placeholder_repository_url` (high) at `INSTALL.md:107`: Release-facing file still references placeholder URL 'github.com/yourusername/fyi-cli'.
- `legacy_python_command` (medium) at `INSTALL.md:92`: Release-facing file still presents legacy Python-era command text matching 'fyi-system'.
- `legacy_python_command` (medium) at `INSTALL.md:173`: Release-facing file still presents legacy Python-era command text matching 'fyi-system'.
- `placeholder_repository_url` (high) at `QUICKSTART.md:211`: Release-facing file still references placeholder URL 'github.com/yourusername/fyi-cli'.
- `legacy_python_command` (medium) at `QUICKSTART.md:17`: Release-facing file still presents legacy Python-era command text matching 'fyi-system'.
- `legacy_python_command` (medium) at `RELEASE_PLAN.md:180`: Release-facing file still presents legacy Python-era command text matching 'fyi-system'.
- `legacy_python_command` (medium) at `RELEASE_PLAN.md:394`: Release-facing file still presents legacy Python-era command text matching 'fyi-system.'.
- `legacy_python_command` (medium) at `RELEASE_PLAN.md:69`: Release-facing file still presents legacy Python-era command text matching 'build/fyi-system'.
- `placeholder_repository_url` (high) at `CHANGELOG.md:261`: Release-facing file still references placeholder URL 'github.com/yourusername/fyi-cli'.
- `legacy_python_command` (medium) at `CHANGELOG.md:58`: Release-facing file still presents legacy Python-era command text matching 'fyi-system'.
- `placeholder_repository_url` (high) at `pyproject.toml:64`: Release-facing file still references placeholder URL 'github.com/yourusername/fyi-cli'.
- `legacy_python_command` (medium) at `pyproject.toml:73`: Release-facing file still presents legacy Python-era command text matching 'fyi-system'.
- `rust_release_command_missing` (medium) at `release surfaces`: Release docs do not mention required check: `cargo +stable-x86_64-pc-windows-gnu fmt --all -- --check`.
- `rust_release_command_missing` (medium) at `release surfaces`: Release docs do not mention required check: `cargo +stable-x86_64-pc-windows-gnu clippy --workspace --all-targets --all-features -- -D warnings`.
- `rust_release_command_missing` (medium) at `release surfaces`: Release docs do not mention required check: `cargo +stable-x86_64-pc-windows-gnu test --workspace --all-features`.

## Required Rust Release Checks

- `cargo +stable-x86_64-pc-windows-gnu fmt --all -- --check`
- `cargo +stable-x86_64-pc-windows-gnu clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo +stable-x86_64-pc-windows-gnu test --workspace --all-features`
