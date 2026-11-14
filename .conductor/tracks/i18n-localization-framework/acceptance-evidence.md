# Localization acceptance evidence

## Reconciliation and scope — 2026-11-14

Baseline `062c264` contained six i18n smoke tests, hardcoded greeting/term maps,
English-only body scaffolds and a duplicate weekday loop. It had no Fluent
dependency, versioned resources, typed request contract, holiday data or translation
review evidence. Its completed metadata and shared registry description were stale.
The initial increment corrected this track's owned metadata/plan and deferred
shared status reconciliation. The final integration receipt below now records
that correction. The track is not archived.

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

- Additional terminology regimes and human terminology review; the bounded registry
  is implemented by increment 3 below.
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

## Coordinated integration checkpoint — 2026-11-14 05:30 UTC

The resumed work rechecked PR [#436](https://github.com/edithatogo/fyi-cli/pull/436)
at exact head `727b841588591e980b1a6b1eb5dc906b73998048`: all 12 live protected
contexts succeeded with their required GitHub App IDs (Actions 15368; CodeQL
57789). CI run `43554070706` and 30 of 31 check runs succeeded. The remaining
optional self-hosted `offline-ai-security / offline-security` job was queued,
not passed. There were no review submissions or inline review threads. The earlier
automated review at `8810332` reported no findings; localization source, resources,
tests and guide are byte-unchanged between that reviewed head and `727b841`.

Master had advanced to `7cf2fd2a062ecba450637c45a31b8919fa2b86f5` after #434.
#436 was behind and unmerged. Read-only merge-tree inspection was clean, but no
remote head was refreshed. These results verify the implementation checkpoint;
they do not certify a future reconciled head. The sole integration coordinator,
`FYI-CLI — Issue fixes and PR merges`, owns the exclusive final verification/merge
window following source PRs #432 and #445 (or an explicitly deferred source slot).

The existing typed implementation was preserved at
`75d932b545de0fe91aaa89ae16427c86871c517c`; both production hashes above still
match the measured source. Draft PR [#447](https://github.com/edithatogo/fyi-cli/pull/447)
exposes its 19-file localization-only delta against the #436 feature branch.
Do not merge #447 into that feature branch. After #436 merges, retarget #447 to
master, reconcile once in its assigned window, and require exact-final-head checks
and review inspection. This dependent draft does not claim either merge complete.
No manual, translation, legal or calendar approval is inferred from publication.

## Increment 3: explicit jurisdiction terminology — 2026-11-14

Issue #39 was reconciled against master
`9050a6649ebbcf48e6e2919e5bc19bbed4b4f8c0`. The foundation is now merged:
#436 at `a0fa71a373d0a8e2028aca3818666df328ab811b` and #447 at
`4322d475594eeeafaef7adf18a634671b76dd085`. Their dated pending-merge observations
above are retained as history. No open PR duplicated the remaining terminology
work. The original July closures of #64–#67 did not evidence their full acceptance;
those children are reopened with the exact outstanding scope. #63 remains closed.

PR [#450](https://github.com/edithatogo/fyi-cli/pull/450) adds nine explicit regime
profiles and eleven typed concepts, including OIA/FOI/RTI/IFG/FOIA, French CRPA and
Spanish LTAIBG. Primary official references and the date checked are embedded with
each profile and linked in the localization guide. The English/DE/FR/ES glossary
resources share the existing Fluent parser and locale resolver; they serve all
six advertised display locales. Legal names/abbreviations remain literal.

Country, instance and display language do not select a regime. Unknown identifiers
fail without revealing the supplied value. Lookup reports the selected profile,
message locale and independent catalog/message versions. Custom catalogs replace
the selected registry in memory; they neither merge silently nor load executable
code, files or network resources. Input parsing rejects unsupported versions,
unknown fields, duplicates, empty/oversized catalogs, malformed references and
disallowed controls. Output remains plain text and rendered_unvalidated.

### Executable verification and limits

- Test-first remote checkpoint
  [`5e6ad0aa9c9c`](https://github.com/edithatogo/fyi-cli/commit/5e6ad0aa9c9cd8e678afbac1c4a06f2230e29dde)
  contains the contracts without production code. The
  [hosted compiler log](https://github.com/edithatogo/fyi-cli/actions/runs/43594109805/job/132444851656)
  reports E0432/E0433 for the absent i18n::terminology API. This is an observed
  missing-feature compilation failure, not a claim that runnable tests failed.
- Implementation checkpoint
  [`9f34fdae895d`](https://github.com/edithatogo/fyi-cli/commit/9f34fdae895d162e3bac859a28117ef600d57c44)
  passed all **12 initial terminology tests** in the existing instrumented
  workspace run: **670 passed, zero failed, one ignored**, across 44 result
  groups. The matrix exercises 9 profiles × 6 locales × 11 concepts, six
  hand-authored glossaries, fallback provenance, literal Unicode, malformed
  input/privacy, independent catalog versions and concurrent lookup.
- That [coverage run](https://github.com/edithatogo/fyi-cli/actions/runs/43594411469/job/132445817337)
  measured terminology **170/175 lines (97.14%)** and workspace
  **13,975/14,780 lines (94.55%)**. The existing 90% workspace gate and exclusions
  were unchanged. Strict hosted Clippy and Rust formatting also passed.
- Final review adds a thirteenth test for exact UTF-8 byte limits, plus assertions
  that custom-catalog revisions remain distinct and no embedded profiles leak
  into a custom registry. The production source remains unchanged from the
  measured checkpoint. The final PR head must pass all required checks again;
  its exact result is recorded in the PR before coordinator handoff.
- Local formatting, Conductor verification and whitespace checks pass.
  **Local Rust tests and package-list verification are blocked**, not passed:
  the cache lacks the already-merged Fluent dependency. Automatic review rejected
  an index-updating test attempt and a separate dependency-fetch attempt; offline
  Cargo reports the missing cache entry. Existing hosted runners restored their
  dependency cache and supplied the executable evidence above. No dependency,
  manifest or shared-workflow change was made to work around that limitation.

Measured production identities:

```text
i18n.rs: 4f4588f5e3ad9f1990a7ff5267e4125c4cbecd2cdb0cbf128c471ed534a3f4b2
i18n/terminology.rs: c0ed81901e36b5ba3cc07c12af64cd21499013268a2c347cd32f6ac33954bd47
```

The source references support naming/context, not human approval of translations,
legal applicability, remedy availability or deadline correctness. Additional
regimes, holiday/statutory rules, external message overrides, measured performance
and manual acceptance remain open. Deadline/source/platform components are not
modified. Reverting this increment removes the optional glossary API/resources;
it does not change existing letters, instance metadata, database or submission.

### Final terminology integration receipt

The user explicitly authorized merging after the earlier integration hold.
PR [#450](https://github.com/edithatogo/fyi-cli/pull/450) was squash-merged on
14 November 2026 as `70fef298442c72744583deb39c87ed83e005e60b`.
All 12 protected contexts were rechecked on final head
`87178aae1a42da812321b4f3cc4a115038925757`, with their required application IDs,
immediately before the merge. Master remained `9050a66` during that decision.
The merged tree `7c39ab7a9f369958c671133268845371764acba9` is identical to the
verified final-head tree. GitHub deleted the feature branch automatically and
the missing remote ref was verified.

- [Workspace Tests](https://github.com/edithatogo/fyi-cli/actions/runs/43595087056/job/132447775851):
  **672 passed, zero failed, one ignored**, including **13 terminology contracts**.
- [Code Coverage](https://github.com/edithatogo/fyi-cli/actions/runs/43595087056/job/132447775843):
  **671 instrumented tests passed**, one ignored; terminology **173/175 lines
  (98.86%)**, workspace **13,978/14,780 (94.57%)**. The 90% gate and exclusions
  are unchanged.
- [Python Tests](https://github.com/edithatogo/fyi-cli/actions/runs/43595087056/job/132447775838):
  **1,512 passed, two skipped**, including the existing MCP interoperability tests.
- [CodeQL](https://github.com/edithatogo/fyi-cli/actions/runs/43595087117),
  strict Clippy, formatting, package, audit and the remaining required contexts
  passed. Optional self-hosted offline AI remained queued and is not a pass.

These are hosted executable results; the local missing-Fluent-cache limitation
above remains recorded. #64 is complete. Parent #39 was found closed after the
merge and explicitly reopened because its remaining acceptance is incomplete;
#65, #66 and #67 remain open. The shared registry and index now describe this
track as active. Human approval, holidays/statutory accuracy and the remaining
original gates are neither waived nor archived.

The track remains in_progress and archive_allowed remains false. The legacy
shared tracks.md row still says #39 is completed/closed at the checked master;
its reconciliation is explicitly handed to the sole integration coordinator with
the final merge-window request. This PR updates the existing owned track and
GitHub acceptance rather than claiming that stale shared status is evidence.
