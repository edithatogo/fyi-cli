# Retained local authority and request management

Date: 2026-11-14
Parent track: `frontier-rust-convergence-20261113`
Status: implementation contract; capability acceptance remains pending

## Ownership and scope

This document refines the existing parent track's retained local request-tracking
scope. It does not create another track or change the shared Conductor registry.
It covers local authority CSV import/listing, request registration/listing, local
status changes, request timeline/detail, request export/import and explicit
handling of existing Python SQLite data.

The `source-adapter-boundary-20260812` workstream owns source acquisition,
receipts, CDX/Wayback, network transports and archive integration. The
`quality-frontier-testing-cicd-20261113` workstream owns CI/CD and testing
infrastructure. Localisation owns `i18n.rs`, locale resources and request-template
language support. This increment must not modify those implementations or make
their completion a prerequisite for local functionality.

No command in this scope performs network operations, prepares remote submission
queue entries, or marks local tracking records dirty for remote synchronization.
Providing an existing external request identifier only records a local link.

## Reconciled baseline

The inspected baseline is master commit
`062c2647e9dc5300fee249cdd7b81a3d164807bc`. PR #431 was open at the initial
reconciliation; its shared registry and execution-order changes remain owned by
the parallel source workstream. Recheck master and active PRs before each merge.

The nine retained command names already exist in the Rust parser but dispatch to
`unsupported_command`. Name overlap is not behavioural parity. The baseline
evidence is `docs/rust-python-command-parity-20261113.md` and
`crates/fyi-cli/src/main.rs`.

`fyi-core::db::DbPool` supplies the existing SQLx pool, pool accessor and embedded
migrations. Its `requests` and correspondence APIs implement the remote API and
sync domain. `insert_request` and `update_request` create dirty sync metadata;
`SyncClient::push_dirty_with_config` can submit those records. Those mutators
must not be used for local registration or local status changes.

The Python `authorities` and `tracked_requests` schemas in
`src/fyi_system/db.py` define a separate local-tracking domain. A tracked record
has a local ID, authority slug and optional external request ID, as well as exact
tag text and last-observation metadata. These fields cannot be represented
losslessly by the existing `AlaveteliRequest` alone.

## Storage and API contract

- Reuse `DbPool`, SQLx, the embedded migration mechanism and existing
  serialization/error libraries. Add local repository methods within `fyi-core`.
- Retain the Python-compatible `authorities` and `tracked_requests` table shapes.
  Do not copy local records into remote `requests` or remote sync tables.
- Preserve authority slugs as keys and positive signed 64-bit local request IDs.
  Use SQLite allocation for new registration; retain explicit IDs during
  versioned import and migration. Do not derive IDs from titles or bodies.
- Preserve the distinct optional external `fyi_request_id`; it is never a local
  primary key or evidence of permission to submit remotely.
- Keep tag text and nullable URL/observation fields losslessly. Do not silently
  split or normalize stored legacy tags.
- Add an append-only local status history table with a foreign key to the local
  tracked request. History records include a stable sequence, previous/new
  status, timestamp and operation provenance. Initial registration records a
  creation event; migration must not invent historical transitions.
- Enable foreign-key enforcement for every pooled connection. Use one
  transaction for every compound mutation, including history and provenance.
- Read commands open an existing database without file creation or migration.
  Missing, uninitialized and unsupported databases return explicit errors.
- New schema changes are additive migrations. Never edit already published SQLx
  migration files or change the remote sync schema for this workstream.

## Delivery sequence

Each slice starts with executable failing contract/negative tests, then an
implementation and focused PR. Record test commit and hosted red/green evidence
when local Rust tooling is unavailable; absence of local tooling is not a pass.
Merge only when required checks pass for the exact final head and the branch is
current with master. Verify the merge and prune the merged branch before the
next slice is reconciled.

### Slice 1: local authorities

- [x] Implement local CSV parsing and atomic authority upsert.
- [x] Implement deterministic authority listing and text/JSON output.
- [x] Prove repeated import, invalid-input rollback and request-reference safety.

`import-authorities CSV_PATH --db PATH` retains the existing local CLI shape.
Header aliases and precedence follow `src/fyi_system/importers.py`:
`url_name`/`slug`/`authority_slug`, `name`/`authority_name`/`public_body_name`, and
`url`/`authority_url`/`request_url`. Handle UTF-8 BOM, quoted commas, doubled
quotes, embedded newlines and Unicode. Whitespace around selected authority
fields is trimmed. Missing slug or name follows the independently recorded
compatibility decision; malformed CSV and invalid encoding fail atomically.

