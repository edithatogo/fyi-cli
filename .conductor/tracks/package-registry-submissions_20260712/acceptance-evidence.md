# Packaging and distribution acceptance evidence

## Scope and baseline

Review date: 14 November 2026. Existing issue
[#194](https://github.com/edithatogo/fyi-cli/issues/194) remains open for external
release/publication/verification requirements. The work uses the existing track;
no duplicate track or release authority was created. Initial inspected source:
`a816dcd425a7b6277761a18faeae1685202eefde` (PR #431), reconciled against
`797e76cf120988372f4df1f2efb9c9e164c69050` (quality PR #433). Installer and integrity
changes are reviewed in [PR #439](https://github.com/edithatogo/fyi-cli/pull/439).

Application functionality, acquisition, MCP application code, localization, TUI
and shared workflows are outside this workstream. The later metadata slice only
touches crate `package.metadata.binstall`, not application code or dependencies.

## Slice 1 — installer and integrity controls

The old asdf plugin guessed unsupported platforms, could fall back to Linux
amd64, downloaded/extracted unverified archives and offered unqualified Cargo
fallbacks. It now requires a committed per-version checksum approval, exact
supported platform, bounded download/decompression, a single regular root
executable, a matching version in a clean temporary home and atomic installation.
It has no approved versions yet. mise documentation no longer suggests the
colliding public crate or a main repository with no plugin-root layout.

The existing release-manifest helper now streams hashes and rejects empty,
unsafe or ambiguous directories. It validates full source commit and SemVer,
binds the exact canonical manifest bytes in SHA256SUMS, and has a read-only
verification mode that rejects altered/missing/extra assets. It does not claim
signature, SBOM, reproducibility or native-install verification.

| Verification | Result |
| --- | --- |
| asdf regressions on old implementation | 20 failed, 2 passed |
| Independent-review regressions before fixes | 10 failed, 29 passed |
| final focused asdf installer tests | 39 passed |
| integrity regressions on old implementation | 41 failed, 5 passed |
| final focused release-manifest tests | 49 passed |
| Combined focused tests | 88 passed |
| Script syntax, scoped Ruff lint/format, scoped diff whitespace | Passed |
| Actual plugin `list-all` | Successful empty list; no unqualified version exposed |
| Native macOS/Windows install and authenticated provenance | Not run; explicitly blocked |

Exact focused command:

```bash
python -m pytest -q tests/test_asdf_packaging.py tests/test_release_manifest.py
```

The installer tests use synthetic local archives and mocked download commands.
They demonstrate installer behavior, not native macOS package acceptance. The
integrity tests validate bytes and evidence structure, not origin authenticity.
Independent review additionally required curl 8.4.0 or newer for a bounded
download and complete bounded gzip decompression before parsing a single ordinary
tar member. Concatenated archives, hidden tails, extended headers and invalid
gzip trailers fail without replacing an existing installation.

## Remaining gates

The local metadata/ledger/native-source-template reconciliation is the next
focused slice. Public GitHub `v1.2.0` contains Rust component `0.1.2`; its eight
observed assets do not include an aggregate release manifest or SBOM. Release
version authority, crate names, actual provenance, native builders/install
transcripts, owner accounts and external review remain explicit operator gates.
No external submission, credential change, live-install result or publication is
asserted by this evidence.
