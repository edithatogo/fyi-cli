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
| final focused asdf installer tests | 40 passed |
| Real Scalene 2.4.0 focused profiling run | 40 passed; no tests skipped |
| integrity regressions on old implementation | 41 failed, 5 passed |
| final focused release-manifest tests | 49 passed |
| Combined focused tests | 89 passed |
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
The hosted profiling run exposed inherited Python-wrapper interference in the
test fixtures. A dedicated injected-wrapper regression failed before the fixture
was pinned to the underlying interpreter; normal and real Scalene runs now pass.
Installer behavior and shared workflows were unchanged by that test isolation fix.

## Slice 2 — release-bound packets and submission contracts

The public observation is captured in
[`packaging/release-evidence.json`](../../../packaging/release-evidence.json):
GitHub release `v1.2.0`, its full source commit, Rust component `0.1.2`, all eight
asset names/sizes/API digests and the absent `v0.1.2` tag/release. API observations
do not assert that this work downloaded, executed or authenticated those bytes.
The release authority and all existing package versions are unchanged.

Homebrew, Chocolatey, Scoop and WinGet packets now reference the actual CLI
archive names and observed SHA-256 values. Windows packets no longer substitute
an MCPB bundle for the CLI executable. Linux arm64 is not advertised by the
binary formula. Binstall defaults are disabled in the two crate metadata blocks
and the inventory until crate identity and archive mapping are qualified.

Source templates bind the historical full source commit and frozen Cargo inputs.
They require explicit source/vendor hashes, dependency and toolchain qualification,
propagated build/test failures and native package checks. Unresolved hash tokens
are visible blockers, never valid submission values. Snap requires qualification
of an offline compiler toolchain rather than assuming core22's default Rust can
build the pinned lockfile. Flathub is not applicable to this console application;
private Flatpak remains an optional blocked template.

The existing Packaging Assets entry point now parses package-native version
fields, checks release URL/hash/platform relationships, requires canonical packet
files and distinguishes repository consistency from submission readiness. The
fourteen smoke contracts use exact help commands and require bounded isolated
execution, version/commit/artifact evidence and hashed transcripts. Structured
operator verification records bind successful checks to the relevant subjects;
the validator does not perform signature authentication or native installations.

The registry ledger has dated evidence, route, responsible maintainer, packet
inventory, readiness and exact blockers for every requested target. Existing
MCP/AI registry rows were preserved. The runbook documents native qualification,
submission, update, withdrawal and rollback steps. No shared workflow was edited.

### Verification scope

| Final local verification | Result |
| --- | --- |
| Combined packaging, integrity, installer, source, preflight, freshness, release-scope and GHCR contract regressions | 185 passed |
| Parsed package/release contract validator | Passed; zero errors and warnings |
| Registry ledger | Passed; 28 total targets, including 14 requested distribution channels |
| Install-smoke contract validator | Passed; 14 contracts, no execution claim |
| Ordinary release preflight | Passed; `submission_ready: false` |
| Submission-readiness guards | Correctly rejected the current release; exit 1 |
| WinGet official singleton 1.6.0 JSON schema | Passed |
| Strict scoped Ruff lint/format | Passed |

Exact combined regression command:

```bash
python -m pytest -q tests/test_asdf_packaging.py tests/test_release_manifest.py \
  tests/test_distribution_contracts.py tests/test_source_packaging_contracts.py \
  tests/test_verify_packaging_assets.py tests/test_registry_submission_ledger.py \
  tests/test_install_smoke_matrix.py tests/test_release_preflight.py \
  tests/test_distribution_freshness.py tests/test_verify_release_scope.py \
  tests/test_ghcr_multarch.py
```

Independent review reproduced and then rejected the unrelated-platform smoke,
failed-plaintext receipt and omitted-canonical-manifest bypasses. Positive
synthetic Homebrew and native GHCR records exercise the ready path; they are
fixtures, not public/native-install evidence. The installer policy guards also
reject readiness with disabled Binstall or a missing/mutated asdf approval.

Regression tests demonstrate rejection of stale/comment-only versions, malformed
documents, wrong package executables and archive mappings, altered digests,
unsupported platforms, invented or missing verification evidence, unresolved
source inputs and readiness claims that skip local checks. The WinGet singleton
manifest passes the official Microsoft 1.6.0 JSON schema; this is not a Windows
installation result. Strict scoped Ruff, shell syntax, Debian changelog parsing,
Conductor checks and whitespace checks are also run.

The required hosted checks must pass on the final PR head before merge. Their
actual final-head result and merge commits are recorded in the PRs and the issue
#194 reconciliation, rather than inferred from local tests or an older head.

## External gates and closure boundary

No current Rust channel is submission-ready, submitted or independently verified
live by this audit. The legacy Python PyPI 1.0.0 listing is independently observed
and remains frozen. Docker Hub/Quay mirrors are deferred without a consumer need.

Issue #194 and this track remain **blocked externally** for the following work:

1. **Release operator:** select the next coordinated version through the existing
   release authority, resolve the occupied crate names and preserve historical
   tags. Produce one qualified signed release with the actual payloads, aggregate
   manifest, SBOM and authenticated provenance from the same source/build.
2. **Native builders:** provide immutable source/vendor/toolchain inputs and
   licence closure; run each packet's native lint/build/install/help/uninstall
   checks on every advertised architecture. Record real, reviewed evidence.
3. **Registry maintainers:** establish the tap, bucket, plugin repository,
   publisher identity, signing authority, COPR/PPA or sponsorship as appropriate;
   submit only after qualification and record the actual review URL.
4. **Registry reviewers/operators:** record acceptance, immutable public versions,
   public installation verification and ongoing freshness. A configured workflow,
   valid packet or successful HTTP response does not establish these outcomes.

The exact per-target actions are in
[`packaging/SUBMISSION_RUNBOOK.md`](../../../packaging/SUBMISSION_RUNBOOK.md).
Local implementation completion does not close the track's publication or manual
native-verification tasks. No external submission, credential change, new live
listing or successful native install is asserted here.
