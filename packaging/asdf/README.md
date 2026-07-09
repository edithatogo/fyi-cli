# asdf plugin drafts for fyi-cli

Draft scripts for an [asdf](https://asdf-vm.com/) plugin that installs **fyi-cli** (and **fyi-mcp** when available).

| Path | Role |
|------|------|
| `bin/list-all` | Lists versions from GitHub Releases |
| `bin/install` | Installs a version (prebuilt tarball, then cargo fallback) |

## Local use (before a dedicated plugin repo)

```bash
# Clone or symlink this directory as an asdf plugin named fyi-cli
mkdir -p ~/.asdf/plugins/fyi-cli
cp -r packaging/asdf/* ~/.asdf/plugins/fyi-cli/

asdf install fyi-cli 0.1.2
asdf global fyi-cli 0.1.2
fyi-cli --help
```

## Metadata

- Homepage: https://github.com/edithatogo/fyi-cli
- Version (current draft): 0.1.2
- License: MIT
- Publisher: edithatogo

## Notes

- Prebuilt asset URLs follow cargo-dist / cargo-binstall naming; update `bin/install` when release asset names stabilize.
- For mise users, see `packaging/mise/`.
