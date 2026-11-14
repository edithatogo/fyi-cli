# asdf-compatible CLI packaging

The local plugin installs the `fyi-cli` binary from reviewed GitHub Release
archives. It requires Bash, Python 3.11 or newer, and curl. The [asdf plugin
contract](https://asdf-vm.com/plugins/create.html) supplies `ASDF_INSTALL_TYPE`,
`ASDF_INSTALL_VERSION`, and `ASDF_INSTALL_PATH` to `bin/install`.

**Status: blocked pending a qualified release and dedicated plugin repository.**
No approved checksum files are checked in, so `bin/list-all` currently prints an
empty version list and installation fails before downloading anything. A source
tag, an MCPB-only release, or a package draft does not establish installability.
In particular, the historical `v1.2.0` tag contains crate version `0.1.2` and is
not an approved synchronized release for this plugin.

## Release approval contract

The release operator must verify a signed release, its provenance, SBOM, archive
bytes, and clean-install `--help`/`--version` results before committing
`checksums/X.Y.Z.sha256`. The first two lines must identify the same stable
version as both the GitHub tag and the CLI binary:

```text
# release-tag: vX.Y.Z
# binary-version: X.Y.Z
```

Each subsequent line must contain a lowercase SHA-256 digest, two spaces, and one
of these exact asset names. Copy verified digests from the release evidence; do
not create an approval from an unverified checksum sidecar. Do not add the
illustrative `X.Y.Z` header as a real approval.

| Platform | Allowed archive |
| --- | --- |
| Linux x86_64 | `fyi-cli-linux-amd64.tar.gz` |
| macOS x86_64 | `fyi-cli-macos-amd64.tar.gz` |
| macOS arm64 | `fyi-cli-macos-arm64.tar.gz` |

Only approved platform entries are installable. Linux ARM64, Windows, unknown
platforms, prereleases, and ref installations fail before download. The scripts
do not guess archive names, build source code, install MCP bundles, or consult
crates.io.

`bin/list-all` reads these local approvals without contacting GitHub or using
credentials. Versions are sorted numerically, newest last. Missing approvals
produce an empty list; malformed approvals fail closed.

## Install and verification behavior

The installer downloads one exact HTTPS URL with time and size limits, verifies
its digest, and accepts exactly one regular root `fyi-cli` archive member. It
rejects links, traversal paths, duplicate entries, and unexpected content.
Archive paths are never extracted directly. It checks the verified executable's
`--version` in a temporary HOME/XDG environment with a timeout, then atomically
replaces `bin/fyi-cli`. A download, hash, archive, or version failure preserves
the existing executable. This version check is not a registry acceptance or
clean-install certification.

Offline fixture verification:

```bash
python -m pytest -q tests/test_asdf_packaging.py
bash -n packaging/asdf/bin/install packaging/asdf/bin/list-all
```

Before publication, export this directory as the root of a dedicated plugin
repository, preserving executable modes, and record its URL and maintainer in
the registry ledger. Re-run asdf's plugin test on each supported native platform
after adding a qualified release. To withdraw a bad version, remove its approval
file and update the plugin; for rollback, install a previously approved version.
The [mise integration notes](../mise/README.md) use the same approval boundary.
