# Release integrity and qualification

The package-submission track requires one signed, versioned release as the source
of payloads, hashes, SBOMs and provenance. A public release, a repository manifest
or a successful checksum comparison alone does not establish that requirement.
See [release compatibility](release-compatibility.json), the
[submission ledger](registry-submissions.json) and the
[release runbook](../docs/release-multi-registry.md).

## Deterministic local byte inventory

The existing `scripts/create_release_manifest.py` now provides three operations:

| Operation | Behaviour |
|---|---|
| `create_manifest(assets, version, commit)` | Return a byte inventory; write nothing |
| `write_manifest(assets, version, commit)` | Generate `release-manifest.json` and `SHA256SUMS` locally |
| `verify_manifest(assets, expected_version, expected_commit)` | Return a list of failures, or `[]`; write nothing |

Use a dedicated staging directory under exclusive operator control. Files must be
regular files with portable ASCII basenames. Symlinks, subdirectories, special
files, Windows device names, case-insensitive collisions and aliases of the two
generated filenames are rejected. The helper streams payload hashing in 1 MiB
chunks, accepts at most 4,096 payloads and bounds each evidence file to 4 MiB.
It rejects an empty payload set. Do not change the staging directory while it runs.

The release version is SemVer without a leading `v`; prerelease and build metadata
are supported. The commit must be a full 40-character hexadecimal Git object ID;
it is stored in lowercase. These are input constraints, not a lookup of a tag or
an assertion that the commit produced the bytes.

The manifest inventories every payload by name, byte length and SHA-256. It omits
the two generated files to avoid a circular hash. `SHA256SUMS` binds every payload
**and the exact manifest bytes**; it excludes itself. All lists and JSON keys are
sorted, with fixed UTF-8/ASCII encoding and LF newlines. Regenerating an unchanged
directory produces identical bytes. Evidence files are individually replaced
atomically; the pair is not a filesystem transaction. Interrupted generation must
pass verification before use.

With the approved release version and commit supplied by the operator:

```sh
python scripts/create_release_manifest.py release-assets \
  --version "${RELEASE_VERSION:?set approved version}" \
  --commit "${RELEASE_COMMIT:?set full approved commit}"
python scripts/create_release_manifest.py release-assets \
  --version "$RELEASE_VERSION" --commit "$RELEASE_COMMIT" --verify
```

Verification recomputes the inventory and requires the exact canonical manifest
and checksum list. Missing, extra, renamed, modified or unsafe files fail. So do
malformed/duplicate-key JSON, changed release identity, additional metadata and
checksum-list drift. `--verify` exits nonzero without repairing evidence. Never
regenerate evidence to make an untrusted downloaded release pass.

This helper neither extracts archives nor validates their contents or executables.
An archive's checksum cannot establish its target architecture, safe internal
paths, CLI version, dependency availability or clean-install behaviour. Those
checks belong to the package-specific smoke contract.

## Authenticated provenance is a separate requirement

An attacker able to replace a binary can also recompute this manifest and its
checksums. The helper checks byte consistency, not publisher identity. A field
such as `verified: true` or an attestation-shaped JSON document is not signature
verification. Preserve the actual verification output and exact artifact digests
in the release evidence record.

1. The release operator chooses the release version, signing identity and commit
   through the existing release process. Do not rename or move historical tags.
2. Verify the signed annotated tag against the operator's independently trusted
   signing identity, then compare its peeled commit with the approved commit:

   ```sh
   git verify-tag "${RELEASE_TAG:?set approved tag}"
   test "$(git rev-parse "${RELEASE_TAG}^{commit}")" = "$RELEASE_COMMIT"
   ```

3. The existing [release workflow](../.github/workflows/release.yml) requests
   GitHub artifact provenance. Inspect the actual run and verify its attestation
   against the bytes, repository, workflow, source ref and source commit. With a
   GitHub CLI version supporting these documented options:

   ```sh
   gh attestation verify release-assets/SHA256SUMS \
     --repo edithatogo/fyi-cli \
     --signer-workflow edithatogo/fyi-cli/.github/workflows/release.yml \
     --source-ref "refs/tags/$RELEASE_TAG" \
     --source-digest "$RELEASE_COMMIT" \
     --format json > release-verification.json
   ```

   This authenticates the checksum inventory only when the required attestation
   exists and the command succeeds. Run the local inventory verifier as well.
   Also verify each directly attested payload and SBOM before recording those
   individual attestations as verified. Keep verification receipts outside the
   inventoried directory; otherwise they are new payloads and invalidate it.
4. Missing signatures, unavailable CLI capabilities, missing attestations or a
   mismatched identity/ref/digest remain release blockers. The installed CLI in
   the packaging implementation environment did not support `gh attestation`;
   no cryptographic verification or live publication was performed here.

The command and policy controls are described in the
[official GitHub CLI attestation manual](https://cli.github.com/manual/gh_attestation_verify).
These checks do not prove reproducible builds or that a trusted workflow was free
of compromise; the verified workflow and its build inputs still require review.

## SBOM scope and outstanding release inputs

Every distributable payload needs an SPDX or CycloneDX dependency inventory for
its exact source revision, lockfile, feature selection, target and build. Record
generator identity/version and review bundled dependency licences. Include native
SBOM documents as payloads **before** generating the integrity manifest. Verify
their schema and coverage; a nonempty JSON file is insufficient.

The current native release workflow does not generate a native CLI SBOM. The
release-engineering owner must select and pin a suitable generator, generate the
native inventory, attest/bind it to the release, and record verification. This
packaging change neither modifies that workflow nor claims the missing evidence.

The existing [container workflow](../.github/workflows/container-publish.yml)
requests BuildKit provenance and SBOMs for amd64/arm64. Those declarations do not
prove a published image has either. Inspect the immutable image digest, including
each platform, and retain the results. Existing Buildx tooling can inspect the
SPDX document for each platform, for example:

```sh
docker buildx imagetools inspect "${RELEASE_IMAGE:?set image at sha256 digest}" \
  --format '{{ json (index .SBOM "linux/amd64").SPDX }}'
docker buildx imagetools inspect "$RELEASE_IMAGE" \
  --format '{{ json (index .SBOM "linux/arm64").SPDX }}'
```

Inspecting an SBOM does not authenticate its publisher. BuildKit's default scan
scope covers the final image stage; build-stage and source dependencies require
explicit scope and review. A container SBOM does not automatically cover a
separately compiled CLI archive. See
[Docker's SBOM documentation](https://docs.docker.com/build/metadata/attestations/sbom/).

## Clean-install evidence and rollback

Use the existing [install smoke matrix](install-smoke-matrix.json) for the chosen
channel. Resolve an exact qualified version, install into a clean disposable
environment, check only the declared help/version commands with no credentials,
and record OS, architecture, package/binary version, digests and result. A parsed
manifest or generated submission packet is not an executed install smoke test.

For rollback, choose an earlier **independently qualified** release and repeat
identity, integrity and clean-install checks before changing the package-manager
pin. Do not overwrite existing release assets or retarget an existing version.
Yank/unlist/revoke actions remain registry-specific operator operations; retain
the affected artifact's evidence and block its use in new packets.
