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

- Terminology/glossary registry beyond the existing catalog request terms.
- National/regional/observed holidays and reviewed statutory counting rules.
- Configurable external resource overrides and adding locales without code changes.
- Human review of translations, register, citation wording and legal applicability.
- Measured template/deadline performance (the 95% i18n coverage target is now met).

Manual-verification plan: inspect the golden fixtures and run the listed tests;
check the rendered wording with language/jurisdiction reviewers. No manual or legal
approval is inferred from the automated checks, and the corresponding original
checklist items remain unchecked. Reverting this increment changes localization
only; it has no persistence, migration, remote-write or submission effect.

## Increment 2: typed letter foundation and deadline boundary

The second test-first checkpoint (`87e6266`) contains the
French-body regression and typed template/deadline contracts. The French legacy
scaffold test failed because its body was still English. Typed template and checked
deadline tests failed to compile before those APIs were implemented. A further
Unicode-separator regression failed before single-line validation was corrected.

The implementation adds `RequestTemplateInput`, catalog-borrowing `LegalCitation`,
optional `ReviewInformation` and `RenderedRequest`. All four variants (initial,
follow-up, internal review, appeal) and both registers have translated resources
for the six baseline locales. A 48-combination test checks complete message-source
coverage, literal Unicode body/title handling and determinism; six hand-authored
complete-letter goldens assert exact output against existing jurisdiction fixtures.
Initial request titles are retained; other variants add localized prefixes.

Missing fields, line-breaking/control characters in single-line fields, malformed
optional values and absent appeal contacts fail. Plain text preserves content;
HTML text escapes title/body exactly once. The only representable validation state
is `rendered_unvalidated`; deserialization of a claimed legally validated state
is rejected. No request is persisted or submitted.

The old weekday loop and duplicate instance-day policy were removed from i18n.
Existing methods now delegate to `deadlines.rs`; new fallible adapters guard the
representable date/count range first. Nine integration tests verify locale-invariant
parity, default/negative catalog policy, overflow rejection and exact capacity near
the maximum date. This is representation safety, not statutory verification.
The canonical direct deadline APIs and extreme valid-period runtime remain owned
by the separate deadline workstream. Legal reliance is blocked pending reviewed
holiday/time-unit/extension/receipt rules; no legal-calendar completion is claimed.

### Verification

- Full workspace tests passed before the final Unicode guard: 408 tests including
  doctests. After the fix, 21 focused template/deadline integration tests passed;
  the complete instrumented workspace run passed 410 tests (one ignored), with
  the same current production-file hashes before and after the run.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passed.
- `python3 scripts/verify_frontier_conductor.py` and diff whitespace checks passed.
- Coverage with Rust 1.99.1 and cargo-llvm-cov 0.9.1, exact existing CI exclusions:

  | Scope | Covered lines | Line coverage |
  | --- | --- | --- |
  | `i18n.rs` | 365 / 367 | 99.4550% |
  | `i18n/templates.rs` | 181 / 181 | 100% |
  | Localization combined | 546 / 548 | 99.6350% |
  | Workspace | 9,124 / 9,848 | 92.6483% |

  The unchanged workspace 90% gate passed. No new exclusions were added.
- Independent review found and prompted a test-first fix for U+2028/U+2029 in
  single-line fields. It found no remaining representation-boundary, escaping,
  jurisdiction-ownership or arbitrary-execution defect in this increment.

### Source identity for measured coverage

```text
i18n.rs: 74c2f4d6314338df9667c8719c9ade72526315e7e17f1ae51d469c5f7fbebee6
i18n/templates.rs: 586608a106c807302afa1e67a7aa7bd7091d8daa99f9f2a899418eb233cf23fa
```

Manual language/legal review remains unchecked. Phases 1 and 2 implementation and
automated verification are complete; the whole track remains active for the
outstanding original criteria above. Subsequent PR/head/merge evidence is recorded
only after the required final-head checks pass.
