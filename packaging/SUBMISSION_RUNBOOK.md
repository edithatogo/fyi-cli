# Package submission packets and operator runbook

Review date: **14 November 2026**. Owner: **edithatogo**. Track:
[`package-registry-submissions_20260712`](../.conductor/tracks/package-registry-submissions_20260712/).
Issue: [#194](https://github.com/edithatogo/fyi-cli/issues/194).

**No current Rust distribution packet is submission-ready.** The repository now
contains corrected, reviewable native metadata and explicit blocked source
build templates. Native package builders, clean installs and public registry
acceptance are separate evidence gates. No credentials or external submissions
are part of these changes.

## Release and version authority

The observed public GitHub release is
[`v1.2.0`](https://github.com/edithatogo/fyi-cli/releases/tag/v1.2.0), published
10 July 2026. Its immutable source commit is
`86fcbd8914037735b6f27e5a329d5f3013033efd`, and its
[`crates/fyi-cli/Cargo.toml`](https://github.com/edithatogo/fyi-cli/blob/86fcbd8914037735b6f27e5a329d5f3013033efd/crates/fyi-cli/Cargo.toml)
declares **0.1.2**. `v0.1.2` tag and release lookups returned HTTP 404 on the
review date. These are different identifiers; do not silently change the Rust
version, repoint historical tags or describe the July binaries as current master.

The existing Release Please manifest records root **1.1.0**, while
`pyproject.toml` declares **1.2.1** and all three Rust components declare **0.1.2**.
Release Please groups the components but does not resolve those historical
version differences. The release operator must select and synchronize the next
release through the existing release process. This work changes no version,
release authority, credential, tag or publication workflow.

[`release-evidence.json`](release-evidence.json) records the observed release,
all eight asset names, sizes and GitHub API SHA-256 values, the source binding and
unverified controls. The native binary review packets reference those exact
historical CLI assets and retain package version 0.1.2. Hashes observed through
the release API are **not** independent verification of downloaded bytes,
authenticated build provenance or a successful installation.

Observed platforms: Linux x86_64, macOS x86_64, macOS arm64 and Windows x64.
The release has no Linux arm64 CLI archive and no standalone MCP server archives.
It also has no attached `SHA256SUMS`, `release-manifest.json` or SBOM among the
eight observed assets. The lightweight tag is not a verified signed release;
no artifact attestation has been verified in this audit. Historical MCPB assets
belong to a separate release and must never substitute for CLI executables.

## Repository validation and submission gate

The existing required **Packaging Assets** job invokes these scripts; no shared
GitHub Actions workflow change is needed:

```bash
python scripts/verify_packaging_assets.py
python scripts/validate_registry_submission_ledger.py
python scripts/validate_install_smoke_matrix.py
python scripts/release_preflight.py --json
```

The first checks repository consistency, exact version fields, native binary
URLs/digests, package identity and blocked-template status. Passing does not mean
publication readiness. The preflight explicitly reports `submission_ready: false`.
To ask whether a packet can be submitted, use the fail-closed gate:

```bash
python scripts/verify_packaging_assets.py --require-submission-ready --target homebrew
python scripts/release_preflight.py --require-submission-ready --json
```

Both currently exit **1**, with precise blockers. Deliberately unresolved source
hash tokens are permitted only in blocked source templates. They are not valid
hashes and cannot be carried into a ready/submitted/live packet. A packet may be
marked `assets-ready` only after its blockers are empty, its immutable release
binding is valid and the required evidence exists. `submitted` requires an actual
submission URL; `live` additionally requires verified public install evidence.

## Reviewable packet inventory

The machine-readable inventory is
[`registry-submissions.json`](registry-submissions.json): each of the fourteen
requested channels has an owner, route, packet files, dated evidence, readiness
flag, explicit blockers and exact next action. The following commands are
**operator instructions for a qualified release**, not claims that native tools
or publication have already run.

| Target / packet | Status and required operator sequence | Update, withdrawal and rollback |
| --- | --- | --- |
| **crates.io** — `crates/fyi-{core,cli,mcp}/Cargo.toml` | Blocked: `fyi-cli` and `fyi-core` names are occupied; the index has no matching 0.1.2 release. Select and verify names/ownership, synchronize dependent manifests, run `cargo package` and `cargo publish --dry-run` in dependency order. Operator publishes only after identity and release checks. | Publish a new patch version; yank a broken version using the owner account. Do not replace crate bytes or reuse a version. |
| **GHCR** — `Dockerfile`, existing multi-arch verifier/workflows | Blocked: public amd64/arm64 pulls, signed provenance, SBOM and digest/version binding unverified. Operator publishes, records index and child digests, then uses `docker buildx imagetools inspect` and `scripts/verify_ghcr_multarch.py` plus isolated help checks on each architecture. | Promote immutable digests. Roll back the consumer deployment to a previously verified digest; preserve incident evidence before any deletion. |
| **Homebrew tap** — `homebrew/fyi-cli.rb` | Blocked review packet with concrete hashes. Run `brew audit --strict` and `brew test` after fresh installs on the three advertised targets; operator places it in an owned tap. This native-binary formula is not a Homebrew/core source formula. | Update formula and hashes together; retain a verified prior formula/version in the tap. Remove or disable a compromised formula and advise installed users. |
| **Chocolatey** — `chocolatey/` | Blocked review packet: CLI ZIP, SHA-256, verification notes, MIT licence and uninstall script. Run `choco pack` in that directory; in a fresh Windows Sandbox perform local package install, `fyi-cli.exe --help` and uninstall. Operator uploads using the maintainer account and records moderation URL. | Submit a corrected version; deprecate/unlist broken metadata using the maintainer route. Restore a previously verified package; do not alter system PATH manually. |
| **Scoop** — `scoop/fyi-cli.json` | Blocked review packet: x64 CLI ZIP and executable. Validate with the chosen bucket's tooling, then clean `scoop install ./fyi-cli.json`, help and uninstall. Operator opens a bucket PR. Automatic version discovery is deliberately absent while tags and component versions differ. | Update URL/hash/version atomically; revert the bucket commit to a verified prior package and remove the bad entry if necessary. |
| **WinGet** — `winget/edithatogo.fyi-cli.yaml` | Blocked review packet: JSON-compatible YAML singleton, ZIP with nested portable `fyi-cli.exe`. Run `winget validate --manifest packaging/winget` and a clean Windows Sandbox install/help/uninstall; operator opens a microsoft/winget-pkgs PR at the correct identifier/version path. | Submit a corrected version or removal PR; use an explicitly verified prior version when rollback is necessary. |
| **cargo-binstall** — `cargo-binstall/metadata.toml` and crate metadata | Blocked: occupied crate identity and unqualified asset mapping. All default Binstall strategies are disabled; explicit operator `--strategies` can override that upstream safeguard. After identities and a synchronized release are qualified, replace this policy and its verification gate with exact per-target archive URL/format/bin-dir mappings, then smoke from an empty install root. The current gate deliberately rejects a ready claim while this disabled policy is in force. | Restore disabled strategies if identity/integrity is uncertain; update the published crate metadata with a new version. Do not treat MCPB ZIP bytes as `pkg-fmt = "bin"`. |
| **AUR** — `aur/PKGBUILD` | Blocked source template. Produce immutable source and locked vendor archives; replace source/vendor hash tokens. Run `makepkg --verifysource`, build in a clean Arch chroot, run tests/help, generate `.SRCINFO` with `makepkg --printsrcinfo`, then operator pushes to AUR. | Bump `pkgrel` for packaging fixes or `pkgver` for upstream changes; revert to a verified source/hash pair. |
| **nixpkgs** — `nix/default.nix` | Blocked source template. Compute actual source NAR and Cargo vendor hashes, select a pinned nixpkgs/toolchain revision and real maintainer entry, then `nix-build` and native tests on each claimed Linux architecture. Operator submits upstream after successful builds. | Update source and dependency hashes together; revert a nixpkgs commit or consumer flake lock to a verified derivation. |
| **Snap** — `snap/snapcraft.yaml` | Blocked strict-confinement template. Verify/reserve publisher/name; supply source/vendor hashes and a separately qualified, checksummed offline Rust/Cargo toolchain for each architecture. The core22 default Rust 1.75 cannot build the locked dependencies. Follow `snap/README.md`, build each native manifest/archive pair with `snapcraft`, install in a clean system, verify help and confinement, then operator uploads to edge for validation. | Promote only verified revisions; close a bad channel release and revert to a retained prior revision. Stable promotion is a separate operator step. |
| **Flatpak / Flathub** — `flatpak/` | **Not applicable to Flathub** for this console product under current inclusion policy. Private Flatpak remains a blocked optional template: complete source/vendor hashes, exact SDK/runtime qualification, `appstreamcli validate`, offline `flatpak-builder`, sandbox help and uninstall first. | For a private repository, sign new commits and roll back to a verified commit. There is no Flathub submission to withdraw. |
| **asdf / mise** — `asdf/`, `mise/` | Blocked: no approved synchronized version or standalone plugin repository. Verify release/SBOM/provenance and native smoke, then add reviewed per-version checksums. Publish a plugin repo with `bin/` at root and request registry inclusion. `mise/backend.toml` is an inventory, not a live mise plugin. | Remove an unsafe version from the approval inventory; publish the plugin correction. Already-installed binaries require explicit user rollback/reinstall. |
| **Debian / PPA** — `debian/` | Blocked: prepare source/vendor/dependency licence closure and native offline `sbuild`, `lintian` and `autopkgtest` results; sign `.dsc`, `.changes` and `.buildinfo`. Operator chooses Debian sponsorship or an owned PPA. A PPA upload is not Debian archive acceptance. | Issue a corrected package revision; remove the affected PPA publication or work with Debian maintainers for archive remediation. Pin a verified previous binary only when safe. |
| **Fedora / COPR** — `fedora/fyi-cli.spec` | Blocked: compute source/vendor hashes, build SRPM, run `rpmlint` and clean `mock` builds; operator creates/uses a COPR project. Fedora archive inclusion separately needs Fedora Rust packaging review and dependency policy compliance. | Publish a corrected RPM release, disable/remove the affected COPR repository build and restore a previously verified NEVRA as appropriate. |

Docker Hub and Quay are **not applicable/deferred** until a consumer need and
owner justify a mirror after GHCR qualification. Legacy PyPI **1.0.0** is publicly
listed and frozen for migration; it is not the Rust package. Its published
placeholder support URLs and absent provenance remain historical operator
follow-up, not a reason to publish a new Python application release here.

## Integrity, SBOM, provenance and reproducibility

Use [`RELEASE_INTEGRITY.md`](RELEASE_INTEGRITY.md) and the existing
`scripts/create_release_manifest.py` for deterministic hashing and read-only
verification. A release must bind the source commit, all payloads, dependency
lockfile and SBOM to the same build. A checksum list establishes integrity only;
a signed tag and authenticated attestation establish separate origin evidence.
Do not manufacture an SBOM from a package name, copy a Cargo.lock inventory and
claim binary coverage, or infer a successful signature check from JSON flags.

For every `verification` flag set to `true`, `release-evidence.json` must also
contain a matching entry in `verification_receipts`: `downloaded_bytes`,
`clean_install`, `signed_release`, `sbom` or `provenance`. Each receipt contains
`path`, `sha256`, `version` and `source_commit`. The path must resolve to a
nonempty file within this repository, at most 4 MiB; its SHA-256 must match the
actual bytes and its version/commit must match the observed release. Keep the
redacted verifier output and immutable artifact references in that file. The
referenced file must be structured JSON with `schema_version: 1`, the matching
`kind`, `version`, `source_commit`, `result: "passed"`, integer `exit_code: 0`
and exact `subjects` entries containing `name` and `sha256`. Arbitrary text,
failed results and receipts for unrelated subjects do not qualify.

| Receipt kind | Required subjects |
| --- | --- |
| `signed_release` | `release-manifest.json` and `SHA256SUMS` |
| `downloaded_bytes` | Every observed release asset and native artifact |
| `clean_install`, `provenance` | The four CLI archives and every native artifact |
| `sbom` | Installed artifact subjects plus the identified SPDX/CycloneDX SBOM assets |

A ready release must actually inventory `release-manifest.json`, `SHA256SUMS`
and an SBOM named with `.spdx.json`, `.cdx.json` or `.cyclonedx.json`. An inventory
entry still needs authenticated byte verification; a filename is not schema or
coverage validation. The qualified operator records that result in the receipt.

Source-built packages and container images use the optional `native_artifacts`
inventory. Each entry identifies `target`, `platform`, `version`, `source_commit`,
a unique safe `name`, the actual native artifact `sha256`, and a
`verification_receipt` envelope with the same four fields used above. Its JSON
receipt uses `kind: "native_artifact"`, the matching target/platform and exactly
that artifact subject. For OCI images, record each platform's immutable image
digest. For native packages, record the built package/binary digest. A source
tarball digest cannot stand in for the resulting executable or container.

These records bind an operator's evidence; the metadata validator does not
itself authenticate signatures, attest a build or execute native installs.
Review actual verifier output before asserting success. All five verification
flags currently remain `false`, and no qualified native artifact is invented.

The existing release workflow must be reconciled by its owner before publication:
it still builds only CLI archives, does not itself produce the required release
SBOM, and has best-effort crates.io publication under names that are occupied.
The separate container workflow is not evidence of public pulls. These exact
release-operator prerequisites are recorded in this track; no workflow was
modified or dispatched by the packaging workstream.

Source reproducibility needs the exact source, Cargo.lock, generated vendor
configuration, dependency licences, toolchain/build dependencies and
`SOURCE_DATE_EPOCH`. Frozen build commands now fail when those inputs are missing;
there is no unlocked or network fallback and test failures cannot be ignored.
Native reproducible-build comparisons remain unperformed until those inputs and
platform builders are available.

## Clean-install evidence contract

[`install-smoke-matrix.json`](install-smoke-matrix.json) covers all fourteen
channels. It requires an empty user/container root, no credentials or production
database, verified artifact digest and binary version, a bounded **help-only**
command with networking disabled during execution, and uninstall verification.
Package download/build may require network; the installed program's smoke must
not. The allowed command is exact: strings containing `--help` plus shell
operators or write commands are rejected.

Keep the redacted transcript, version, OS/architecture, SHA-256, source commit,
timestamp, exit status and package-manager version in the evidence record. Run
container smoke with `--network none`, a read-only root and an ephemeral writable
home if needed. Record real native results for Windows/macOS; synthetic archive
fixtures are installer behavior tests, not native installation evidence.

A `verified` matrix row requires an evidence object containing `version`,
`source_commit` and `samples`. Each sample contains `platform`, `asset`, `sha256`,
`verified_at`, `transcript` and `transcript_sha256`. The transcript is a nonempty
repository-relative JSON file of at most 4 MiB; its digest must match its bytes.
It uses the successful receipt fields above with `kind: "clean_install"`, its
single exact artifact subject and matching `target`, `platform` and `command`.
It must also record `clean_environment: true`, `network_during_smoke: false`,
`remote_writes: false`, the contract's `timeout_seconds`, and `uninstalled: true`.
Keep the redacted install/help/uninstall output and exit statuses with the record.

Every advertised platform needs its own sample. Binary channels must match the
actual CLI archive for that platform; source-built/container channels must match
a separately recorded native artifact. A ready registry row requires its
corresponding verified smoke row. Missing or altered transcripts, stale
versions/commits, a Windows result for Homebrew, a source hash used as a binary
hash or a command for the wrong executable fail validation. Clearing external
blockers cannot override a failed local preflight check.

## Public evidence and monitoring

Record the actual submission URL when an operator submits; use `submitted` until
review completes. Public listing alone is not a clean-install result. Use a
package/version URL and verified install transcript before `live`, with a dated
re-check. Existing freshness monitoring remains in place; a successful HTTP
response alone must never promote readiness or install status.

Primary policy references, reviewed 14 November 2026:

- [Homebrew core eligibility](https://docs.brew.sh/Acceptable-Formulae)
- [Flathub requirements](https://docs.flathub.org/docs/for-app-authors/requirements)
- [Binstall supported metadata](https://github.com/cargo-bins/cargo-binstall/blob/e93a9f89058afc12903de4d5c3696de365b7fe36/SUPPORT.md)
- [crates.io fyi-cli index snapshot](https://github.com/rust-lang/crates.io-index/blob/59a67e5410ccd818baaeea3f836df244972e2f3e/fy/i-/fyi-cli)
- [crates.io fyi-core index snapshot](https://github.com/rust-lang/crates.io-index/blob/59a67e5410ccd818baaeea3f836df244972e2f3e/fy/i-/fyi-core)
- [Legacy PyPI listing](https://pypi.org/project/fyi-cli/)

The crates.io index independently establishes that both names are occupied and
does not contain the expected 0.1.2 versions. Current owner/API and docs.rs
lookups were unavailable in this audit, so the owner/repository relationship
must be verified by the operator; no crate ownership transfer or new name is
assumed here.

GitHub code search found no `fyi-cli` entry in the searched Homebrew/core,
Scoop/Main, winget-pkgs, nixpkgs, asdf-plugins or mise repositories on the review
date. This is bounded negative evidence, not proof that no private tap, fork,
bucket or unindexed listing exists. GHCR/AUR/Snap/Chocolatey live status remains
unverified, not asserted absent.
