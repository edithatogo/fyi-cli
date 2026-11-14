# Localization acceptance evidence

## Reconciliation and scope — 2026-11-14

Baseline `062c264` contained six i18n smoke tests, hardcoded greeting/term maps,
English-only body scaffolds and a duplicate weekday loop. It had no Fluent
dependency, versioned resources, typed request contract, holiday data or translation
review evidence. Its completed metadata and shared registry description were stale.
Only this track's owned metadata/plan are corrected; shared registry reconciliation
is deferred to the coordinating workstream. The track is not archived.

Master `a816dcd` (PR #431) was reconciled before the dependency commit;
`797e76c` (PR #433) was then merged unchanged to satisfy the strict up-to-date gate.
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
- Coverage: Rust 1.99.1 / cargo-llvm-cov 0.9.1 measured **99.375% i18n.rs line
  coverage** (318/320) and **92.5052% workspace line coverage** (8,899/9,620)
  using the existing CI exclusions and unchanged 90% gate. The run passed.
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
hosted final-head evidence. PR [#436](https://github.com/edithatogo/fyi-cli/pull/436) carries the increment.
  Initial head `8810332` passed hosted Code Coverage; final checks rerun after
  reconciliation with the newly merged quality-workflow baseline.

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
