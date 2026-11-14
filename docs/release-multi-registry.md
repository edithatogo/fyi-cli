# Multi-registry release operation

Reviewed **14 November 2026**. The existing release automation remains the
release authority. This packaging workstream does not publish releases, create
registry submissions, alter credentials, or change shared workflows.

Canonical distribution state:
[`registry-distribution-matrix.md`](registry-distribution-matrix.md).
Review packets, exact operator actions and rollback procedures:
[`packaging/SUBMISSION_RUNBOOK.md`](../packaging/SUBMISSION_RUNBOOK.md).

## Current release reconciliation

| Surface | Observed value | Consequence |
| --- | --- | --- |
| Latest public GitHub Release | `v1.2.0`, 10 July 2026 | Contains four CLI archives and four SHA-256 sidecars |
| CLI Cargo metadata at that release | `0.1.2` | Release tag and binary component version must not be conflated |
| Rust workspace on the reviewed branch | `fyi-core`, `fyi-cli`, `fyi-mcp` all `0.1.2` | Current master contains unreleased work beyond the historical source |
| Expected `v0.1.2` tag/release | HTTP 404 on 14 November | Do not construct download URLs by assuming that tag exists |
| Root Release Please manifest / Python project | `1.1.0` / `1.2.1` | Existing root/component version differences need operator reconciliation |
| crates.io | Current names `fyi-cli` and `fyi-core` occupied; no matching 0.1.2 index release | Name/ownership decision and coordinated metadata change required before publication |
| Integrity qualification | No attached aggregate manifest/checksum inventory/SBOM; signed release and attestations unverified | Current packets remain blocked |

Exact API observations, payload sizes and hashes are recorded in
[`release-evidence.json`](../packaging/release-evidence.json). They identify
historical artifacts; they are not clean-install or authenticated provenance
results. No release tag, package version or Release Please authority is changed
by the reconciliation.

## Existing workflow responsibilities

| Workflow | Implemented responsibility | Remaining operator prerequisite |
| --- | --- | --- |
| `release-please.yml` | Groups root/Rust release PR components | Resolve historical version mapping and coordinate all distribution metadata |
| `release.yml` | `v*` tag builds Linux amd64, macOS amd64/arm64 and Windows amd64 CLI archives; creates integrity inventory and requests provenance attestation | Signed/version-qualified release, SBOM generation and validation, actual attestation verification, crate-name resolution; standalone MCP assets are not built by this matrix |
| `container-publish.yml` | Intended amd64/arm64 GHCR publication on tags or manual dispatch | Verify public pulls, immutable image/index digests, SBOM, provenance and isolated native help |
| `ci.yml` → `Packaging Assets` | Existing scripts validate version scope, review metadata, registry ledger and smoke contracts | Passing repository checks does not publish or qualify external installs |
| Distribution freshness workflows | Bounded read-only evidence monitoring | HTTP availability alone cannot promote a channel to live |

The release workflow's current best-effort crates.io commands are **not** a
resolution of the occupied names and must not be dispatched until the release
operator reconciles them. Workflow changes, including adding a release SBOM and
qualified standalone MCP assets, belong to the release/quality owner and must
be based on that workstream's latest merged commit. No such workflow edits are
included here.

## Preflight

The ordinary preflight checks repository consistency and remains credential-free:

```bash
python scripts/verify_packaging_assets.py
python scripts/release_preflight.py --json
python scripts/validate_registry_submission_ledger.py
python scripts/validate_install_smoke_matrix.py
```

A passing ordinary report is compatible with `submission_ready: false`. The
operator must separately require submission readiness:

```bash
python scripts/verify_packaging_assets.py --require-submission-ready --target homebrew
python scripts/release_preflight.py --require-submission-ready --json
```

These commands currently fail with explicit release, identity and native-install
blockers. Do not create a tag just because repository metadata checks pass.
For a version-changing PR, the existing scope check still applies:

```bash
python scripts/verify_release_scope.py --base origin/master --head HEAD
```

## Qualifying the next release

The existing release operator must:

1. Decide the coordinated root/component version mapping and crate identities,
   without replacing historical release bytes or retagging old commits.
2. Build the exact source commit with locked dependency closure, record native
   platform/toolchain identity and produce the necessary CLI/MCP/container assets.
3. Generate SBOMs from those actual builds, record payload hashes, sign the release
   and verify authenticated provenance for the same source commit and subjects.
   Follow [`RELEASE_INTEGRITY.md`](../packaging/RELEASE_INTEGRITY.md).
4. Independently download/hash the resulting artifacts, inspect their executable
   paths and run help/version/install/uninstall smoke in clean native environments.
5. Regenerate package metadata from that evidence, resolve every packet blocker,
   run native schema/lint/build checks and the strict submission gate, then perform
   each explicitly authorized external publication using the maintainer account.
6. Record actual submission URLs, review outcomes and public install evidence in
   the existing ledger. Do not mark a package live solely because a workflow ran.

No current packet is ready for immediate external submission. The Homebrew tap,
Chocolatey, Scoop and WinGet review files have concrete observed CLI URLs/hashes;
source packages deliberately retain explicit unresolved checksum inputs behind
blocked status. asdf/mise has no approved installable version. Flathub is not an
eligible route for the current console application.

## Hosted connector boundary

Hosted connector submissions additionally require the
[hosted MCP deployment contract](../deploy/remote-mcp/README.md). The MCP server
has opt-in HTTP mode via `FYI_MCP_TRANSPORT=http`; verify the HTTPS deployment,
`FYI_MCP_HTTP_BEARER_TOKEN`, authorization boundaries and `/healthz` before
claiming a hosted endpoint. This is separate from OS package distribution and
remains under the MCP workstream.

MCP catalog submission records are preserved in
[`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](../packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md).
They do not establish GHCR image installation or native CLI package availability.
