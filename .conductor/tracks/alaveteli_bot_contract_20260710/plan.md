# Implementation Plan: Alaveteli Bot Contract

## Phase 1: Rust identity and guardrails

- [x] Issue #141: Enforce traceable identity across Rust HTTP constructors.
- [x] Add validated identity wiring to the Tor-routed reqwest client and preserve the safe default constructor — `45f7e15`.
- [x] Add deterministic identity-header and Tor-client construction tests — `be9604d`; verified with the user-scoped MSVC toolchain.
- [x] Issue #145: Enforce resource guardrails at every Rust send boundary; stacked
  fork-local PR #147 depends on identity PR #146.
- [x] Route all `SyncClient` request, health-check, and retry send paths through
  one guarded executor — `c30b9f4`.
- [x] Bound response buffering before accounting bytes and enforce request-count,
  response-byte, runtime, and concurrency limits at that boundary — `c30b9f4`.
- [x] Add red/green wiremock tests proving request-count and response-byte trips
  block the next remote call without exposing response content — `c30b9f4`.
- [x] Integrate and test the same executor contract for Tor/proxy-assisted Rust
  sends and complete the all-constructor audit — `5735d9d`.
- [x] Working tree was clean before the guardrail slice; no unrelated changes
  were included.

## Phase 2: Back-pressure parity

- [x] Issue #142: Parse and honor Alaveteli rate and advisory headers; stacked
  fork-local PR #150 depends on PR #147.
- [x] Align Rust and Python snapshots for standard rate-limit fields and
  `X-Advisory-Status`, including degraded pacing — `37677ca`.
- [x] Add shared offline fixtures and parity tests for valid and malformed
  headers; reject negative unsigned values consistently — `37677ca`.
- [x] Complete cross-path 429 halt, HTTP-date, bounded-jitter, and instance
  isolation evidence against the full client contract — `08ef89a`.

## Phase 3: Cache and bounded bulk mode

- [x] Issue #143: Add conditional caching and bounded bulk export mode; stacked
  fork-local PR #152 depends on PR #150.
- [x] Add bounded streamed Python bulk export with item/byte ceilings and
  guaranteed response closure — `68f2f53`.
- [x] Ensure streamed bulk bodies are not written into the validator cache —
  `68f2f53`.
- [x] Complete Rust validator metadata, 304/stale/error behavior, and bounded
  request/time/batch evidence — `866add9`.
- [x] Ensure unavailable or unauthorized bulk export cannot fall back to unbounded retrieval — `866add9`.

## Phase 4: Cross-repo verification

- [~] Issue #144: Run fork-local Alaveteli contract verification and reconcile both tracks — fyi-cli harness complete (`0d40a65`); paired server verification remains external.
- [x] Consume shared offline fixtures and run opt-in bounded live smoke only when explicitly enabled — `0d40a65`, `3ec2c12`; live mode remains disabled by default.
- [ ] Close only when every known risk is fixed, verified false positive, or blocked by a dated disabled follow-up.

## PR standard

One child issue maps to one PR. Each PR must state scope, non-scope, test-first
evidence, security/quality sensors, rollback, and harness changes. Parent and
paired issue links are mandatory. No PR may include unrelated refactoring or
the pre-existing dirty `agent_runtime.rs` change unless explicitly assigned.

## Verification blockers

- [x] Provision a non-admin MSVC/Windows SDK toolchain under
  `%USERPROFILE%\msvc_portable` and verify it through
  `scripts/verify_msvc_portable.ps1`.
- [x] Re-run the focused identity test through
  `scripts/Invoke-MsvcPortable.ps1` and remove this blocker only after it
  passes: 5 passed, 0 failed on 2026-07-10.

## Verification record

- `cl.exe`: Microsoft C/C++ Optimizing Compiler `19.44.35228` for x64.
- Command: `pwsh -NoProfile -File scripts/verify_msvc_portable.ps1`.
- Command: `pwsh -NoProfile -File scripts/Invoke-MsvcPortable.ps1 cargo test --target x86_64-pc-windows-msvc -p fyi-core --test tor_tests`.
- Result: 5 passed, 0 failed; no elevation, registry, system environment, or
  `Program Files` writes were used.

## Guardrail slice verification

- Fork-local draft PR: https://github.com/edithatogo/fyi-cli/pull/147
- Scoped command: `pwsh -NoProfile -File scripts/Invoke-MsvcPortable.ps1 cargo test --locked -p fyi-core sync::tests --lib`.
- Result: 28 passed, 0 failed.
- Full scoped core library command: `pwsh -NoProfile -File scripts/Invoke-MsvcPortable.ps1 cargo test --locked -p fyi-core --lib`.
- Full result: 115 passed, 0 failed.
- Scoped check: `cargo check --locked -p fyi-core` completed successfully.
- Tor/proxy-assisted send integration and deterministic boundary tests are
  covered by `5735d9d`; the former closure blocker is resolved.

## Back-pressure slice verification