Upsert uses the slug key and preserves dependent tracked requests. Reimporting
the same records does not create duplicates. Preserve the reference processed-row
count convention, including repeated slugs, if claimed as output parity.
Do not use `INSERT OR REPLACE`, which can break dependent references.

`list-authorities` returns `slug`, `name`, and nullable `url`. Order by name with
slug as a deterministic tie-break. Its normal text projection retains the
reference tab-separated fields. JSON is a documented Rust addition because the
Python command does not have equivalent output-format handling.

### Slice 2: local request lifecycle

- [x] Implement registration and request listing.
- [x] Implement atomic status/history changes with stable no-op behaviour.
- [x] Implement local timeline and full local request detail.
- [x] Prove privacy, missing-record, ID-width, transaction and no-submission
  contracts.

`register-request AUTHORITY_SLUG TITLE BODY` retains `--tags`, `--status`,
`--fyi-request-id`, and `--db`. Require an existing authority and nonempty title
and body. IDs use `i64` throughout parsing, storage and output. Default status is
`draft`; registration creates one local record and its creation event in the
same transaction. Repeating registration creates another request, matching the
reference operation; do not silently deduplicate distinct requests by content.

Statuses remain arbitrary nonempty bounded strings. Document and test the byte
limit at its boundary. Do not invent a transition graph or apply the remote
Alaveteli status validator to local tracking. Migration preserves historical
status strings under its explicit compatibility policy.

`list-requests` uses the reference ascending local-ID order and the projection
`id`, `authority_slug`, `title`, `status`, `fyi_request_id`. Normal lists omit
request bodies. JSON fields have stable types and explicit nulls for optional
values; text uses the existing tab-separated projection.

`set-status REQUEST_ID STATUS` changes only local status and update time and
appends a history entry atomically. The previous status is recorded. Repeating
the current status is a successful no-op: no timestamp churn and no duplicate
history. A missing request fails. These two safety semantics differ from the
reference unconditional update and must be documented as partial compatibility.

`request-timeline REQUEST_ID` presents ordered local events with the reference
`ts`, `kind`, `title`, `detail` projection where applicable. Its deterministic
ordering uses parsed timestamps and stable event sequence for ties. Preserve
legacy timestamp strings in output. Existing feed events and request snapshots
may be read locally when supported, but must never be fetched or rewritten.
Do not claim full Python timeline parity until their projections are covered by
fixtures. Missing request behaviour is specified and tested explicitly because
the Python helper currently returns an empty list.

`request-detail REQUEST_ID` returns the full local tracked-request projection,
including body, tags, local/external IDs, status, original timestamps, URL and
last-observation fields. It does not fabricate follow-up, response-analysis,
snapshot or next-action results. Python's detail command adds those derived
fields; the Rust local detail command remains partial parity until each retained
derived field is implemented and independently verified. Local status history
may be exposed as a documented additive field.

### Slice 3: local transfer and legacy migration

- [ ] Define and test a strict versioned local export contract.
- [ ] Implement deterministic private export and transactional import.
- [ ] Implement explicit legacy migration while retaining default fail-closed
  initialization for unsupported databases.
- [ ] Prove round trip, repeated import, collision policy, replacement rollback,
  legacy-data preservation and schema-version rejection.

The versioned export contract must preserve authorities, tracked-request IDs and
fields, and local status/provenance records needed for a lossless local-state
round trip. Use stable ordering and no volatile export timestamp in canonical
content. Include a schema version; reject unsupported versions before any
database mutation. Publish the concrete fixture and field-level contract before
implementation. A plain legacy JSON request array is a separately recognized
input format, not an implicitly versioned export.

Export includes sensitive bodies only because the operator explicitly requested
the export. Create private files, use atomic publication and protect existing
destinations according to the documented overwrite policy. Do not include
credentials, remote queue payloads, arbitrary cache content or unrelated tables.

Versioned import retains IDs. Identical records are idempotent; conflicting
records fail unless the documented explicit replacement mode permits them.
Duplicate IDs inside one document are errors. Validate all records, references,
timestamps and schema versions before committing. `--replace` operates only on
the local tracking domain and performs validation, deletion and insertion as one
transaction. It must not orphan retained legacy provenance or touch remote sync
records. Preserve and test SQLite ID allocation after explicit-ID imports.

