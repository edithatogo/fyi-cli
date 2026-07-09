# mise integration drafts for fyi-cli

This directory documents how to install **fyi-cli** with [mise](https://mise.jdx.dev/).

## Quick options

### 1. asdf-compatible plugin (in-repo drafts)

```bash
# Register the in-tree asdf plugin scripts
mise plugins install fyi-cli "$PWD/packaging/asdf"
mise install fyi-cli@0.1.2
mise use -g fyi-cli@0.1.2
```

### 2. cargo backend

```bash
mise use cargo:fyi-cli@0.1.2
# or from git:
# mise use cargo:fyi-cli@git+https://github.com/edithatogo/fyi-cli
```

### 3. ubi / GitHub release (when assets exist)

```toml
# mise.toml
[tools]
"ubi:edithatogo/fyi-cli" = "0.1.2"
```

See `backend.toml` for structured metadata used by maintainers.

## Metadata

- Homepage: https://github.com/edithatogo/fyi-cli
- Version: 0.1.2
- License: MIT
- Publisher: edithatogo
