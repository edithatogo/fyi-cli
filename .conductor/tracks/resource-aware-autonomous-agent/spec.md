# Specification: resource-aware-autonomous-agent

## Overview

Refactor **edithatogo/fyi-cli** from a primarily passive scraper/client into a
**production-grade, resource-aware Autonomous Agent**. The agent must natively
respect server-side constraints and dynamically adjust its bandwidth footprint so
volunteer-run Alaveteli platforms (especially **FYI.org.nz**) remain sustainable
under multi-jurisdiction archival, discovery, and sync workloads.

This track operationalizes etiquette already documented in
`docs/upstream-relations.md` as enforceable client behaviour, and evolves the
core network/execution loop so it is compatible with modern agentic architectures
(perception → reasoning → tool execution) without treating live FOI instances as
unlimited scrape targets.

## Problem Statement

Current state (partial capabilities already exist):

| Area | Today | Gap |
|------|--------|-----|
| Shared client pacing | Python `shared_rate_limit_*` + `fyi rate-limit-status` | Fixed/local interval; not driven by live RFC rate-limit headers |
| HTTP 429 | Rust sync surfaces Retry-After context; Python retries 429 | No unified exponential backoff policy across Rust/Python paths |
| RateLimit-* headers | Not systematically parsed | No structural capacity awareness (`Limit` / `Remaining` / `Reset`) |
| Latency memory | Transient only | No durable endpoint latency / load-period memory for rescheduling |
| User-Agent hygiene | Documented norm; partial paths | Blank/generic agents must be impossible on live paths |
| Plan quality | Operator/script driven | No pre-execution plan reflection to avoid broad recursive queries |
| Agent frameworks | CLI + MCP tools | Core loop not explicitly shaped as perception/reason/act boundaries |

Without these controls, bulk discovery, capture, and multi-instance sync risk
accidental Layer-7 load against small volunteer deployments.

## Goals

1. **Bidirectional rate signalling** — Parse and honour Alaveteli/RFC response headers.
2. **Adaptive pacing** — Throttle concurrency and request velocity under pressure; scale back to baseline when clear.
3. **Durable load memory** — Local cache of endpoint latency and rate-limit events; prefer low-load windows for heavy jobs.
4. **Agentic plan-and-solve** — Evaluate retrieval plans before execution; reject/reshape expensive plans.
5. **Framework-ready core loop** — Clean boundaries for perception, reasoning, and tool execution (OpenClaw / LangGraph-style composition).
6. **Zero-trust identity hygiene** — Mandatory contactable User-Agent; no blank/generic default on live traffic.

## Architectural Patterns

### 1. Agentic Reflection & Plan-and-Solve

Implement a **self-correction layer** that evaluates a data-retrieval plan *before*
network execution:

- Prefer date-windowed, paginated, checkpointed work over unbounded recursion.
- Prefer cached / local SQLite state when freshness SLAs allow.
- Reject or rewrite plans that would fan out into broad recursive authority/request walks without bounds.
- Emit a human-readable plan summary (and optional machine JSON) for operator review when `--dry-plan` / agent mode is used.

### 2. Deliberate Memory Engineering

Introduce a **lightweight local state cache** (SQLite-backed, co-located with
existing rate-limit tables where practical) that logs:

- Per-endpoint (or per-instance+route class) latency histograms / EWMA
- Rate-limit hits (`429`, low `RateLimit-Remaining`, explicit resets)
- High-load time-of-day / day-of-week signals (local, privacy-preserving aggregates)

The agent **must** use this memory to reschedule heavy tasks (historical seed,
bulk backfill, full-corpus discovery) away from historically high-load periods
when urgency is not user-forced.

### 3. Framework Integration

Design the core loop so perception, reasoning, and tool execution are separable:

```
Perception  → headers, latency, local memory, instance capabilities
Reasoning   → plan evaluation, pacing decision, schedule
Action      → bounded HTTP tools (read/search/feed/capture), checkpoint writes
Reflection  → update memory, adjust plan / concurrency
```

Compatibility target: tools and state adapters should be mappable into
LangGraph-style graphs or OpenClaw-style agent runtimes without rewriting the
HTTP/good-citizen core. Prefer traits/modules over a hard dependency on any one
agent framework.

## Bidirectional Communication Mechanisms

### Standardized Header Interception

The network client (Rust primary; Python legacy parity where still used for
archive/discovery) must parse and respect:

| Header | Meaning for the agent |
|--------|------------------------|
| `RateLimit-Limit` | Total quota for the current window |
| `RateLimit-Remaining` | Immediate remaining capacity |
| `RateLimit-Reset` | Window refresh (seconds or HTTP-date per RFC semantics) |
| `Retry-After` | Mandatory wait on 429 / selected 5xx |

When headers are absent, fall back to conservative defaults (existing shared
interval limiter + etiquette floors from upstream-relations).

