# Localization acceptance evidence

## Reconciliation and scope — 2026-11-14

Baseline `062c264` contained six i18n smoke tests, hardcoded greeting/term maps,
English-only body scaffolds and a duplicate weekday loop. It had no Fluent
dependency, versioned resources, typed request contract, holiday data or translation
review evidence. Its completed metadata and shared registry description were stale.
Only this track's owned metadata/plan are corrected; shared registry reconciliation
is deferred to the coordinating workstream. The track is not archived.

The latest master `a816dcd` (PR #431) was reconciled before the dependency commit.
The two new direct dependencies and their lock entries are the only shared Cargo
changes. No `lib.rs`, database, sync, adapter, receipt, CLI dispatch or workflow edit
belongs to this increment.

## Increment 1: locale and message foundation

- Tests first: commit `bc2c804` defines the contracts. The production command
  `cargo test --locked -p fyi-core --test i18n_regressions` failed with two expected
  failures and one pass: unknown `ja-JP` returned Spanish; `EN_nz` lost the NZ
  greeting. `i18n_messages` initially failed to compile because the typed APIs did
  not exist. No implementation preceded those failing checks.
- Green: `cargo test --workspace --all-features --locked` passed **389 tests**
  across 26 result groups, including 16 localization integration tests, the existing
  six i18n unit tests and drafting regressions. The six-locale message fixture is
  hand-authored and asserts exact locale membership, Unicode greetings, terms,
  sign-offs and zero/singular/plural day labels.
- Formatting: `cargo fmt --all -- --check` passed.
- Linting: `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  passed, using the repository-pinned Rust 1.99.1 toolchain.
- Conductor: `python3 scripts/verify_frontier_conductor.py` passed.
- Diff whitespace checks passed. Package-list inspection verifies embedded locale
  resources are included in `fyi-core` packaging.
- Independent read-only review identified an attribute-only-message fallback
  ambiguity. Resource lookup now distinguishes message presence from usable value;
  a present malformed message reaches the strict renderer and fails.

### Dependency decision

`fluent-bundle` 0.16 and `unic-langid` 0.9 materially replace ad hoc string/locale
handling with maintained parsers. Tests justify their use through structural
identifier normalization, complete-resource validation, duplicate rejection,
missing-variable errors, cyclic/unresolved expression rejection and literal
interpolation. No callback functions are registered. Fluent formatting errors are
checked; partial output is never accepted. Message fallback is implemented explicitly.
User content is not NFC/NFKC-normalized or parsed recursively.

### Required hosted merge evidence

Before merging, inspect the exact final PR head and require all applicable mandatory
GitHub Actions to pass. Preserve the 90% workspace Rust coverage gate, security
checks, mutation thresholds and source-boundary checks. Local results do not replace
hosted final-head evidence. PR/head/run identifiers are appended as verified.

## Outstanding acceptance criteria

- Phase 2 typed templates and variants; complete non-English bodies.
- Terminology/glossary registry beyond the existing catalog request terms.
- National/regional/observed holidays and reviewed statutory counting rules.
- Configurable external resource overrides and adding locales without code changes.
- Human review of translations, register, citation wording and legal applicability.
- Measured template/deadline performance and the track's 95% i18n coverage target.

Manual-verification plan: inspect the golden fixtures and run the listed tests;
check the rendered wording with language/jurisdiction reviewers. No manual or legal
approval is inferred from the automated checks, and the corresponding original
checklist items remain unchecked. Reverting this increment changes localization
only; it has no persistence, migration, remote-write or submission effect.
