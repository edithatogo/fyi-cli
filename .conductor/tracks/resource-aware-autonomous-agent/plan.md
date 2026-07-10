# Plan: resource-aware-autonomous-agent

## Phase 0: Inventory & Policy Design

### 0.1 Current behaviour audit
- [x] Task: Inventory Rust HTTP call sites (sync, API, discovery-related) for 429, headers, User-Agent, concurrency — 413d8b2
- [x] Task: Inventory Python discovery/capture/client paths for shared rate limit, Retry-After, User-Agent — 413d8b2
- [x] Task: Document gap matrix (headers, adaptive pacing, memory, plan reflection, agent loop) in this plan's Track History when done — 413d8b2
- [x] Task: Propose module layout under `crates/fyi-core` (and Python parity touchpoints) without hard LangGraph dependency — 413d8b2

### 0.2 Policy constants
- [x] Task: Define baseline concurrency, min interval, degraded thresholds (`RateLimit-Remaining` bands), recovery window, backoff ceiling — 413d8b2
- [x] Task: Define mandatory User-Agent format and validation rules — 413d8b2
- [x] Task: Update `.conductor/tech-stack.md` if new crates/modules or dependencies are required **before** implementation — 413d8b2

## Phase 1: Identity Hygiene (Zero Trust UA)

### 1.1 User-Agent policy
- [x] Task: Implement `ClientIdentity` / `UserAgentPolicy` with reject-blank/generic, product+version+SHA-256 fingerprint, repo URL, opt-in admin contact — 413d8b2
- [x] Task: Wire policy into Rust client construction / request builders — 413d8b2
- [x] Task: Wire policy into Python live paths still used for discovery/capture — 50bb43e
- [x] Task: Unit tests for accepted and rejected UA strings (incl. opt-in contact present/absent) — 413d8b2
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Identity Hygiene' (Protocol in workflow.md)

### 1.2 Continuous behavioral guardrails
- [x] Task: Implement run guardrails: max requests, max response bytes, max wall-clock duration, max concurrency — 413d8b2
- [x] Task: Embed checks in the network execution loop; trip → halt + structured reason — 413d8b2
- [x] Task: Unit tests for each trip condition — 413d8b2
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Guardrails' (Protocol in workflow.md)

## Phase 2: Standardized Header Interception & 429

### 2.1 Header parsing
- [x] Task: Parse `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`, `Retry-After` (delta-seconds + HTTP-date) — 413d8b2 / 50bb43e
- [x] Task: Normalize into a `RateLimitSnapshot` / `ThrottleSignal` type used by the pacing engine — 413d8b2
- [x] Task: Unit tests with wiremock/header fixtures (present, partial, absent, malformed) — 413d8b2 / 50bb43e

### 2.2 Graceful 429 + exponential backoff
- [x] Task: Unified 429 path: halt instance workers, honour Retry-After, else exponential backoff + jitter with ceiling — 413d8b2
- [x] Task: Ensure error/status strings never leak secrets (extend existing Rust sync coverage pattern) — 413d8b2
- [x] Task: Python parity for archive/discovery retry paths that currently special-case 429 — 50bb43e
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Headers & 429' (Protocol in workflow.md)

## Phase 3: Dynamic Bandwidth Scaling

### 3.1 Pacing engine
- [x] Task: Implement adaptive scaler states: `Baseline`, `Degraded`, `BackingOff`, `Recovering` — 413d8b2
- [x] Task: Map scaler state → concurrency, inter-request delay, batch size — 9333935
- [x] Task: Integrate scaler into primary Rust outbound paths; document Python integration points — 413d8b2
- [x] Task: Property/unit tests for threshold transitions and recovery hysteresis — 413d8b2
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Adaptive Pacing' (Protocol in workflow.md)

## Phase 4: Deliberate Load Memory

### 4.1 Schema & API
- [x] Task: Design SQLite tables/events for endpoint latency EWMA and rate-limit occurrences (per `instance_id` + route class) — 413d8b2
- [x] Task: Implement write path on each response (success and throttle) — 413d8b2
- [x] Task: Implement prune/retention so memory stays lightweight — 9333935
- [x] Task: Extend or complement `fyi rate-limit-status` (CLI) with load-memory summary; MCP status if natural fit — 36c1e54

### 4.2 Rescheduling heavy work
- [x] Task: Classify jobs as light vs heavy (e.g. single request fetch vs historical seed / bulk backfill) — 413d8b2
- [x] Task: Defer or reschedule heavy jobs using historical high-load windows unless urgency override is set — 413d8b2
- [x] Task: Tests with injected memory snapshots — 413d8b2 / 8267e31
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Load Memory' (Protocol in workflow.md)

## Phase 5: Agentic Reflection & Plan-and-Solve

### 5.1 Plan model
- [x] Task: Define a retrieval `Plan` structure (targets, windows, pagination bounds, estimated request count, instance) — 413d8b2
- [x] Task: Implement reflector rules: reject unbounded recursion, prefer date windows/checkpoints, prefer local cache when fresh — 413d8b2
- [x] Task: CLI/agent dry-run output (`--dry-plan` or equivalent) showing accept/rewrite/reject rationale — 50bb43e

