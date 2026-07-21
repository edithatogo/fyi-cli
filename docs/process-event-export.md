# Process-event export

`fyi export-process-events` creates a deterministic, public-safe NDJSON projection for `fyi-archive` and `foi-process`.

The contract is versioned as `1.0.0` in `schemas/process-event.schema.json` and `schemas/attachment-metadata.schema.json`. Source order is retained in `source_order`; timestamps are used for analysis but never for reordering. Request titles, message bodies, requester identity, OCR text, attachment names, and attachment bytes are excluded. Attachment rows contain only metadata and an optional public locator/WARC record reference.

Use `--checkpoint` for resumable continuation. A changed source activity with the same source reference retains its `event_id` and increments `revision`. Removed events produce one `operation=delete` tombstone. Re-running against the same checkpoint is a no-op.

Offline benchmark:

python scripts/benchmark_process_events.py

The default fixture contains 1,000 requests and 8,000 events. It uses no network, archive credentials, or publication target.

Example:

```text
fyi export-process-events --derived-dir data/raw/requests --output outputs/process-events.ndjson --attachments-output outputs/attachment-metadata.ndjson --checkpoint data/_state/process-events.json --captured-at 2026-01-01T00:00:00Z
```
