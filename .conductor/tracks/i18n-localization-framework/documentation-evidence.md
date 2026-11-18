# Issue #67 localization-guide evidence

Date: 2026-11-17. PR: [#490](https://github.com/edithatogo/fyi-cli/pull/490).

## Scope and checkpoint

Base: `1f0ae6d0b00663d52ccb65904152ca4e933b3486`.
Documentation implementation: `67fe138501fa04f1d06c9149ed5e4d199578c73b`.
Implementation tree: `9c3f0b16773925d5d3c3c40e4347517153d168ad`.

The changes complete the practical usage/contribution instructions requested by
#67: explicit locale registration and fallback policy, template customization,
and holiday-calendar extension. The README now links to the guide. No runtime
source, embedded data, dependencies, schemas, workflows or gates changed.

The earlier guide's calendar snippet referenced undefined `custom_json` and
`rule`, and its one-date Wellington sample advertised a full year of coverage.
The replacement is a complete fictional `ZZ-DEMO` example. The guide also
corrects the implied transition to approved output: the statutory result enum
only supports `Unvalidated`.

## Verification

| Check | Outcome |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `git diff --check` | Passed |
| `python scripts/verify_frontier_conductor.py` | Passed |
| Existing `tests/test_holiday_calendar_catalog_schema.py` | 1 passed |
| Exact Python block in the guide, using its fictional catalog | Passed with Draft 2020-12 `FormatChecker` |
| Guide-relative file links and section anchors | All resolved |
| All nine Rust integration target names listed in the guide | Files exist |
| Six exact Markdown Rust blocks in the bounded harness below | 6 passed, 0 failed, 0 ignored |
| Two independent source reviews | No blocking findings; clarification fixes applied |

The embedded schema test was isolated from unrelated legacy fixtures with:

```sh
uv run --offline --no-project --with pytest --with jsonschema \
  python -m pytest --noconftest -o addopts='' -q \
  tests/test_holiday_calendar_catalog_schema.py
```

The exact Python example was executed with the guide's synthetic JSON saved as
`calendar.json` and an unchanged copy of the schema at the documented relative
path in a temporary directory. It performed no network I/O.

The full local command targeting `i18n_messages`, `i18n_request_templates`,
`i18n_terminology` and `statutory_deadline_contract` did not reach test execution.
Rust compilation was terminated with SIGKILL first in dependencies `syn` and
`serde_core`, then in `rustls` after retrying with one build job and disabled debug
information. These are build-environment failures, not behavioral test failures
or passing workspace evidence. Full integration and the unchanged required
checks are reported on the PR for its exact current head.

## Bounded Rust example execution

The temporary harness used Cargo 1.100.0, rustc/rustdoc 1.100.0 and the six code
blocks from `docs/localization.md` unchanged. Rustdoc compiled and ran them via
`#![doc = include_str!("../localization.md")]`. Its result was:

```text
running 6 tests
test src/lib.rs - (line 56) ... ok
test src/lib.rs - (line 284) ... ok
test src/lib.rs - (line 256) ... ok
test src/lib.rs - (line 490) ... ok
test src/lib.rs - (line 174) ... ok
test src/lib.rs - (line 118) ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

This verifies real localization and deadline code, with the following explicit
scope. The five modules below and all embedded locale resources/`instances.toml`
were copied byte-for-byte. `jurisdiction.rs` retained its original data and
registry definitions before `FoiProvider` (original lines 13–116), with the four
original imports they require. Provider/network implementations were omitted;
no replacement types or behavior stubs were introduced. The actual `reqwest`
crate was used with default features disabled for URL parsing. Network transport,
API/sync/database integration and full-workspace feature coverage are outside
this harness's evidence.

| File | SHA-256 |
| --- | --- |
| `docs/localization.md` | `4a6e87e497973f5198920dcc542b7878676636ca0ba9554e14aa7accf80f8075` |
| `src/i18n.rs` | `4f4588f5e3ad9f1990a7ff5267e4125c4cbecd2cdb0cbf128c471ed534a3f4b2` |
| `src/i18n/templates.rs` | `586608a106c807302afa1e67a7aa7bd7091d8daa99f9f2a899418eb233cf23fa` |
| `src/i18n/terminology.rs` | `c0ed81901e36b5ba3cc07c12af64cd21499013268a2c347cd32f6ac33954bd47` |
| `src/deadlines.rs` | `d3ef827ff4515bfbdbd76ed7461fd7f7947f563050636dc76d71f76b1e64a6c3` |
| `src/deadlines/statutory.rs` | `93684872f82aca8dc310241e1c932efc858f02ac1c42d6a8447823d75316b5b0` |

Module paths in that table are relative to `crates/fyi-core/`. A final comparison
confirmed the guide and copied module bytes had not changed since execution.

To reproduce this bounded check, create a temporary Rust 2021 crate outside the
workspace, set `[lib] name = "fyi_core"`, preserve the source/resource layout
described above, copy the guide to its root, and declare `deadlines`, `i18n` and
`jurisdiction` modules alongside the `doc` attribute. Use these repository-lock
dependencies:

```toml
[dependencies]
anyhow = "=1.0.104"
chrono = { version = "=0.4.45", features = ["serde"] }
fluent-bundle = "=0.16.0"
reqwest = { version = "=0.13.5", default-features = false }
serde = { version = "=1.0.229", features = ["derive"] }
serde_json = { version = "=1.0.154", features = ["float_roundtrip", "raw_value"] }
thiserror = "=1.0.69"
toml = "=0.8.23"
unic-langid = "=0.9.6"
```

Resolve against the implementation checkpoint's lockfile and compare every
resolved name/version/source/checksum tuple. All 126 resolved dependencies in
this execution matched the repository lockfile. With a populated dependency
cache, the command from that temporary crate was:

```sh
CARGO_BUILD_JOBS=1 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo +1.100.0 test --locked --offline --doc
```

## Review and remaining acceptance

Independent reviewers checked locale registration, glossary fallback, message
variables, public APIs, schema/runtime differences, coverage/year rollover,
review status and all documented targets. Their two minor clarity findings were
applied before the implementation checkpoint: name the compatibility instance
catalog explicitly, and tell calendar contributors to update the expected
jurisdiction inventory in the existing Rust test.

These reviews establish source/documentation consistency. They are not human
translation or legal/calendar approvals. The recorded #67 human/locale review
and #66 source/calendar review remain open. #39 and #65 retain their independent
acceptance gates; the parent track stays `in_progress`, with
`archive_allowed: false`. The original manual-verification checkboxes are
unchanged. This evidence does not claim CI or merge completion before GitHub
reports it for the current PR head.
