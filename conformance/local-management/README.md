# Local management reference fixtures

`authorities-v1-python.json` freezes local CSV import and authority-list behavior
from the Python reference implementation. `tests/test_local_authority_contract.py`
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
claim that the legacy database or request export has a schema-version marker.

## Local request lifecycle

`requests-v1-python.json` is shared by `tests/test_local_request_contract.py`
and `crates/fyi-core/tests/local_requests_conformance.rs`. The three valid
registration cases exercise default arguments, verbatim nonblank whitespace,
CRLF/Unicode, and distinct local/external 64-bit IDs. Python tests execute the
real handlers and subprocess entrypoint, checking output, stored fields and exit
codes. Native core/CLI tests additionally verify typed JSON, history, rollback,
foreign keys, repeated operations and absence of remote queue changes.

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
is retained and sorted after parseable times. This does not claim legacy
feed/snapshot projection or Python derived-detail reporting parity.
