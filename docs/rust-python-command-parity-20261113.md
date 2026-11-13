# Python ↔ Rust CLI parity inventory (2026-11-13)

Scope: top-level parser names on PR #403: `src/fyi_system/cli.py` calls to `.add_parser`, and `Commands` in `crates/fyi-cli/src/main.rs`; **not** a claim of runtime or output equivalence. Source: GitHub PR #403 and branch `plan/frontier-rust-migration-quality-20261113`.

## Coverage summary
- Python: **46** top-level subcommands, exposed through **three** Python package executables (`fyi`, `fyi-cli`, `fyi-system`).
- Rust: **44** top-level clap variants, exposed as the native `fyi-cli` binary (plus `fyi-mcp` separately).
- **34 names overlap**; **12 Python commands have no matching Rust top-level name**; **10 Rust commands have no matching Python top-level name**.
- **Important:** Rust implementation for many name-matched commands contains diagnostic `println!` placeholders rather than actual DB/network/report processing. The matching 34 names are **syntax-only potential parity**, not proven behavioural parity.

## Python-only commands: port or replace with contract-tested Rust implementation
| Command | Proposed Rust destination | Exposure / acceptance |
|---|---|---|
| `discover-bodies` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `discover` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `rate-limit-status` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `dry-plan` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `discover-reconcile` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `internet-archive-cdx` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `internet-archive-replay` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `archive-health` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `capture` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `diff` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `emit-evidence-delta` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `validate-process-events` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |

The `capture`, `internet-archive-cdx`, `internet-archive-replay`, `discover` and `discover-bodies` commands are source-network paths. They must use the existing #309 source boundary with versioned acquisition receipts and must not be retired before `fyi-archive` shadow parity.

## Top-level names shared by Python and Rust
| Name | Initial inspection | Required evidence |
|---|---|---|
| `init-db` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `import-authorities` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `list-authorities` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `register-request` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `list-requests` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `set-status` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `request-timeline` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `export-requests` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `import-requests` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `build-prefilled-url` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `ingest-feed` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `reconcile-events` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `fetch-request-page` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `export-process-events` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `attention-report` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `handover` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `dashboard` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `run-cycle` | Rust match arm currently prints a placeholder (not functional parity) | Independent output/state equivalence, failure semantics and side-effect parity |
| `scheduler` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `serve` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `request-detail` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `export-request` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `follow-up-draft` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `attachment-manifest` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `attachment-manifest-csv` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `follow-up-variants` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `follow-up-pack` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `triage-report` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `response-analysis` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `next-best-action` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `correspondence-pack` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `export-bundle` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `show-settings` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `privacy-audit` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |

## Rust-only names
- `instances`: native-only; not a reason to retire a Python command.
- `mfa`: native-only; not a reason to retire a Python command.
- `sync`: native-only; not a reason to retire a Python command.
- `deadline`: native-only; not a reason to retire a Python command.
- `search`: native-only; not a reason to retire a Python command.
- `federation`: native-only; not a reason to retire a Python command.
- `provenance`: native-only; not a reason to retire a Python command.
- `mcp-server`: native-only; not a reason to retire a Python command.
- `tui`: native-only; not a reason to retire a Python command.
- `verify-acquisition-receipt`: native, offline verification of a Python-compatible v1 receipt, not yet an acquisition writer.

## Contract-driven migration rules
1. Capture checked-in Rust `--help` and Python `--help` command manifests on the release runner; compare flags, defaults, stdout/stderr, exit codes, JSON schemas, SQLite mutations, filesystem effects, network effects and retries.
2. Prove semantic equivalence on deterministic fixtures before changing the user-facing executable name. No live network operations in ordinary CI.
3. Preserve the three Python distribution scripts while legacy consumers remain. Publish explicit replacement/version migration notes; do not silently swap `fyi-cli` Python packaging for Rust.
4. Track each command as `unimplemented`, `stub`, `partial`, `parity_verified` or `retired`. Only `parity_verified` commands can advance the deprecation ledger.
5. Preserve the established `source-adapter-boundary-20260812` workstream as the exclusive owner of source transports and receipts.
6. Follow up with a finer-grained option/IO matrix and audit the `fyi-mcp` tools separately (not equivalent to CLI commands).

## Recommended implementation order
- Source boundary receipt verification and archive command restoration in Rust.
- Working DB/query request and authority handlers currently represented by placeholders.
- Offline reporting/export functions with golden snapshots, then monitoring/scheduler and server.
- Only then package-level deprecation/removal of legacy Python, with downstream integration evidence.