Python legacy import ignores exported IDs and appends duplicates. Rust's retained
IDs and idempotency are intentional compatibility differences, which must appear
in the command ledger. The handling of absent legacy IDs is specified before
that format is enabled; do not guess or silently remap provenance links.

Legacy database migration is explicit, for example through a dedicated
`init-db` migration option whose actual syntax is frozen with its tests. Ordinary
read commands never migrate. Recognize the complete known Python table/column
layout; reject incomplete or unsupported schemas and orphaned references without
changing the source data. Preserve IDs, nullable fields, exact timestamps,
authority links, feed/snapshot references and unrelated existing tables. Record
schema adoption and new migrations transactionally. Repeating migration is a
stable no-op. Do not manufacture historical status events where evidence does
not exist.

## Executable acceptance matrix

| Concern | Minimum executable evidence |
|---|---|
| Arguments and exits | Real CLI tests for defaults, missing/invalid arguments, global/subcommand database selection, positive `i64` IDs, unknown records and JSON output |
| Authority CSV | Independent fixture for alias precedence, BOM/Unicode, multiline quoting, null URL, repeated slug, malformed rows and atomic rollback |
| Local storage | SQL assertions on authorities, tracked requests, history and all remote sync/queue tables after registration/status/import |
| Transactions | Inject a failing later SQL operation or trigger and prove no earlier mutation or history survives |
| Repetition | Stable authority upsert, status no-op, versioned import and migration; explicit repeat-registration behaviour |
| Sensitive fields | Bodies absent from lists and errors; included only in explicit detail/export; private database/export files |
| Timeline | Stable timestamp ordering, deterministic ties, preserved original timestamp strings and no invented events |
| Existing data | Complete Python fixture, old Rust database, malformed legacy schema, orphan reference and unknown schema-version cases |
| Export/import | Exact field/ID/provenance round trip, duplicate/collision rejection, future version rejection and replacement rollback |
| No remote work | Local records absent from dirty sync state and outgoing queues; no acquisition client constructed by local handlers |

Use synthetic data only. Assertions must inspect observable CLI and SQLite
effects, not merely function names or implementation source strings. Fixtures
must identify whether they describe Python-compatible behaviour or a documented
Rust safety extension. Tests do not access the network.

## Verification and compatibility ledger

Completed delivery and pending acceptance are distinguished below. The parent
track remains in progress; this document does not close it or authorize Python
retirement.

| Slice | Failing contract commit/run | Implementation commit | Final-head checks | PR / merge / branch pruning |
|---|---|---|---|---|
| Authorities | `c9f7db3`; run `43552118106`, 2 expected failures | `33b043d`; MSRV `43553220643` | `34feb0a`: all 12 protected contexts and 32 applicable checks passed | #434 merged as `7cf2fd2`; remote/local branch pruned |
| Request lifecycle | `ddb1b3e`; run `43553385063`, 11 expected failures | `cd160e8`; CI `43558451180`, MSRV `43558451176` | Green implementation checkpoint; final reconciled-head acceptance pending | #440; authority prerequisite merged, lifecycle merge pending |
| Export/import and migration | Pending | Pending | Pending | Pending |

Required local commands are targeted Rust tests, `cargo fmt --all -- --check`,
and the repository Clippy invocation with warnings denied. Hosted final-head
required Actions remain authoritative, including the existing 90% Rust coverage
gate and mandatory security checks. No threshold or exclusion changes are part
of this workstream.

| Command | Baseline state | Completion evidence required |
|---|---|---|
| `import-authorities` | Unimplemented | Local CSV, CLI, transaction and repeat-operation fixtures |
| `list-authorities` | Unimplemented | Exact text/JSON projection and deterministic ordering |
| `register-request` | Unimplemented | Local-only registration, FK/ID/privacy and atomic creation history |
| `list-requests` | Unimplemented | Minimal projection, ordering and read-only database opening |
| `set-status` | Unimplemented | Atomic history, no-op stability and explicit compatibility differences |
| `request-timeline` | Unimplemented | Local events plus separately documented legacy feed/snapshot coverage |
| `request-detail` | Unimplemented | Full local projection; derived Python fields tracked separately |
| `export-requests` | Unimplemented | Versioned, deterministic, private, lossless local export |
| `import-requests` | Unimplemented | Strict versions, IDs/references, idempotency and full rollback |

