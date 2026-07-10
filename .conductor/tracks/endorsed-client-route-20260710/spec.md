# Track Specification: Endorsed Client Route

## Outcome

Produce a maintainer-readable proposal for an Alaveteli-supported, opt-in
client route that fyi-cli and MCP can use when an instance sysadmin explicitly
enables it. The route should make bounded, identifiable, observable behavior
easy for ordinary users while preserving the ability to disable it immediately.

This track prepares a future upstream conversation. It does not authorize an
upstream issue or pull request before the evidence gate is satisfied.

## Design principles

- Disabled by default; fail closed when configuration or authorization is
  ambiguous.
- The sysadmin controls enablement, client cohorts, quotas, maintenance
  windows, observability, revocation, and the kill switch.
- Authentication and authorization remain mandatory; the route never creates
  anonymous privileged access or a bypass around existing abuse controls.
- Clients identify themselves, honor server feedback, use validators where
  safe, and enforce request/byte/time/concurrency/retry budgets locally.
- Bulk or export behavior is separately enabled, bounded, auditable, and never
  an implicit fallback for unavailable or unauthorized endpoints.
- Errors and traces are secret-free and operationally useful.
- The route complements ordinary web/API controls; it is not presented as a
  complete solution to all bot traffic.

## Threats and controls

| Threat | Required control | Evidence |
|---|---|---|
| Accidental high-volume client | client and instance budgets, bounded retries, back-pressure | deterministic trip tests |
| Misconfigured or hostile client | scoped credentials, revocation, server quotas, kill switch | authorization and disablement tests |
| Sensitive or unbounded export | explicit capability, bounded jobs, privacy checks, audit | negative-path fixtures |
| Operator uncertainty | status, metrics, audit events, rollout/runbook | operator documentation tests |
| Protocol drift | versioned capability discovery and shared fixtures | conformance suite |
| Trace or error leakage | redaction and structured secret-free events | scanner and assertion evidence |

## Evidence gate

Upstream engagement remains disabled until the fork-local client/server work
has deterministic offline evidence, a threat-model review, explicit rollback and
disablement behavior, no known unresolved security/quality/availability risk,
and a prepared maintainer package. An upstream issue may then be opened for
discussion; upstream code is still subject to maintainer agreement and separate
small PRs.

