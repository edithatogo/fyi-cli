# Receipt portability implementation plan — #310

**Goal:** Produce repeatable Linux, Windows and macOS evidence for the actual Rust
receipt writer and verifier, without confusing process tests with power-loss proof.
**Architecture:** Keep production code unchanged. Run the existing core/CLI receipt
contracts and a separate-process publication suite on three explicit hosted images.
**Tech stack:** Existing Rust workspace, Serde fixtures, GitHub Actions, standard-library
Python workflow guards. No new application or test dependency.
**Spec:** Existing source-adapter spec and the filesystem/Windows task in `plan.md`.

## Design decisions

The prior Linux-only checks do not establish Windows/macOS filesystem behavior.
Prefer actual workspace builds over a copied/mocked writer harness: tests must link
production code and include the real CLI. Use ubuntu-24.04, windows-2025 and macos-15;
record the exact commit, compiler, runner OS/architecture and complete test logs.
Use bounded jobs/caches, a 45-minute timeout, read-only token permissions and immutable
action pins. These are additional PR checks, not a change to branch-protection settings.
All three platform checks must succeed before this increment is merged.

Tests never acquire live source data. Build dependency retrieval is separate from
runtime acquisition; only synthetic local files and Python-origin fixtures are used.

## Tasks and evidence

- [x] Add six workflow contract tests. Local baseline: six failures because the
  portability workflow was absent. After implementation: six passed; YAML parsed.
- [~] Add `.github/workflows/receipt-portability.yml`, running existing writer,
  wire, numeric/midpoint and actual CLI receipt suites on all three platforms.
- [~] Add `crates/fyi-core/tests/receipt_persistence_portability.rs`: independent
  processes race on identical/different receipts; verify exactly one winner when
  contents differ, canonical integrity and clean publication. A bounded child
  helper is explicitly invoked by its parent tests; its ignored default entry
  point is not a skipped acceptance test.
- [~] Test abrupt process exit *after* successful publication, idempotent restart,
  simulated orphan temporary files, existing-directory/parent-file conflicts,
  dangling/existing symlinks on Windows and POSIX, and POSIX private permissions.
- [ ] Inspect final-head platform logs and existing full CI, MSRV, formatting,
  coverage/security and Conductor checks; repair defects rather than waive failures.
- [ ] Record verified run IDs and the merged commit; reconcile the issue checkpoint.

## Explicit evidence limits

`process::exit` after the writer returns exercises loss of destructors and restart,
not interruption inside `write`, `sync_all` or hard-link publication. The orphan is
an explicitly simulated partial file. Neither proves crash consistency under power
loss, disk-full/short-write/fsync faults, network filesystems or unsupported hard links.
POSIX mode 0600 is tested only where defined; this does not establish a Windows ACL
privacy policy. Unsupported symlink setup fails the platform test instead of being
silently skipped. True fault injection and Windows durability/ACL review remain open.

The issue remains active for receipt coverage across every network command, full
schema/number conformance, raw archive linkage and downstream shadow parity. The
historical plan entries are not permission for archival cutover. In particular,
#426/#428 numeric fixes and #429 strict wire ingestion are already merged increments;
they do not complete the programme. Local Rust is unavailable in this environment:
hosted checks supply actual Rust execution evidence, not a claimed local build.