- Fork-local draft PR: https://github.com/edithatogo/fyi-cli/pull/150
- Paired Alaveteli fixture issue: https://github.com/edithatogo/alaveteli/issues/25
- Scoped Rust library result: 117 passed, 0 failed.
- Python full result: 604 passed, 1 opt-in live smoke skipped.
- Repository Ruff still reports pre-existing baseline findings; this slice did
  not broaden that unrelated lint debt.

## Cache/bulk slice verification

- Fork-local draft PR: https://github.com/edithatogo/fyi-cli/pull/152
- Paired Alaveteli fixture/control issue: https://github.com/edithatogo/alaveteli/issues/25
- Python Alaveteli client tests: 34 passed, including item/byte trip and stream
  closure tests.
- Resolved by the validator/bulk slice below; remaining closure work is the
  fork-local Alaveteli reconciliation in Phase 4.

## Validator/bulk slice verification

- Fork-local implementation checkpoint: `866add9`.
- Focused GNU Rust tests: conditional 304 reuse, stale/error preservation,
  authenticated bounded NDJSON export, and unauthorized no-fallback — 4 passed.
- The bulk route is explicitly opt-in and has no ordinary recursive retrieval
  fallback; item, byte, and runtime limits are enforced before returning data.

## Tor executor verification

- Checkpoint: `5735d9d`.
- `TorAgentClient` constructs the proxy client with the supplied traceable
  identity and routes requests through `AgentNetworkMiddleware` for pacing,
  concurrency, response-byte guardrails, load memory, traces, and 429 feedback.
- GNU Tor integration test: `test_tor_guarded_client_uses_shared_agent_identity_contract` passed.

## Back-pressure verification

- Checkpoint: `08ef89a`.
- Full GNU `fyi-core` suite: 128 passed, including HTTP-date Retry-After,
  deterministic bounded jitter, instance/route memory isolation, and 429
  BackingOff state.
- The remaining Phase 4 work is fork-local Alaveteli verification and shared
  server-side reconciliation, not an untested client-side behavior.

## Fork-local harness verification

- Offline harness: `scripts/verify_alaveteli_bot_contract.py`.
- Runtime result: `mode=offline`, two shared back-pressure cases, bounded route
  bulk ceiling reported, and live smoke explicitly disabled.
- Quality gates: Ruff and basedpyright pass for the changed Python harness and
  route module; targeted route coverage is 88%.
- External follow-up: run the paired Alaveteli server fixture/conformance suite
  under issue #144 before evidence-gate closure.

### Paired-server verification blocker (2026-07-10)

- Server implementation evidence exists in sibling checkout
  `edithatogo/alaveteli` at `1c81bdeaa` (validator controller/spec) and
  `6bfc7e4ac` (Conductor evidence checkpoint).
- Focused command attempted: `bundle exec rspec
  spec/controllers/api/v1/sustainability_controller_spec.rb
  spec/controllers/concerns/traffic_control_spec.rb
  spec/requests/contracts_validation_spec.rb`.
- Local result: cannot execute on this workstation because Ruby/Bundler/RSpec
  are not installed. The follow-up was subsequently exercised in paired PR #29
  CI; see the current CI evidence below.

### Paired-server CI evidence (2026-07-10)

- The paired implementation was exercised by Alaveteli PR #29 CI run
  `29072076457`; the Ruby 3.4 core job completed with 9,465 examples, 41
  failures.
- Contract-relevant failures are not yet a clean pass: six
  `BulkExportStreamer` examples fail (filtering, deterministic ordering,
  bounded row count, public-body join, and calculated status), and the
  `TrafficControl` rate-limit reset assertion returns `42` where the fixture
  expects `18`.
- Additional failures are unrelated baseline/environment suites (admin notes,
  multipart/TNEF, route/cache and other existing Rails behavior). The paired
  security workflow also reports dependency-review unavailable and existing
  vulnerable gems; these are not silently attributed to fyi-cli.
- Closure remains open pending a focused paired-server fix/re-run of the
  contract failures, with the exact CI run retained as the deterministic
  sensor.

## Final local verification record

- Python repository suite after contract additions: 616 passed, 1 opt-in live
  smoke skipped.
- GNU Rust `fyi-core`: 128 passed; GNU Rust `fyi-mcp`: 19 passed.
- Local harness, Ruff, basedpyright, validator/bulk, Tor executor, and
  back-pressure checks all pass. Paired Alaveteli server verification remains
  the only external Phase 4 dependency.

## Repository-wide harness hardening (2026-07-10)

- Added `scripts/verify_test_harness.py` and `tests/test_harness_contract.py`.
- The checked-in inventory validates 13 layers: unit, integration, end-to-end,
  smoke/system, mutation, property-based, edge, performance, security,
  compatibility, usability, regression, and sanity.
- CI now hard-fails Rust line coverage below 90%; Codecov project and patch
  statuses use a zero-tolerance 90% target. The prior 80% hard floor and
  informational 90% notice are removed.
- Weekly/manual Rust mutation analysis is defined in
  `.github/workflows/mutation.yml`; long-running local mutation and benchmark
  runs are explicit `--run-expensive` harness modes.
- Harness contract test, Ruff, and basedpyright pass. GNU `fyi-core` remains
  128/128 passing after the gate changes.
