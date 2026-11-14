# Local management reference fixtures

These fixtures define the retained Rust authority, request lifecycle, transfer
and database-migration contracts. They compare observable Python behavior with
the native implementation and record intentional differences. Matching command
names alone does not establish complete Python/Rust parity.

| Commands | Implemented local contract | Fixture or specification |
|---|---|---|
| `import-authorities`, `list-authorities` | Atomic CSV import, deterministic catalogue listing and repeated imports | [Authority reference](authorities-v1-python.json) |
| `register-request`, `list-requests` | Local registration with a separate local ID; summary listing omits bodies and raw evidence | [Lifecycle reference](requests-v1-python.json) |
| `set-status` | Atomic status/history changes; identical status is a no-op | [Lifecycle reference](requests-v1-python.json) |
| `request-timeline` | Local history and existing linked feed/snapshot projections, ordered by chronology | [Legacy SQL](legacy-v1.sql) and [expected projections](legacy-v1-expected.json) |
| `request-detail` | Stored request fields and timeline from one database snapshot; Python derived reports remain partial | [Lifecycle reference](requests-v1-python.json) and [legacy projections](legacy-v1-expected.json) |
| `export-requests`, `import-requests` | Versioned private transfer with original IDs, provenance, allocator state and atomic conflict handling | [Transfer v1 contract](transfer-v1.md) and [native example](requests-v1-transfer.json) |
| `init-db`, `init-db --migrate-legacy` | Native initialization/upgrade and explicit atomic adoption of the known Python schema | [Legacy SQL](legacy-v1.sql), [expected state](legacy-v1-expected.json) and [migration contract](transfer-v1.md#existing-databases) |

All fixture values are synthetic. The local commands do not acquire source data
or submit requests to external platforms. `register-request` only records a
local request, including when `--fyi-request-id` or `--status` is supplied.

## Local authorities

[authorities-v1-python.json](authorities-v1-python.json) freezes local CSV import
and authority-list behavior from the Python reference implementation.
[test_local_authority_contract.py](../../tests/test_local_authority_contract.py)
passes each case through the real local command handlers, SQLite persistence and
text output twice. All authorities and URLs are synthetic; no network calls are
needed.

`valid_cases` records UTF-8 BOM handling, CRLF, quoted commas and newlines,
escaped quotes, Unicode, column aliases and their precedence, whitespace trimming,
optional null URLs, duplicate-slug updates, and empty catalogues. `rows` is the
expected database projection ordered by name and then slug. `stdout` is the
exact Python `list-authorities` text output. `imported_count` counts valid input
rows, including repeated slugs and unchanged repeated imports.

`legacy_differences` records observable Python behavior that the Rust contract
deliberately improves. Python skips rows missing a slug or name, even after valid
rows have been applied. Rust must reject those imports atomically. Python orders
authorities only by name; equal-name rows follow insertion order in these fixtures.
Rust uses an explicit slug tie-break. These cases document differences and must
not be mistaken for requirements to preserve the legacy defects.

The fixture format's `schema_version` versions this test document. It is not a
claim that the legacy database or Python request export has a schema-version
marker.

## Local request lifecycle

[requests-v1-python.json](requests-v1-python.json) is shared by the
[Python reference tests](../../tests/test_local_request_contract.py) and
[Rust conformance tests](../../crates/fyi-core/tests/local_requests_conformance.rs).
The three valid registration cases exercise default arguments, verbatim nonblank whitespace,
CRLF/Unicode, and distinct local/external 64-bit IDs. Python tests execute the
real handlers and subprocess entrypoint, checking output, stored fields and exit
codes. [Native core tests](../../crates/fyi-core/tests/local_requests.rs) and
[CLI tests](../../crates/fyi-cli/tests/local_requests_tests.rs) additionally verify
typed JSON, history, rollback, foreign keys, repeated operations and absence of
remote queue changes.

Eight legacy-invalid registration cases are retained as explicit differences,
not acceptance requirements. Native required fields reject whitespace-only
values while preserving valid input bytes. Missing native targets fail with
exit 1; invalid/overflowing numeric arguments fail in Clap with exit 2.
Python overflows instead reach SQLite and exit 1. Native statuses accept
arbitrary nonblank labels up to 128 UTF-8 bytes with no control characters.
Python permits empty labels and silently succeeds on missing status targets.

Python's same-status update rewrites its timestamp; native repetition preserves
both timestamp and event count. Native history records every new local
transition. Existing rows without native history expose only their known
creation/latest-update projections, without inserting invented events.
Native timelines sort parsed timestamps newest-first, using descending event
ID for ties, and preserve original timestamp strings. Invalid legacy time text
is retained and sorted after parseable times. Stored feed/snapshot projection is
specified by the [legacy fixture](legacy-v1-expected.json) and
[provenance tests](../../crates/fyi-core/tests/local_provenance.rs).

`request-detail` returns the twelve stored request fields plus `timeline`.
It does not implement Python's `latest_snapshot`, `follow_up`,
`follow_up_variants`, `follow_up_pack`, `response_analysis` or `next_best_action`
fields, so this command has partial Python compatibility. Timeline and detail
output are JSON in both output modes. `--output-format json` also selects
structured acknowledgements and list output for the other local commands;
registration returns `{"id": 1}` for the first request in a new database.
Successful commands exit 0, local validation/database failures exit 1, and
argument-parser failures exit 2. Local errors go to stderr.

## Transfer and migration

[transfer-v1.md](transfer-v1.md) and the
[native JSON example](requests-v1-transfer.json) define the private versioned
interchange contract. [legacy-v1.sql](legacy-v1.sql) captures the complete Python
schema and synthetic records, including IDs above i32, true NULL fields, raw
Unicode/CRLF data, linked and unrelated evidence, run logs, cache rows and shared
pacing state. [legacy-v1-expected.json](legacy-v1-expected.json) independently
freezes all original schemas, rows, defaults, allocator values and Python
projections. Four Python tests
in [test_local_request_fixture.py](../../tests/test_local_request_fixture.py)
verify this fixture. The [core migration tests](../../crates/fyi-core/tests/local_legacy_migration.rs),
[core transfer tests](../../crates/fyi-core/tests/local_transfer.rs) and
[CLI transfer tests](../../crates/fyi-cli/tests/local_transfer_tests.rs) compare
native behavior against the frozen state and negative contracts.

This is a lossless local-domain transfer, not a whole-database backup. Existing
full database files should be retained when migrating; native export excludes
unrelated legacy/remote tables. The document includes sensitive bodies and raw
linked evidence. Both import and encoded export are bounded to 16 MiB.

Exports are private data, including when their acknowledgement is printed as
JSON. New export files use atomic publication and Unix mode 0600. An existing
identical private destination is accepted unchanged; a different destination
file is not overwritten. Default import accepts identical records without
changes and rejects conflicting IDs. `--replace` explicitly replaces the local
request set and history under the provenance-preservation rules in the
[transfer contract](transfer-v1.md#import-policy).

## Runnable local workflow

With the Rust `fyi-cli` binary on `PATH`, this sequence uses a fresh directory
and synthetic content. The first registration therefore has local ID 1.
`--db` selects the database for each command; export takes `--output`, while
import takes a positional input filename.

```sh
demo_dir="$(mktemp -d)"
cd "$demo_dir"
printf 'slug,name,url\nexample,Example Authority,\n' > authorities.csv

fyi-cli init-db --db local.db
fyi-cli import-authorities authorities.csv --db local.db
fyi-cli list-authorities --db local.db
fyi-cli --output-format json register-request example "Example request" \
  "Synthetic request body." --tags "example,local" --status draft --db local.db
fyi-cli --output-format json list-requests --db local.db
fyi-cli --output-format json set-status 1 waiting_response --db local.db
fyi-cli request-timeline 1 --db local.db
fyi-cli request-detail 1 --db local.db
fyi-cli --output-format json export-requests --output requests.json --db local.db

fyi-cli init-db --db imported.db
fyi-cli --output-format json import-requests requests.json --db imported.db
fyi-cli --output-format json list-requests --db imported.db
```

The imported request keeps ID 1, its original timestamps and local history.
Repeating the same import succeeds without adding duplicate rows or events.
No command in this sequence submits the request remotely.

For an existing supported Python database, retain a full database backup and
explicitly request adoption:

```sh
fyi-cli init-db --migrate-legacy --db /path/to/existing-python.db
fyi-cli --output-format json list-requests --db /path/to/existing-python.db
```

Migration requires an existing regular nonsymlink database file. It validates
the known schema and references before adding native schema and ledger entries
in one transaction. It preserves existing rows and does not invent missing
history. Repeating migration on a valid current native database is a no-op;
ordinary local commands do not silently migrate an old database.
