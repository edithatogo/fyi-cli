# CLI entrypoints audit

Status captured: 2026-06-15.

## Cross-reference map

| Surface | Entry point | Registry / docs |
|---------|-------------|-----------------|
| Python CLI | `fyi`, `fyi-cli`, `fyi-system` -> `fyi_system.cli:main` | [README](../README.md), [docs site](../docs-site/src/content/docs/index.md) |
| Rust CLI | `crates/fyi-cli` -> `fyi-cli` binary from `crates/fyi-cli/src/main.rs` | [Rust migration guide](../docs-site/src/content/docs/guides/rust-migration.md), [release checklist](release-candidate-checklist.md) |
| Rust MCP | `crates/fyi-mcp` -> `fyi-mcp` binary from `crates/fyi-mcp/src/main.rs` | [GitHub MCP Registry](https://registry.modelcontextprotocol.io/servers/io.github.edithatogo/fyi-mcp), [Smithery](https://smithery.ai/server/@edithatogo/fyi-mcp), [MCP bundle manifest](../server.json) |

## Python package entrypoints

Defined in `pyproject.toml` under `[project.scripts]`:

- `fyi` -> `fyi_system.cli:main`
- `fyi-cli` -> `fyi_system.cli:main`
- `fyi-system` -> `fyi_system.cli:main`

The canonical Python implementation module is `src/fyi_system/cli.py`, which uses `argparse` with program name `fyi-system`.

## Rust workspace entrypoints

Defined in `Cargo.toml` as a workspace with members:

- `crates/fyi-cli` -> Rust CLI binary using `clap` in `crates/fyi-cli/src/main.rs`
- `crates/fyi-mcp` -> MCP server binary in `crates/fyi-mcp/src/main.rs`
- `crates/fyi-core` -> shared Rust core library

## Auxiliary scripts

- `scripts/autonomous-tracks.py` -> conductor automation helper
- `scripts/check_conductor_status.py` -> conductor status helper
- `scripts/run_local_cycle.sh` -> local cycle runner

## CLI-first decision

The current stable user-facing command surface is the Python package entrypoint set: `fyi`, `fyi-cli`, and `fyi-system`. The Rust CLI/MCP workspace is an active migration and packaging surface, not yet the sole canonical command surface.

Going forward, automation should prefer `fyi`/`fyi-cli` for user workflows, `fyi-system` for explicit legacy compatibility, and Rust binaries only for tracks that are specifically implementing or validating the Rust migration.
