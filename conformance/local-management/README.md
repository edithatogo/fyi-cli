# Local authority reference fixtures

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
