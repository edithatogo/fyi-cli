# Implementation Plan: Endorsed Client Route

## Phase 0: Proposal and evidence boundary

- [x] Open fyi-cli planning issue #148 with explicit upstream-submission gate.
- [x] Record opt-in, sysadmin-controlled, kill-switchable design principles.
- [x] Reconcile the proposal with existing fyi-cli #140/#142-#144 and Alaveteli #23-#27 without duplicating contracts — `356e470`, `3e984f0`.
- [x] Write a threat model covering accidental overload, hostile clients, credential misuse, privacy, export abuse, and operator failure — `3e984f0`.

## Phase 1: Fork-local contract design

- [x] Define a versioned capability-discovery and negotiation contract — `356e470`, `a02512a`.
- [x] Define configuration and rollout semantics: disabled default, allowlists/cohorts, quotas, maintenance windows, revocation, and emergency disablement — `3e984f0`.
- [x] Define authentication, identity, token scope, audit, metrics, and secret-redaction requirements — `3e984f0`.
- [x] Define request/byte/time/concurrency/retry budgets and server feedback semantics — `3e984f0`.
- [x] Define bounded bulk/export capability and its authorization and privacy boundaries — `356e470`, `a02512a`.

## Phase 2: Harness and implementation slices

- [~] Create shared offline fixtures and conformance tests in the fork repositories — fyi-cli fixture/tests complete; paired server fixture remains external.
- [x] Implement or verify fyi-cli Rust, Python, and MCP behavior against the contract — `356e470`, `a02512a`.
- [ ] Implement the fork-local Alaveteli server-side controls through separate child issues and focused PRs.
- [x] Add operator status, audit, disablement, rollback, and bounded-smoke evidence — `3e984f0`, `a02512a`.
- [ ] Run security, dependency, lint, type, property, mutation, and coverage gates appropriate to changed code.

## Phase 3: Maintainer package

- [x] Produce a short problem/solution comparison showing how the route complements existing Alaveteli controls — `3e984f0`.
- [x] Include threat model, fixtures, test evidence, rollout/rollback runbook, and known limitations — `3e984f0`, `a02512a`.
- [x] Prepare reciprocal issue drafts in fyi-cli and the fork-local Alaveteli repository — `3e984f0`.
- [ ] Obtain evidence-gate sign-off in Conductor; upstream submission remains disabled until then.

## Phase 4: Upstream engagement, only after approval

- [ ] Open one narrowly scoped upstream Alaveteli issue for maintainer discussion, linked to the evidence package.
- [ ] Wait for maintainer direction before opening any upstream PR.
- [ ] If invited, submit small upstream PRs with one concern per PR and no unrelated changes.
- [ ] Record upstream issue/PR links and maintain fork/upstream divergence evidence.

## Local verification record

- Python repository suite: 616 passed, 1 opt-in live smoke skipped.
- Rust `fyi-core`: 128 passed; Rust `fyi-mcp`: 19 passed.
- Endorsed-route Python coverage: 88%; Ruff and basedpyright pass for changed
  Python route/harness files.
- Remaining quality-gate work is the paired server-side suite plus any
  maintainer-approved security/dependency/mutation gates; upstream submission
  remains disabled.

## Closure gate

This track cannot close while the evidence gate is incomplete, while a known
risk lacks a deterministic sensor or disablement path, or while upstream work
is represented as accepted rather than merely proposed.