### Dynamic Bandwidth Scaling

Adaptive pacing engine:

- **Scale down** when `RateLimit-Remaining` drops below configurable thresholds,
  when EWMA latency exceeds high-water marks, or on consecutive slow responses.
- **Scale up** toward a **baseline** (not an aggressive max) when metrics clear
  for a sustained recovery window.
- Control knobs: concurrency (workers), inter-request delay, batch size, and
  whether heavy jobs are deferred.

### Graceful Throttling & Retry-After

On HTTP **429 Too Many Requests**:

1. Halt further concurrent requests for the affected instance (or global worker pool as configured).
2. Parse `Retry-After` precisely (delta-seconds or HTTP-date).
3. Apply **exponential backoff with jitter** when Retry-After is missing or on
   repeated failures, with a hard ceiling and max attempts.
4. Record the event in load memory; surface non-secret context to CLI/MCP
   (pattern already started in Rust sync 429 tests).

## Agent Zero-Trust Security

### Identity hygiene (cryptographic-aligned User-Agent)

Enforce strict identity hygiene on all live outbound HTTP:

- **Reject** blank or generic User-Agent values (`""`, `python-requests/*` alone,
  `curl/*` alone, library defaults without product identity).
- **Require** a distinct, **traceable, cryptographic-aligned** identity string that
  identifies:
  1. the script/product name (`fyi-cli`),
  2. its **version**,
  3. a short **content fingerprint** (SHA-256 prefix over product+version+repo for
     stable, non-secret build alignment — not a secret credential),
  4. a product homepage/repo URL, and
  5. an **opt-in** administrative contact (email or URL) when the operator chooses
     to disclose one.
- Example form:
  `fyi-cli/0.1.2 (fp:a1b2c3d4; +https://github.com/edithatogo/fyi-cli; contact:ops@example.org)`
- Operator override is allowed only if the override still includes product token,
  version (or equivalent), and a contact surface (URL and/or opt-in admin contact).
- Tor/proxy paths **must still** send the same identity; anonymity must not be
  used to hide abusive load (align with `docs/upstream-relations.md`).

### Continuous behavioral guardrails

Embed operational boundary checks **directly in the execution loop** to prevent
rogue loops or runaway automation:

| Guardrail | Purpose |
|-----------|---------|
| Max requests per run | Cap total HTTP calls in one agent session |
| Max response bytes / data volume | Cap download footprint |
| Max wall-clock runtime | Hard stop after duration |
| Max concurrent workers | Bound fan-out |
| Max retries / backoff ceiling | Bound 429/5xx retry storms |

When any guardrail trips, the loop **halts cleanly**, emits a structured reason
(trace + CLI/MCP status), and refuses further remote calls until a new run is
started with explicit operator intent.

## Observability and Governance

### Trace-capture infrastructure

Integrate **lightweight telemetry hooks** (schema-compatible with platforms such
as **Langfuse** or **Braintrust**) that output execution traces so developers or
host administrators can audit agent decisions during bulk retrieval:

- Span/event types: `plan.reflect`, `pacing.decision`, `http.request`,
  `http.response_headers`, `guardrail.trip`, `cache.hit` / `cache.miss`,
  `backoff.wait`.
- Default sink: local JSONL file (FOSS, no network) under the operator data dir.
- Optional export adapter trait for Langfuse/Braintrust-compatible payloads
  **without** hard-wiring proprietary SDKs; any optional integration must use
  permissive-licensed clients only.
- Traces must **never** include API keys, cookies, or response body secrets.

## Implementation Constraints

- Adhere strictly to the **existing** `edithatogo/fyi-cli` languages and layout
  (Rust `fyi-core` primary network path; Python legacy discovery/capture parity).
- Rely entirely on **FOSS-friendly, permissive-licensed** dependencies (MIT/Apache
  preferred; no new GPL-only hard deps).
- Maintain robust **local filesystem caching** to eliminate redundant remote
  network calls for unchanged resources (content-addressed or URL+ETag keyed).

## Expected Deliverable

An updated **network execution engine / middleware adapter** woven into the CLI
request pipeline that combines:

- bidirectional RateLimit-* / Retry-After loops,
- adaptive pacing,
- plan reflection,
- identity + guardrails,
- load memory,
- filesystem response cache,
- and trace hooks.

## Functional Requirements

1. **Header-aware HTTP middleware** in Rust `fyi-core` (and Python parity for
   discovery/capture paths still on `requests`).
2. **Pacing engine** with baseline / degraded / recovery states, unit-tested
   with synthetic header sequences.
3. **Load memory store** (SQLite and/or durable local files + API; CLI inspect
   extending or complementing `fyi rate-limit-status`).
4. **Plan reflector** for bulk/discovery/sync plans: bounds checks, cost
   estimate, rewrite suggestions.
5. **Agent loop facade** (module or trait set) documenting perception/reason/act
   boundaries; optional thin adapter example (not a full LangGraph dependency).
