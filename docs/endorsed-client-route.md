# Endorsed client route: fork-local evidence package

This document defines the proposed `fyi-endorsed-client/v1` contract for a
future Alaveteli-supported client route. It is disabled by default and is an
evidence package for maintainer discussion; it does not authorize an upstream
issue or pull request.

## Capability discovery and negotiation

An instance operator may publish a JSON capability document at a deployment-
chosen capability endpoint (for example, `/.well-known/fyi-bot-capabilities`):

```json
{
  "protocol": "fyi-endorsed-client/v1",
  "instance_id": "nz-fyi",
  "enabled": true,
  "kill_switch": false,
  "revoked": false,
  "expires_at": 1800000000,
  "client_allowlist": ["fyi-cli-prod"],
  "scopes": ["read", "bulk_export"],
  "quotas": {
    "max_requests": 1000,
    "max_bytes": 52428800,
    "max_runtime_seconds": 300,
    "max_concurrency": 2,
    "max_retries": 3
  },
  "bulk_export": {
    "enabled": true,
    "scope": "bulk_export",
    "max_items": 1000,
    "max_bytes": 52428800
  }
}
```

The client accepts the route only when the protocol version, instance, expiry,
allowlist, requested scopes, and positive quotas all validate. `enabled=false`,
`kill_switch=true`, `revoked=true`, an expired document, an unknown client, or
an unknown scope fails closed. Tokens are supplied out-of-band and are never
placed in capability documents, errors, or traces.

## Configuration and rollout

- The route is opt-in and disabled unless an operator explicitly publishes an
  enabled capability document and authorizes the client cohort.
- Cohorts are represented by `client_allowlist`; deployments should issue
  scoped credentials separately for each cohort.
- Quotas are hard ceilings. The client applies the lower of local guardrails
  and negotiated quotas; it never raises local limits from server metadata.
- `expires_at`, `revoked`, and `kill_switch` provide expiry, revocation, and
  emergency disablement. Operators should rotate credentials and publish a
  disabled document before maintenance or incident response.
- Maintenance windows are an operator scheduling concern; clients must defer
  heavy work during a published window and never use `force` to bypass a kill
  switch or revocation.

## Authentication, audit, and observability

Authentication is mandatory. The fyi-cli token is sent only as the explicit
`X-FYI-Bot-Token` header when configured, alongside the traceable User-Agent.
The token is scoped to the negotiated operations and is not used as a generic
API-key fallback. Every request remains subject to RateLimit/Retry-After,
validator, pacing, byte, runtime, concurrency, and retry controls.

The Rust `fyi-core::endorsed_route` module and Python
`fyi_system.endorsed_route` module implement the same fail-closed decision
model. The MCP server may expose the resulting status as read-only context;
MCP tools must not silently enable the route or bypass local budgets.

## Bulk/export boundary

Bulk export is a separate capability and requires the explicit `bulk_export`
scope. It has independent item and byte ceilings, is auditable, and has no
fallback to recursive request retrieval when unavailable or unauthorized.
Export responses must remain bounded and privacy-reviewed before archival use.

## Threat model and controls

| Threat | Control and sensor |
|---|---|
| Accidental high-volume client | negotiated quotas plus local guardrail trip events |
| Hostile or misconfigured client | allowlist, scoped token, expiry, revocation, kill switch |
| Export abuse | separate scope, item/byte ceilings, no fallback, audit event |
| Operator uncertainty | capability/status report, JSONL trace, rollout runbook |
| Protocol drift | version field, shared JSON fixture, Rust/Python parity tests |
| Secret leakage | token excluded from capability payloads, errors, and traces |

## Rollback and evidence gate

Rollback is: disable the capability document, activate the kill switch, revoke
the client cohort, rotate the scoped token, and verify that the client fails
closed. This repository includes offline Rust/Python conformance tests and the
bounded bulk/validator tests from the bot-contract track. No upstream issue or
PR is opened until the fork-local Alaveteli controls, threat-model review,
security/quality gates, and operator sign-off are complete.
