# Agent network middleware

Resource-aware network execution engine for fyi-cli. It turns passive scraping
into a **good-citizen agent loop** that respects volunteer-run Alaveteli hosts
(FYI.org.nz and peers).

## Modules

| Stack | Path |
|-------|------|
| Rust (primary) | `crates/fyi-core/src/agent_runtime.rs` |
| Python (parity) | `src/fyi_system/agent_runtime.py` |

`SyncClient::new` builds HTTP clients with the default cryptographic-aligned
User-Agent. Discovery/importers use the Python builder (optional
`FYI_ADMIN_CONTACT` for opt-in operator contact).

## Pipeline

```
Plan reflect → Guardrails → Pacing wait → Cache lookup
    → HTTP (RateLimit-* / Retry-After) → Memory update → Trace → Cache store
```

## Phase 0 implementation inventory

The live outbound paths are deliberately split between the Rust primary client
and the legacy Python archive/discovery clients:

| Path | HTTP surface | Good-citizen boundary |
|------|--------------|-----------------------|
| `crates/fyi-core/src/sync.rs` | `SyncClient` pull, feed, and push requests | `build_http_client`, SSRF validation, sync error classification |
| `crates/fyi-core/src/agent_runtime.rs` | middleware adapter around outbound requests | header parsing, pacing, guardrails, load memory, cache, traces |
| `src/fyi_system/discovery.py` | paginated discovery and ID backfill via `httpx` | robots checks, shared SQLite limiter, Retry-After-aware backoff, checkpointing |
| `src/fyi_system/importers.py` | official authority CSV via `httpx` | mandatory identity header and read-only import |
| `src/fyi_system/alaveteli_client.py` | legacy `requests` read/write API | mandatory identity header, ETag cache, rate-limit capture, bounded 429 retry |

The current policy baseline is conservative: Rust starts at two workers with a
250 ms interval; degraded mode uses one worker and increases delay; backoff is
capped at five minutes; Python discovery retains its existing one-second floor
and uses the server's `Retry-After` value when supplied. Rate-limit and latency
memory is keyed by instance and route class, while heavy retrieval plans can be
deferred unless explicitly forced.

Python and Rust intentionally share the identity shape and retry semantics, but
do not share a runtime dependency: Python remains a compatibility path while
Rust is the primary network execution path.

### Identity hygiene

User-Agent form:

```text
fyi-cli/<version> (fp:<sha256-prefix>; +https://github.com/edithatogo/fyi-cli[; contact:<opt-in>])
```

- `fp` is a non-secret SHA-256 prefix over product+version+repo.
- Admin contact is **opt-in** only (`FYI_ADMIN_CONTACT` in Python; constructor arg in Rust).
- Blank/generic agents (`curl/*`, `python-requests/*`, empty) are rejected.

### Bidirectional headers

Parsed when present:

- `RateLimit-Limit` / `RateLimit-Remaining` / `RateLimit-Reset`
- `Retry-After` (delta-seconds or HTTP-date)

Adaptive pacing states: `Baseline` → `Degraded` → `BackingOff` → `Recovering`.

### Guardrails

Per-run hard stops:

- max requests
- max response bytes
- max wall-clock runtime
- max concurrency (policy)

### Local cache & memory

- **Filesystem response cache** — URL-hashed bodies; skips redundant GETs.
- **Load memory** — EWMA latency + rate-limit hits + hour-of-day histograms;
  can defer heavy plans.

### Traces

Default sink: append-only **JSONL** with Langfuse/Braintrust-friendly fields
(`type`, `name`, `timestamp`, `run_id`, `metadata`, `id`). Secrets are redacted.
No proprietary SDK is required.

### Plan reflection

Rejects unbounded recursive retrieval without a date window; rewrites when a
window is present; can require `force_schedule` for very large estimates.

## Conductor track

`.conductor/tracks/resource-aware-autonomous-agent/`

## Related

- `docs/upstream-relations.md` — etiquette norms
- `fyi rate-limit-status` — shared Python limiter inspection
