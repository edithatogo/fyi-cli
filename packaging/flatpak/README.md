# Private Flatpak template

The manifest uses JSON syntax, which is a YAML subset, so the existing
standard-library packaging gate can parse its native fields without an extra
YAML dependency. Flatpak supplies private XDG directories; exports require an
explicit per-run directory grant rather than access to the host home or XDG data.

This is an unbuilt, blocked template for a private Flatpak repository. It is
**not eligible for the current Flathub submission route**: Flathub's
[inclusion policy](https://docs.flathub.org/docs/for-app-authors/requirements)
excludes console software except Flatpak/Flathub tooling. Its generative AI
policy also prohibits assisted submission materials. Do not submit these files
to Flathub or present them as a reviewed submission packet.

The application ID is retained: Flatpak's
[ID conventions](https://docs.flatpak.org/en/latest/conventions.html)
permit a hyphen in the final component. Runtime 25.08 replaces the expired
24.08 branch; record the exact SDK/runtime commits when building. Freedesktop
branches have a [two-year support period](https://docs.flathub.org/docs/for-app-authors/runtimes).

The source URL pins the historical v1.2.0 commit whose Cargo package version is
0.1.2. That mismatch must be reconciled by the release operator before distribution.
Generate a vendor archive against that source's Cargo.lock containing `vendor/`
and the `cargo vendor --locked` output as `cargo-config.toml`, then record its
SHA-256 and source SHA-256 in the placeholders. The template fails without them.
It has no networked build fallback and does not grant access to the host home.

On a native builder, run `appstreamcli validate --explain` on the metainfo and
build using `flatpak-builder --disable-updates --force-clean build-dir
io.github.edithatogo.fyi-cli.yml`. Validate `flatpak run
io.github.edithatogo.fyi-cli --help` and `flatpak run --command=fyi-mcp
io.github.edithatogo.fyi-cli --help` in a fresh user profile with outbound
network access denied. Store logs, exact runtime commits, SBOM, signatures and
provenance before claiming a working private package. These checks have not run.
