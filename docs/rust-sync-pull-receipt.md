# Receipt evidence for a single sync pull

The Rust CLI supports an acquisition receipt for one incremental API list
request using `fyi-cli sync pull --receipt <receipt> --raw-file <response>
--result-file <result>`. All three files are required and must be new, distinct
from one another and from the SQLite database and its sidecars. The database
must be specified as a filesystem path for this mode. The command rejects
symlinked paths, nested evidence paths and collisions before opening the
database or contacting the source.

The dedicated client makes one read-only GET, does not follow redirects, retry
(including reqwest's protocol-level retries),
reuse a cache or decompress the HTTP body, and bounds that body to 16 MiB. It
requests identity encoding and rejects an encoded response it cannot parse. A
complete response is saved before database updates. Its actual status, redacted
URL, length and SHA-256 are recorded separately from the canonical result
projection; the stored incremental boundary is represented by a digest. Use
`fyi-cli verify-acquisition-receipt <receipt> --raw-file <response>
--result-file <result>` to check both files offline.

A complete HTTP error, redirect or parse/application failure produces a failed
receipt when the evidence can be published. An incomplete exchange cannot be
represented honestly by receipt v1 and produces no receipt. A filesystem error
or interruption can also leave partial evidence. Applying a list currently
updates database records individually, so an application or publication error
can leave partial database changes; the error result does not claim a count of
completed changes. Inspect the database and preserved evidence before resuming.

## Incremental boundary lineage

The saved result contains `checkpoint.before` and `checkpoint.after` snapshots:

```json
{
  "schema": "urn:fyi-cli:sync-boundary:1",
  "updated_since_sha256": null
}
```

The value is null when no stored boundary exists; otherwise it is the SHA-256
of the exact UTF-8 stored boundary, without exposing that text. Each receipt
checkpoint digest hashes the corresponding snapshot using the existing
`canonical_json_bytes` contract, including its trailing LF. The result file's
own digest binds both snapshots, so an offline consumer can recompute the two
checkpoint hashes from the saved result and compare them to the receipt.

The final boundary is read even when applying the response fails partway.
Consequently a failed receipt can truthfully show an advanced cursor, without
claiming that all database changes succeeded. If the final boundary cannot be
read or bounded, the raw response is retained and no result or receipt is
published. These snapshots describe observed database state; they do not create
a transaction or establish exclusive causation when other writers are active.

## Rate-limit observations and filesystem boundary

The result's `rate_limit_observation` contains only the parsed numeric `limit`,
`remaining`, `reset_seconds` and `retry_after_seconds` fields from the received
response. Missing or unparseable signals are null. HTTP-date values are interpreted
when the complete response arrives, before database processing. No raw headers,
cookies or free-text advisory values are retained in this projection. These are
server signals, not measurements of sleep durations or additional retry attempts.

Paths are rechecked after the source exchange, before raw publication or database
application. A file appearing or a parent becoming a symlink during that wait
causes rejection. Use an operator-owned directory tree; the path checks are not
a general defense against every concurrent hostile directory replacement.

This increment covers only the incremental API list. Sync push and feed-based
receipt requests continue to fail before source or database I/O. Actual adaptive
pacing waits are not yet recorded. Full network-command coverage, attachment/WARC linkage and
downstream shadow parity remain open under #310.
