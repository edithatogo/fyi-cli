# Blocked Snap template

`snapcraft.yaml` uses JSON syntax, which is a YAML subset. This preserves native
Snapcraft fields and permits strict parsing by the existing standard-library
packaging gate without an additional YAML dependency.

The template is not built, published or ready to upload. Hashes, qualified native
toolchains, amd64/arm64 builds, clean-install checks and store ownership remain unresolved. The
historical GitHub release v1.2.0 points to the pinned source commit, whose Cargo
metadata declares package version 0.1.2. The release operator must reconcile
that mapping before distribution; the source does not represent current master.

Before building, provide the local `fyi-cli-0.1.2-vendor.tar.gz` containing
`vendor/` and `cargo-config.toml`, generated against the pinned commit's Cargo.lock
using `cargo vendor --locked vendor`. Review bundled dependency licences and
record both archive hashes in the explicit placeholders. The vendor part is
excluded from the installed runtime. Source checksum syntax follows
[Craft Parts properties](https://documentation.ubuntu.com/craft-parts/stable/common/craft-parts/reference/part_properties/).

## Supply and qualify a toolchain for each architecture

The unchanged `core22` base is Ubuntu 22.04. Its generic
[`rustc`](https://packages.ubuntu.com/jammy-updates/rustc) and
[`cargo`](https://packages.ubuntu.com/jammy-updates/cargo) packages supply 1.75.0.
The pinned source's Cargo.lock includes `time` 0.3.49, whose
[crates.io index entry](https://github.com/rust-lang/crates.io-index/blob/master/ti/me/time)
requires Rust 1.88.0. Therefore installing the base's generic compiler packages
does not make this template buildable. They are no longer build dependencies.
Rust 1.88.0 is a known lower bound, **not** a claim that the complete dependency
graph builds with that version; the operator must qualify an exact toolchain
against the entire historical lockfile.

Prepare a relocatable compiler prefix outside the build sandbox using a fixed
Rust distribution release. Authenticate and record its upstream archive digests,
release, host target and standard-library components. The prefix must include
the actual Rust compiler, Cargo executable and all libraries they require; do
not package rustup proxies or a toolchain that fetches components on first use.
Verify that the compiler runs on the `core22` build environment and that the
frozen workspace build/tests succeed with the supplied vendor tree.

| Native Snap build | Required Rust host target |
| --- | --- |
| `amd64` | `x86_64-unknown-linux-gnu` |
| `arm64` | `aarch64-unknown-linux-gnu` |

Create **separate manifest/archive pairs** for the two native builds. In each
build directory, provide `fyi-cli-rust-toolchain.tar.gz` with these three
top-level entries:

- `rust-toolchain/`: the full relocatable prefix, including `bin/rustc`,
  `bin/cargo`, and its `lib/` contents.
- `toolchain-version`: one exact stable release, such as `1.x.y`, matching both
  `rustc -vV` and `cargo --version` after qualification. Do not use this example
  literally or select a rolling channel.
- `toolchain-target`: the corresponding Rust host target from the table.

Keep the two metadata files beside the prefix in the archive. This avoids a
single enclosing directory being stripped by the archive source handler.
Normalize archive ownership/order/timestamps for reproducibility, compute its
SHA-256, and replace `@RUST_TOOLCHAIN_SHA256@` only in that architecture's manifest
copy. Record both archive digests, exact versions, qualification transcripts and
the source/vendor bindings in the release evidence. Build each pair natively with
`snapcraft --build-for=amd64` or `snapcraft --build-for=arm64`, respectively.

The build requires executable compiler/Cargo files, verifies their release and
native target against the archive metadata, rejects Rust below 1.88.0, and uses
only that prefix with Cargo networking disabled. Missing, old or mismatched
toolchains fail before Cargo builds anything. The toolchain part is excluded
from the installed runtime. No network rustup/bootstrap fallback is provided.

Frozen build and test commands fail when dependencies are unavailable or tests
fail. There is no fallback fetch. Preserve and record the native toolchain,
build dependencies, base revision, source date, SBOM and provenance when the
operator builds this template. Use a fresh confined install to test each binary's
`--help` with outbound network access denied. The manifest grants no host-home or
removable-media access; assess explicit export-directory access during native
validation before claiming the package is fully functional.