### 5.2 Self-correction loop hook
- [x] Task: After throttle events, allow plan rewrite (smaller windows, lower concurrency) before resume — 8267e31
- [x] Task: Unit tests for reject/rewrite cases — 8267e31
- [ ] Task: Conductor - User Manual Verification 'Phase 5: Plan Reflection' (Protocol in workflow.md)

## Phase 6: Framework Integration, Cache, Trace Facade

### 6.1 Core loop boundaries
- [x] Task: Expose perception / reason / act / reflect traits or modules with docs mapping to LangGraph/OpenClaw concepts — 40008b3
- [x] Task: Provide one thin example adapter (in-repo, no mandatory external runtime dependency) — 40008b3
- [x] Task: Document how MCP tools map to the Action boundary — 40008b3

### 6.2 Filesystem response cache
- [x] Task: URL/ETag or content-hash keyed local cache; skip redundant remote GETs when safe — 9333935
- [x] Task: Tests for cache hit/miss and no-stale-write on non-GET/error — 40008b3

### 6.3 Trace-capture infrastructure
- [x] Task: Define FOSS-friendly span/event schema (Langfuse/Braintrust-compatible fields where practical) — 413d8b2
- [x] Task: Default JSONL file sink; trait for optional export adapters (no proprietary hard deps) — 413d8b2
- [x] Task: Emit plan/pacing/http/guardrail/cache events; redact secrets — 413d8b2
- [ ] Task: Conductor - User Manual Verification 'Phase 6: Facade/Cache/Trace' (Protocol in workflow.md)

## Phase 7: Documentation & Registry

### 7.1 Docs
- [x] Task: Update `docs/upstream-relations.md` etiquette section to match implemented RateLimit-* + adaptive pacing behaviour — 414e109
- [x] Task: Update `docs/ALAVETELI_CLIENT.md` and README good-citizen / rate-limit sections — 40008b3
- [x] Task: Add short architecture note under `docs/` for agent network middleware — 413d8b2, 40008b3
- [x] Task: Cross-link this track from CONTRIBUTING / tracks registry as completed when done — 40008b3

### 7.2 Close-out
- [ ] Task: Full test suite green; coverage gate for new modules
- [ ] Task: Mark track complete in `metadata.json` and `.conductor/tracks.md`
- [ ] Task: Conductor - User Manual Verification 'Phase 7: Docs & Close-out' (Protocol in workflow.md)

## Completion Criteria

- [ ] All acceptance criteria in `spec.md` satisfied
- [ ] Rust primary path is header-aware, adaptive, memory-backed, UA-safe
- [ ] Python live paths used for discovery/capture have documented parity or intentional gaps filed
- [ ] No CI live network; secrets never in rate-limit errors
- [ ] Docs aligned with behaviour

## Track History

- **2026-07-10**: Back-pressure parity checkpoint added standards-aligned
  `X-Advisory-Status` handling, shared Rust/Python fixtures, and consistent
  rejection of negative unsigned rate-limit values. Scoped Rust core suite:
  117 passed; Python suite: 604 passed, 1 opt-in live smoke skipped. Checkpoint:
  `37677ca` / PR #150.

- **2026-07-10**: Added framework-neutral perception/reason/action/reflection
  traits, a thin adapter, cache no-stale-write coverage, and aligned Rust,
  Python, README, and operator documentation. Checkpoint: `40008b3`.

- **2026-07-10**: Phase 0 inventory and policy baseline completed in `docs/agent-network-middleware.md`; Rust and Python live paths now share header-aware retry and identity policy. Checkpoint: `413d8b2`.
- **2026-07-10**: Added offline `dry-plan` CLI reflection and completed the plan-model/reflection subsection. Checkpoint: `50bb43e`.
- **2026-07-10**: Added deterministic post-throttle plan rewrite/rejection hooks and tests; GNU-target Rust suite reached 111 passing tests. Checkpoint: `8267e31`.
- **2026-07-10**: Added explicit adaptive batch sizing and bounded load-memory pruning/status reporting; GNU-target Rust suite reached 112 passing tests. Checkpoint: `9333935`.
- **2026-07-10**: Extended `rate-limit-status --agent-memory` with secret-free persisted memory and identity reporting; Python suite reached 106 passing tests. Checkpoint: `36c1e54`.
- **2026-07-09**: Track created via `/conductor-newtrack` from architectural brief (agentic reflection, load memory, framework integration, RateLimit-* headers, dynamic bandwidth scaling, 429/Retry-After, User-Agent zero-trust hygiene). Active registry entry added.
- **2026-07-09**: Spec/plan extended with cryptographic-aligned UA + opt-in admin contact, continuous behavioral guardrails, Langfuse/Braintrust-compatible trace hooks, FOSS/local-cache constraints, and network middleware deliverable. Implementation of `fyi-core::agent_runtime` begun.
