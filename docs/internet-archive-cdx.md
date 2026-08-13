# Internet Archive CDX discovery

`fyi internet-archive-cdx` is the source-network boundary for Internet Archive
CDX inventory discovery. It writes canonical JSON rows, an atomic resumable
checkpoint, and a versioned acquisition receipt. It does not retrieve archived
pages, capture source records, package datasets, or publish data.

```powershell
fyi internet-archive-cdx `
  --url-pattern "fyi.org.nz/request/*" `
  --allowed-host fyi.org.nz `
  --pagination-mode resume_key `
  --page-size 1000 `
  --max-pages 100 `
  --max-rows 1000000 `
  --max-runtime-seconds 1800 `
  --max-stall-seconds 300 `
  --output evidence/fyi-cdx.json `
  --checkpoint evidence/fyi-cdx.checkpoint.json `
  --receipt evidence/fyi-cdx.receipt.json
```

The endpoint is fixed to `https://web.archive.org/cdx/search/cdx`; callers
cannot override it. `--url-pattern` is scheme-free and must use exactly the
normalized DNS hostname supplied through `--allowed-host`. Redirects, changed
response URLs, credentials, ports, encoded path components, query strings, and
parent path segments fail closed.

Two bounded pagination modes are available:

- `resume_key` follows `showResumeKey` cursors and detects repeated cursors,
  repeated chunks, progress stalls, row overflow, and a missing terminator.
- `page_count` verifies `showNumPages`, detects count drift, repeated pages,
  premature empty pages, row overflow, and a missing terminator.

Both modes use a whole-run deadline, bounded retries, capped `Retry-After`
delays, a 16 MiB response cap, stable row ordering, and canonical SHA-256
fingerprints. After each accepted page or chunk, the command atomically writes
a self-digested checkpoint containing the accumulated rows and next position.
Restart the same command to resume. Query-shape changes or checkpoint tampering
are rejected before a network request. Operational page and row caps may be
increased to continue a verified partial traversal, but cannot be reduced below
accepted progress. A complete checkpoint can regenerate a missing output
without network access.

The output is replaced only after traversal completes. A failed run preserves
the previous output, retains the last valid partial checkpoint, and atomically
writes a failure receipt containing the exception type without its message.

The offline fixtures in `tests/fixtures/internet_archive_cdx/` preserve the
page-count and resume-key row shapes used by the former `fyi-archive` CDX
client. Live parity and archive cutover remain separate hosted acceptance gates.
