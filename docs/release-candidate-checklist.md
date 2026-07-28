# Release Candidate Checklist

## Local Verification

Run these checks from the repository root on Windows with the GNU Rust toolchain:

```powershell
cargo +stable-x86_64-pc-windows-gnu fmt --all -- --check
cargo +stable-x86_64-pc-windows-gnu clippy --workspace --all-targets --all-features -- -D warnings
cargo +stable-x86_64-pc-windows-gnu test --workspace --all-features
cargo +stable-x86_64-pc-windows-gnu build --release --locked --package fyi-cli
.\.venv\Scripts\python.exe -m pytest tests/test_release_readiness.py
.\.venv\Scripts\python.exe scripts\release_readiness.py --json
```

## Artifact Hygiene

- Confirm `git status --short` is clean before tagging.
- Remove generated profiler files such as `crates/fyi-cli/dhat-heap.json`.
- Keep publishing credentials out of local config and logs.
- Keep `CITATION.cff`, `.zenodo.json`, and
  `artifacts/release/zenodo-mirror-manifest.json` aligned to the same released version.
- Leave `concept_doi` and `version_doi` as `null` in the mirror manifest until the live Zenodo
  record is verified for the exact release tag.

## Secret-Gated Steps

- Crates.io publishing requires `CARGO_REGISTRY_TOKEN`.
- GitHub release creation uses the repository `GITHUB_TOKEN`.
- Codecov upload uses `CODECOV_TOKEN` and is allowed to fail without blocking CI.

## Remaining Release Blockers

- Fix any issues reported in `docs/release-readiness-inventory.md`.
- Re-run the local verification commands after documentation or workflow changes.
- Use live FYI.org.nz smoke tests only with explicit opt-in and polite rate limits.
- For each release, verify GitHub→Zenodo archiving in Zenodo, then update `concept_doi`,
  `version_doi`, `verified_at`, and `verification_status` in
  `artifacts/release/zenodo-mirror-manifest.json`.
