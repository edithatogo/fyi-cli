# Guarded Rust Alaveteli acquisition with v1 receipts

Status: read-only acquisition evidence updated 17 November 2026. Canonical Conductor owner: `.conductor/tracks/source-adapter-boundary-20260812/` (#309, #310). **Not a fyi-archive cutover contract.**

## Behaviour

The native Rust `fyi-cli fetch-request-page <id>` uses
`fyi-core::sync::RequestEvidenceClient` and the existing agent-aware guarded
execution boundary. It performs one GET of `/api/v2/request/<id>.json`, validates
a positive requested ID, the parsed request ID, the HTTP response status and a
16 MiB response limit. It produces both original network entity bytes and a
normalized canonical JSON projection. The dedicated transport requests identity
encoding and disables automatic redirects, content decoding, conditional-cache
reuse and all reqwest retries, including protocol-level retries.

The receipt v1 stores **two distinct digests**:
- `responses[0].sha256`: SHA-256 of the exact returned HTTP entity bytes after HTTP transfer decoding, before JSON parsing
- `result.sha256`: SHA-256 of the canonical typed `AlaveteliRequest` JSON projection with a trailing LF

These bytes are **not a WARC HTTP exchange**; complete HTTP headers, redirect
chains, attestation and signing are not captured. An unexpected encoded body is
retained unchanged but rejected for parsing. A 3xx response is captured as a
failed acquisition without following its location. HTTP 304 is likewise a failed
acquisition, without substituting cached bytes.

The response URL stored in receipts has query values redacted and no credentials
or fragment. Authentication headers and response bodies are never included in
error messages. Existing network guardrails constrain request concurrency, rate
limits, bytes and time. This command does not follow any redirect; ordinary
non-evidence `SyncClient` APIs retain their separate same-origin redirect policy.
These controls do not by themselves establish DNS-rebinding protection.

## CLI

```bash
# Read-only guarded fetch (no files written)
fyi-cli fetch-request-page 42 --base-url https://fyi.org.nz

# Read-only fetch with three distinct, explicitly requested private files
fyi-cli fetch-request-page 42 --base-url https://fyi.org.nz \
  --raw-file evidence/42-response.bin \
  --result-file evidence/42-result.json \
  --receipt evidence/42-receipt.json

# Recheck without contacting a source
fyi-cli verify-acquisition-receipt evidence/42-receipt.json \
  --raw-file evidence/42-response.bin \
  --result-file evidence/42-result.json
```

Evidence paths must be supplied together or not at all. Each acquisition requires
three fresh files. Preflight rejects lexical aliases, nested output paths,
symlink ancestors, existing destinations and overlaps with the shared pacing
database or its SQLite sidecars before opening that database or requesting the
source. Paths are checked again after the source exchange. Use an operator-owned
directory tree that other processes cannot rename or modify; these checks do not
provide a directory-handle guarantee against every hostile filesystem race.

Writes create private same-directory temporary files, flush them and atomically
publish without replacing existing evidence. The raw and normalized files are
published **before** the receipt. The low-level writers retain byte-identical
idempotency, but the acquisition command itself requires fresh destinations.
This is not a three-file transaction: interruptions and publication failures may
leave orphan blobs for operator reconciliation.

A complete HTTP error, malformed JSON, mismatched request ID or unsupported
encoding produces a valid failed receipt when all three files can be published.
The command still exits unsuccessfully. Failed result files contain a fixed
failure projection, never the source's error text. Connection failures, truncated
bodies and byte-limit failures cannot establish a complete v1 response and do
not produce a receipt. Publication failure also returns an error and must not be
treated as a successful acquisition.

The compatibility method `SyncClient::fetch_request_evidence` returns successful
evidence without writing files. Built-in `SyncClient` constructors provide a
paired dedicated evidence transport. Clients supplied through `with_http_client`
are opaque and cannot use that compatibility method: silently replacing their
proxy, TLS or other transport policy would be unsafe. Their ordinary APIs remain
available. Use `RequestEvidenceClient::new` only when its documented transport
policy is appropriate.

The `--db` option remains syntactically accepted in Rust. Request records are
written only by the retained Python command. Python's `fetch-request-page` also
uses a different request path and output shape, so semantic parity remains open.

## Shared source pacing

Rust callers can opt into the existing SQLite limiter used by the retained Python
source adapters. Pass the same database path and limiter name to coordinate
future reservations across clients. The default name is `archive-discovery`,
the default minimum interval is two seconds, and jitter is at most 0.25 seconds.
The existing Rust in-process guards also apply.

```bash
fyi-cli fetch-request-page 42 --base-url https://fyi.org.nz \
  --shared-rate-limit-db source-pacing.db \
  --rate-limit-name archive-discovery \
  --minimum-interval-seconds 2

# Read existing local state without contacting a source
fyi-cli rate-limit-status --db source-pacing.db --name archive-discovery
```

Each reservation updates `shared_rate_limit_state` and its event in one SQLite
immediate transaction. A storage failure blocks the HTTP request. HTTP 429 and
503 responses with a valid `Retry-After` advance the shared reservation. Existing
reserved slots retain their scheduled times, matching the retained Python
algorithm. The command performs one HTTP attempt. It adds no automatic retries.
Receipts include the configured limiter name and minimum interval.

`rate-limit-status` opens an existing database read-only and returns the stored
state plus the ten latest events. An unknown name returns JSON `null`. A missing
database or incomplete limiter schema returns an error. This differs from the
Python reader, which initializes missing state tables. Rust accepts `--db`,
`--name` and `--output`. Its `--agent-memory` option remains unimplemented.

`--output` uses the existing private, atomic, no-clobber evidence writer.
Byte-identical repeats succeed. Save changed snapshots to a new path. Conflicting
files and symlinks are preserved and produce an error. Tests compare reservation
values and the complete status JSON with
`conformance/pacing/shared-rate-limit-v1-python.json`, generated from the retained
Python implementation on Python 3.15.1 with fixed clock and jitter inputs.

## Offline conformance and limitations

The Rust 1.99.1 CI checks raw and normalized content against Python-origin v1 fixtures and rejects deliberate mutations without any network traffic. Mocked `SyncClient` tests cover the outbound GET, response digest, normalized result, denied responses, malformed IDs and 304 cache refusal.

The following remain open under #309/#310–#314: complete Alaveteli/Python command semantic parity; integration of receipts into all Rust network commands and external indexes; repeatable fyi-archive old/new NZ shadow comparison; WARC/WACZ original HTTP and attachment byte linkage; cross-platform filesystem fault injection; legal/ethical upstream coordination; signed release/provenance manifests and archive cutover authorisation.

Do not treat this read-only single-request workflow as approval for unattended scraping, bulk retrieval, writes, or access to restricted data.
