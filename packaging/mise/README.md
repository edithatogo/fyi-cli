# mise integration status

**Blocked pending a qualified release and dedicated plugin repository.** The
[asdf-compatible plugin](../asdf/README.md) is the prepared integration path.
Its local checksum approvals, supported platforms, and safe installation
behavior apply unchanged. No versions are approved yet.

`backend.toml` is maintainer inventory, not a mise tool definition that can be
installed directly. The main project repository does not expose the plugin's
`bin/install` and `bin/list-all` at its root, so it must not be advertised as the
plugin repository. The operator must export `packaging/asdf` to a dedicated
repository, preserve executable modes, record its URL and maintainer, and run
mise and asdf install checks on the supported native platforms before submission.

Do not recommend the cargo backend for `fyi-cli`: the crates.io name is occupied
and its ownership/release binding to this project is unverified. A generic GitHub/ubi recipe also bypasses this plugin's
reviewed release approvals and is not the prepared submission path.

See mise's [asdf plugin compatibility documentation](https://mise.jdx.dev/asdf-legacy-plugins.html)
for the current installation hook contract. This packet makes no claim of mise
registry inclusion, public plugin availability, or successful native installation.
