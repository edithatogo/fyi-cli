# Python ↔ Rust CLI parity inventory (2026-11-13)

Scope: top-level parser names in `src/fyi_system/cli.py` and
`crates/fyi-cli/src/main.rs`, reconciled with
`conformance/cli/command-name-parity.json` after PR #410. This is **not** a
claim of runtime or output equivalence.

## Coverage summary
- Python: **48** top-level subcommands, exposed through **three** Python package executables (`fyi`, `fyi-cli`, `fyi-system`).
- Rust: **44** top-level clap variants, exposed as the native `fyi-cli` binary (plus `fyi-mcp` separately).
- **34 names overlap**; **14 Python commands have no matching Rust top-level name**; **10 Rust commands have no matching Python top-level name**.
- **Important:** matching names are **syntax-only potential parity**. Unimplemented Rust commands return an error rather than reporting a successful operation.

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
| `historical-index` | Existing #314 source boundary | Preserve the approved index and receipt contracts if retained |
| `source-probe` | Existing #313 source boundary | Preserve bounded probing and pacing if retained |
| `archive-health` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `capture` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `diff` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `emit-evidence-delta` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |
| `validate-process-events` | `fyi-core` acquisition/archive service + thin `fyi-cli` handler | Offline golden input/output, fixtures, privacy/SSRF budget, receipt + error parity |

The `capture`, `internet-archive-cdx`, `internet-archive-replay`, `discover` and `discover-bodies` commands are source-network paths. They must use the existing #309 source boundary with versioned acquisition receipts and must not be retired before `fyi-archive` shadow parity.

## Top-level names shared by Python and Rust
| Name | Initial inspection | Required evidence |
|---|---|---|
| `init-db` | Initializes the Rust schema and rejects legacy Python databases; migration is not implemented | Independent output/state equivalence, failure semantics and side-effect parity |
| `import-authorities` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `list-authorities` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `register-request` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `list-requests` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `set-status` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `request-timeline` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `export-requests` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `import-requests` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `build-prefilled-url` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `ingest-feed` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `reconcile-events` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `fetch-request-page` | Rust guarded HTTP read and receipt/byte output implemented; **partial parity**, Python endpoint/SQLite/output semantics differ | Offline response, DB side-effect and CLI/error cross-language parity |
| `export-process-events` | Named Rust handler exists; implementation equivalence not established | Independent output/state equivalence, failure semantics and side-effect parity |
| `attention-report` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `handover` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `dashboard` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `run-cycle` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `scheduler` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `serve` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `request-detail` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `export-request` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `follow-up-draft` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `attachment-manifest` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `attachment-manifest-csv` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `follow-up-variants` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `follow-up-pack` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `triage-report` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `response-analysis` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `next-best-action` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `correspondence-pack` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `export-bundle` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `show-settings` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |
| `privacy-audit` | Unimplemented in Rust; exits with an error | Independent output/state equivalence, failure semantics and side-effect parity |

## Rust-only names
- `instances`: native-only; not a reason to retire a Python command.
- `mfa`: native-only; not a reason to retire a Python command.
- `sync`: native-only; not a reason to retire a Python command.
- `deadline`: native-only; not a reason to retire a Python command.
- `search`: native-only; not a reason to retire a Python command.
- `federation`: native-only; not a reason to retire a Python command.
- `provenance`: native-only; not a reason to retire a Python command.
- `mcp-server`: unimplemented CLI shortcut; the separate `fyi-mcp` binary is the MCP server.
- `tui`: unimplemented CLI shortcut.
- `verify-acquisition-receipt`: native, offline verification of a Python-compatible v1 receipt and optional payload bytes; not an archive cutover.

## Contract-driven migration rules
1. Capture checked-in Rust `--help` and Python `--help` command manifests on the release runner; compare flags, defaults, stdout/stderr, exit codes, JSON schemas, SQLite mutations, filesystem effects, network effects and retries.
2. Prove semantic equivalence on deterministic fixtures for the capabilities selected for retention. Explicitly retire unneeded functionality rather than automatically porting every command. No live network operations in ordinary CI.
3. Preserve the three Python distribution scripts while legacy consumers remain. Publish explicit replacement/version migration notes; do not silently swap `fyi-cli` Python packaging for Rust.
4. Track each command as `unimplemented`, `stub`, `partial`, `parity_verified` or `retired`. Only `parity_verified` commands can advance the deprecation ledger.
5. Preserve the established `source-adapter-boundary-20260812` workstream as the exclusive owner of source transports and receipts.
6. Follow up with a finer-grained option/IO matrix and audit the `fyi-mcp` tools separately (not equivalent to CLI commands).

## Recommended implementation order
- Source boundary receipt verification and archive command restoration in Rust.
- Working DB/query request and authority handlers currently represented by placeholders.
- Offline reporting/export functions with golden snapshots, then monitoring/scheduler and server.
- Only then package-level deprecation/removal of legacy Python, with downstream integration evidence.
