# Track Specification: Alaveteli Bot Contract

## Outcome

Implement and verify the fork-local fyi-cli behavior required to consume
Alaveteli back-pressure, identity, cache, and bounded bulk-export contracts
without creating runaway load. This track coordinates with
`edithatogo/alaveteli`; it does not create upstream issues or pull requests.

## Existing state

The active `resource-aware-autonomous-agent` track already contains policy and
plan-reflection work, but its acceptance criteria show that identity enforcement,
live Rust/Python header wiring, adaptive pacing, durable load memory, cache, and
trace integration remain incomplete. The Alaveteli Sustainability Suite claims
client work under an unverified commit, so this track is the authoritative
fork-local interoperability plan.

## Contract surface

- `RateLimit-Limit`, `RateLimit-Remaining`, and `RateLimit-Reset`
- `Retry-After` and `X-Advisory-Status: degraded`
- `ETag`, `Last-Modified`, `If-None-Match`, and `304 Not Modified`
- `/api/v1/rate_limit`
- bounded `/api/v1/bulk_export` NDJSON responses
- traceable User-Agent and explicit `X-FYI-Bot-Token` configuration

## Risk policy

No known security, privacy, availability, correctness, data-integrity, or
quality risk may be accepted. Any unresolved item blocks closure or becomes a
disabled, dated follow-up with a deterministic sensor.

## Harness requirements

Every slice must improve feedforward guidance and feedback sensors: shared
offline fixtures, red/green tests, Rust/Python parity checks, lint/type/security
scans, bounded smoke commands, secret-redaction assertions, and rollback or
disablement instructions. The pre-existing dirty change in
`crates/fyi-core/src/agent_runtime.rs` is outside this track's first planning
slice and must be preserved.

## Delivery boundaries

Each child issue maps to one focused PR. Rust identity, response feedback,
Python parity, cache/bulk behavior, and final verification remain separate so
each change can be reviewed and reverted independently.