Update each command independently as implemented, partial, unimplemented or
explicitly retired. No command is retired by this contract. Mark functional
Python/Rust parity only for a fixture-proven surface; command-name equality,
successful compilation and a passing happy-path smoke are insufficient.

### Authority implementation checkpoint

PR [#434](https://github.com/edithatogo/fyi-cli/pull/434) starts from test-only head
`c9f7db3c4a64eae1e36d6ca851e58b3418ba71f9`. Hosted workspace test job
[132316713810](https://github.com/edithatogo/fyi-cli/actions/runs/43552118106/job/132316713810)
failed both authority success contracts because `import-authorities` was
unimplemented. One negative CLI test passed. Rustfmt identified formatting-only
changes in that initial test file; those changes were applied from its output.

The Python reference fixture suite was executed locally with the existing
Python 3.12 environment: **11 passed**. This is Python evidence only. Native
core, CLI, formatting, Clippy, security, coverage and package results remain
pending for the final implementation head.

The implementation uses the established `csv` crate (1.4) for decoding, with a
small quote-completeness guard because the parser otherwise accepts a truncated
quoted field. This is one focused parser dependency, not a new storage or
network stack. Its locked checksum and csv-core checksum come from the official
crates.io index; hosted `--locked` compilation must verify the lock. Missing or
blank required fields, ambiguous quoting, duplicate headers, malformed row
lengths, invalid UTF-8 and input beyond 16 MiB fail before database writes.
Unlike Python's skipped invalid rows, failure rejects the complete import.

### Authority native verification

Implementation head `33b043d8b3482447b24790b429b420d5541cedec` passed the
[Rust 1.99.1 MSRV job](https://github.com/edithatogo/fyi-cli/actions/runs/43553220643/job/132320153668):
formatting, all-target/all-feature Clippy with warnings denied and locked
all-feature workspace tests. This includes the core authority and actual CLI
contracts, SQL-trigger rollback, physical-schema validation and read-only
database checks. The Conductor contract job passed in the same run.

SQLx 0.9 rejected dynamic test SQL at the prior head. The tests now use literal
queries and static schema mutation fixtures; no SQL safety assertion bypass was
added. Independent review also identified and fixed adoption of incompatible
pre-existing authority tables. Matching migration checksums now accompany
physical column/primary-key/nullability validation.

PR #433's CI hardening is inherited through a normal merge of master. This
workstream changes no workflow file relative to that base. Final-head required
Actions, coverage/security checks and merge confirmation remain the PR gate.

### Reconciliation after parallel MCP merge

Authority head `e409317988573348eec125366cd52d757fd8ee9c` passed all 12 live
protected-branch contexts in CI [43553737589](https://github.com/edithatogo/fyi-cli/actions/runs/43553737589),
including workspace tests, Clippy, Rustfmt, security and **92.49% line coverage**.
Rust 1.99.1 MSRV and Conductor checks passed in run
[43553737486](https://github.com/edithatogo/fyi-cli/actions/runs/43553737486);
Windows, macOS and Linux portability passed in run
[43553737477](https://github.com/edithatogo/fyi-cli/actions/runs/43553737477).
The optional self-hosted offline AI lane remains queued awaiting a runner; it
is not required by live branch protection and is not reported as passing.

Master subsequently advanced through independently verified MCP PR #438 to
`19f3cf376101500036951f9a3d3733da49f0df8b`. This branch incorporates that
merge without editing its implementation. Final-head mandatory verification
will run again on the reconciled authority head before merge.

### Verified authority delivery

PR [#434](https://github.com/edithatogo/fyi-cli/pull/434) squash-merged on
2026-11-14 as `7cf2fd2a062ecba450637c45a31b8919fa2b86f5` from accepted head
`34feb0af831e4e67d0a0b67cb8660e8b55cadaa5`. All twelve protected contexts
passed with their expected GitHub app IDs, and all 32 applicable checks passed.
Final-head runs include CI
[43560612069](https://github.com/edithatogo/fyi-cli/actions/runs/43560612069),
CodeQL [43560612021](https://github.com/edithatogo/fyi-cli/actions/runs/43560612021),
and MSRV/Conductor
[43560612100](https://github.com/edithatogo/fyi-cli/actions/runs/43560612100).
The optional offline AI lane is not reported as a pass.

GitHub's closed/merged state and fetched master were verified. The squash tree
exactly matched the accepted head. The remote feature ref was deleted and the
local authority branch was pruned after that verification. The lifecycle branch
incorporates the actual authority merge; independently merged MCP/quality and
packaging implementation remains unchanged.

### Verified lifecycle acceptance checkpoint

Head `cd160e8e7d20c37b0e8a23e4eafc71388075faa8` passed all 12 live protected
contexts in CI
[43558451180](https://github.com/edithatogo/fyi-cli/actions/runs/43558451180)
and CodeQL
[43558451223](https://github.com/edithatogo/fyi-cli/actions/runs/43558451223).
Rust 1.99.1 MSRV/Conductor
[43558451176](https://github.com/edithatogo/fyi-cli/actions/runs/43558451176)
passed formatting, all-target/all-feature Clippy with warnings denied and
all-feature workspace tests. Privacy, dependency review, profiling, Python
security/fuzzing, workflow lint and existing portability checks also passed.
The optional self-hosted offline AI lane has no designated runner and remains
queued; it is not claimed as a pass.

This verifies the five lifecycle commands and the prerequisite authorities,
including 21 lifecycle repository tests, two shared-fixture tests, five schema
tests and twelve actual lifecycle CLI tests. New CLI fixtures use an atomic
sequence in their temporary directory names after a real parallel coverage run
exposed a timestamp-only name collision. The Python subprocess fixture now
selects its environment's interpreter when run under Scalene; normal and
profiled fixture cases pass without relaxing stderr or exit assertions.

The actual authority merge is now incorporated. Lifecycle delivery still
requires green checks on its exact reconciled final head. Legacy source
projection and transfer/migration are delivered in
the separate third slice. Python detail's derived report schemas remain outside
the implemented local-record contract.

### Request lifecycle implementation checkpoint

PR [#440](https://github.com/edithatogo/fyi-cli/pull/440) began with test-only
head `ddb1b3ed17819a487b1cb69281bd53f3b65532e1`. Hosted run
[43553385063](https://github.com/edithatogo/fyi-cli/actions/runs/43553385063),
workspace job [132320662126](https://github.com/edithatogo/fyi-cli/actions/runs/43553385063/job/132320662126),
reported **11 failing and 1 passing** real CLI contracts: failures explicitly
reported unimplemented registration/listing or the absent local history table.
The contract head compiled and passed all-target Clippy. Rustfmt corrections
were applied from its diagnostic output before implementation publication.

The implementation adds `register-request`, `list-requests`, `set-status`,
`request-timeline` and local `request-detail` using the established `DbPool`.
It adds Python-shaped `tracked_requests` and a separate local history table.
Registration and status changes acquire an immediate transaction and explicitly
roll back on failure. Detail and timeline share one read transaction. No local
write calls the remote-domain mutators or enters sync metadata or outgoing
queues. IDs remain positive signed 64-bit values. Repeated registration creates
a new identity; repeated status preserves the existing timestamp and history.

Native input bounds are 1024 UTF-8 bytes for authority slugs, 16 KiB for titles
and tags, 16 MiB for bodies, and **128 UTF-8 bytes** for status labels. Required
values reject whitespace-only input; otherwise title/body/tags are preserved
verbatim. Status labels have no invented transition graph, but reject control
characters. JSON acknowledgements contain identity/change data; ordinary lists
omit bodies and tags. Full detail is an explicit sensitive-data read.

Current schema validation checks the known successful SQLx migration prefix,
checksums and actual local column/default/key/autoincrement declarations.
Ordinary commands require the complete current schema and never migrate.
Explicit `init-db` can upgrade a valid prior Rust schema, atomically on the
same connection, and recognizes current native tracking on repetition. Legacy
Python databases remain refused pending the separate transfer/migration slice.

Python fixture verification: **19 tests passed**, including six actual
subprocess exit-code cases; Ruff lint/format passed. Native coverage includes
12 real CLI tests, 21 repository tests, two shared-fixture tests and five schema
regressions. Their hosted implementation results remain pending. The branch
incorporates the authority prerequisite and current master; it stays draft
until authority PR #434 is merged and final-head lifecycle checks pass.

Timeline/detail are local implementations with **partial Python parity**.
Legacy feed/snapshot timeline projection is deferred to the transfer slice;
derived follow-up, analysis and recommendation fields remain outside this
local capability. No placeholders are returned for those unimplemented fields.
