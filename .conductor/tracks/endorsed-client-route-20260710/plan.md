# Implementation Plan: Endorsed Client Route

## Phase 0: Proposal and evidence boundary

- [x] Open fyi-cli planning issue #148 with explicit upstream-submission gate.
- [x] Record opt-in, sysadmin-controlled, kill-switchable design principles.
- [ ] Reconcile the proposal with existing fyi-cli #140/#142-#144 and Alaveteli #23-#27 without duplicating contracts.
- [ ] Write a threat model covering accidental overload, hostile clients, credential misuse, privacy, export abuse, and operator failure.

## Phase 1: Fork-local contract design

- [ ] Define a versioned capability-discovery and negotiation contract.
- [ ] Define configuration and rollout semantics: disabled default, allowlists/cohorts, quotas, maintenance windows, revocation, and emergency disablement.
- [ ] Define authentication, identity, token scope, audit, metrics, and secret-redaction requirements.
- [ ] Define request/byte/time/concurrency/retry budgets and server feedback semantics.
- [ ] Define bounded bulk/export capability and its authorization and privacy boundaries.

## Phase 2: Harness and implementation slices

- [ ] Create shared offline fixtures and conformance tests in the fork repositories.
- [ ] Implement or verify fyi-cli Rust, Python, and MCP behavior against the contract.
- [ ] Implement the fork-local Alaveteli server-side controls through separate child issues and focused PRs.
- [ ] Add operator status, audit, disablement, rollback, and bounded-smoke evidence.
- [ ] Run security, dependency, lint, type, property, mutation, and coverage gates appropriate to changed code.

## Phase 3: Maintainer package

- [ ] Produce a short problem/solution comparison showing how the route complements existing Alaveteli controls.
- [ ] Include threat model, fixtures, test evidence, rollout/rollback runbook, and known limitations.
- [ ] Prepare reciprocal issue drafts in fyi-cli and the fork-local Alaveteli repository.
- [ ] Obtain evidence-gate sign-off in Conductor; upstream submission remains disabled until then.

## Phase 4: Upstream engagement, only after approval

- [ ] Open one narrowly scoped upstream Alaveteli issue for maintainer discussion, linked to the evidence package.
- [ ] Wait for maintainer direction before opening any upstream PR.
- [ ] If invited, submit small upstream PRs with one concern per PR and no unrelated changes.
- [ ] Record upstream issue/PR links and maintain fork/upstream divergence evidence.

## Closure gate

This track cannot close while the evidence gate is incomplete, while a known
risk lacks a deterministic sensor or disablement path, or while upstream work
is represented as accepted rather than merely proposed.