6. **Mandatory User-Agent policy** (cryptographic-aligned, opt-in contact)
   enforced at client construction and request send time.
7. **Behavioral guardrails** in the execution loop (max requests, bytes, runtime).
8. **Trace-capture hooks** with local JSONL default + export-compatible schema.
9. **Filesystem response cache** to skip redundant remote GETs when safe.
10. **Tests**: wiremock/header fixtures for RateLimit-* and Retry-After; property
    tests for backoff math; guardrail trip tests; cache hit/miss; secret-free
    rate-limit errors.
11. **Docs**: update `docs/upstream-relations.md`, `docs/ALAVETELI_CLIENT.md`,
    README good-citizen section, and tech-stack note for new modules.

## Non-Functional Requirements

- **Live-safe defaults:** CI and default dev flows remain offline; live smoke
  stays opt-in (`FYI_LIVE_SMOKE` etc.).
- **No unsolicited bulk traffic** introduced by this track itself.
- **Instance isolation:** pacing and memory keyed by `instance_id` (and route
  class), consistent with multi-jurisdiction security work.
- **Performance:** pacing decisions are O(1) or cheap SQLite reads; no blocking
  cloud calls for policy.
- **Observability:** operators can inspect why the agent slowed down
  (remaining quota, latency EWMA, last 429, deferred heavy job).
- **TDD:** failing tests first per `.conductor/workflow.md`.
- **Coverage:** new modules target >80% line coverage.

## Acceptance Criteria

- [ ] Live clients parse `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`, and `Retry-After` when present.
- [ ] On 429, execution for that instance waits for Retry-After (or exponential backoff fallback) and records memory.
- [ ] Adaptive scaler reduces concurrency/velocity under low remaining quota or latency spikes, and recovers to baseline when clear.
- [ ] Local load memory persists across process restarts and influences heavy-task scheduling.
- [ ] Plan reflection blocks or rewrites unbounded recursive retrieval plans in tests.
- [ ] Blank/generic User-Agent cannot be used for live requests; cryptographic-aligned default with version + fingerprint is enforced; admin contact is opt-in.
- [ ] Guardrails stop runaway loops (max requests / bytes / runtime) with auditable trip reasons.
- [ ] Trace JSONL (or compatible sink) records plan/pacing/http/guardrail decisions without secrets.
- [ ] Filesystem cache avoids redundant remote calls for cacheable GETs in tests.
- [ ] CLI (and MCP if applicable) can report pacing/load status without leaking secrets.
- [ ] Docs updated; etiquette in upstream-relations matches implemented behaviour.
- [ ] Unit/integration tests pass; no default live network in CI.
- [ ] No non-permissive dependency introduced for this track.

## Out of Scope

- Deploying a full production LangGraph/OpenClaw runtime as a hard dependency.
- Negotiating live rate-limit policy changes with instance operators (outreach
  remains human-owned; see `upstream-alaveteli-engagement`).
- Layer-3/4 DDoS tooling or offensive load generation.
- Changing Alaveteli server software.
- Full historical re-archive of any instance as part of this track.

## Dependencies

- **upstream-alaveteli-engagement** — etiquette norms and outreach framing.
- **jurisdiction-abstraction-core** — `instance_id` / catalog for per-instance pacing keys.
- **api-contract-hardening-20260630** — live-safe API boundaries.
- **multi-jurisdiction-security-hardening** — identity, SSRF, Tor disclosure alignment.
- Builds on existing Python `shared_rate_limit_*` and Rust sync 429 handling as
  migration/extension points rather than greenfield replacements where possible.

## Success Metrics

| Metric | Target |
|--------|--------|
| 429 handling correctness | 100% of tested Retry-After / missing-header cases match policy |
| Header-driven throttle | Scaler enters degraded mode when Remaining ≤ threshold in fixtures |
| Identity hygiene | 0 live requests with blank/generic UA in unit matrix |
| Operator transparency | `rate-limit-status` (or successor) shows last backoff + memory summary |
| Sustainability | Defaults never exceed baseline RPS; heavy jobs deferrable by memory |

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Instances omit RateLimit-* headers | Conservative defaults + existing shared interval limiter |
| Over-throttling stalls operator work | Explicit `--force-schedule` / urgency flags; clear status output |
| Dual Rust/Python drift | Shared policy constants/docs; Rust primary, Python parity checklist |
| Memory SQLite growth | Retention/pruning for events; aggregate histograms not raw forever |
| Agent frameworks over-scope | Facade + one example adapter only; no monorepo agent runtime |

## Open Questions (resolve during Phase 0 if needed)

1. Single shared policy crate/module name (`pacing` vs `good_citizen` vs `agent`)?
2. Should heavy-job deferral be automatic by default, or opt-in with strong recommendations?
3. Exact baseline RPS / concurrency per instance tier (`supported` vs `community`)?
