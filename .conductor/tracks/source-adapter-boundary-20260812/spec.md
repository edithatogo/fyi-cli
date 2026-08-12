# Source adapter boundary for archive automation

Issue: [fyi-cli #309](https://github.com/edithatogo/fyi-cli/issues/309).
Consumers: [fyi-archive #370](https://github.com/edithatogo/fyi-archive/issues/370)
and [foi-process #114](https://github.com/edithatogo/foi-process/issues/114).

## Goal

Make `fyi-cli` the exclusive source-network boundary for archive automation.
Every operation emits deterministic local outputs plus an atomic, versioned
acquisition receipt. Downstream repositories select, normalize, package,
validate, process, and publish those outputs without reimplementing source HTTP.

## Adapters

- Alaveteli authority and request discovery, numeric-ID reconciliation, capture,
  attachment retrieval, WARC/WACZ, and process-event export.
- Internet Archive CDX discovery with pagination, checkpoints, deadlines,
  retries, duplicate/stall detection, and response digests.
- Bounded Wayback replay over approved CDX rows with redirect/host controls,
  byte verification, and resumable checkpoints.
- Source-health probing and capture pacing through one shared SQLite limiter,
  named rate-limit scopes, and `Retry-After` propagation.
- Narrowly allowlisted retrieval of approved external historical indexes.

## Contract

Every network command must support an atomic receipt containing adapter and
contract versions, source identity, request bounds, timestamps, response status,
byte counts, digests, retry/rate evidence, and checkpoint lineage. Commands must
fail closed without replacing the previous valid receipt or checkpoint.

## Acceptance

- Offline fixtures cover pagination, resume keys, redirects, stalls, retries,
  rate limits, deadlines, checkpoints, partial files, and tampering.
- Concurrent capture proves one shared limiter across processes.
- Replay rejects host escapes and verifies status, length, and digest.
- Old and new adapter fixtures have exact canonical output parity.
- Hosted opt-in smoke tests cover one live Alaveteli capture and one CDX/replay
  record without publication.
- `fyi-archive` pins the implementing release before its direct transports are
  removed, and NZ shadow parity is green before cutover.

## Non-goals

- Archive scheduling, leasing, package assembly, or publication.
- Process mining or dashboard publication.
- Treating Internet Archive discovery as a substitute for captured records.
