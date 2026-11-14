# Blocked Debian/PPA source-package template

Package version remains 0.1.2. The historical GitHub release v1.2.0 points to
source commit `86fcbd8914037735b6f27e5a329d5f3013033efd`, whose Cargo metadata
declares 0.1.2. The release operator must reconcile this mapping before uploading;
these files do not change release authority or describe unreleased master.

Before a native build:

1. Verify the immutable source archive and record its SHA-256, source commit,
   Cargo.lock hash and release provenance. Export it as the orig source package.
2. Generate `vendor/` from that lockfile with `cargo vendor --locked vendor` and
   save the generated source replacement configuration as `.cargo/vendor-config.toml`.
   Record the vendor archive hash. Prepare it outside the offline build chroot.
3. Review bundled dependency licences and update the DEP-5 copyright inventory.
   The current copyright file only covers the upstream application. For Debian
   archive inclusion, work with the Debian Rust team to resolve packaged Cargo
   dependencies; the bundled PPA approach does not establish archive eligibility.
4. Place this directory at the source root as `debian/`, preserving the executable
   `rules` mode. Keep `UNRELEASED` until an operator selects the target suite.
5. Run `dpkg-buildpackage`, `lintian` and clean install/help checks in native amd64
   and arm64 environments. Preserve `SOURCE_DATE_EPOCH` from the changelog, and
   record exact compiler and build dependency versions for reproducibility.

The rules reject missing vendoring and run frozen build/test commands. No builder,
Lintian run, signing, PPA upload or Debian submission is claimed by this template.
After validation, record the signed `.dsc`, `.changes`, `.buildinfo`, package
hashes, SBOM, install logs and a rollback version in the submission ledger.

[Debian source-package policy](https://www.debian.org/doc/debian-policy/ch-source.html)
defines build dependencies, changelog dates, source formats and copyright records.
