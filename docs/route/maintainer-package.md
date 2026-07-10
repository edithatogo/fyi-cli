# Endorsed client route: maintainer package

Status: fork-local preparation only. Do not open an upstream issue or PR until
the evidence gate in `.conductor/tracks/endorsed-client-route-20260710/` is
signed off.

## Problem and solution comparison

| Concern | Ordinary Alaveteli web/API controls | Endorsed client route |
|---|---|---|
| Identification | User-Agent and ordinary credentials | Traceable User-Agent plus scoped cohort token |
| Rate pressure | Server-side limits and Retry-After | Server feedback plus client pacing and hard local budgets |
| Bulk work | General API endpoints | Separate capability, item/byte ceilings, explicit authorization |
| Operator control | Existing deployment controls | Allowlist, expiry, revocation, maintenance scheduling, kill switch |
| Observability | Server logs and metrics | Correlated client status/JSONL events without token or body secrets |
| Rollback | Disable credentials or endpoints | Publish disabled/revoked capability and stop the client route immediately |

The route complements ordinary controls; it is not an exemption from them and
does not make anonymous privileged access possible.

## Rollout / rollback runbook

1. Publish a versioned capability document with `enabled=false` while checking
   the client identifier, scopes, quota values, expiry, and operator contacts.
2. Issue a narrowly scoped credential to one allowlisted cohort and enable only
   read operations first. Confirm status and audit events remain secret-free.
3. Observe server rate-limit and latency metrics during a bounded smoke window.
4. Enable `bulk_export` separately only after privacy review and a successful
   item/byte-bounded fixture run.
5. For rollback, set `kill_switch=true` or `enabled=false`, revoke the cohort,
   rotate the credential, and verify `endorsed_route_status` returns `denied`.
6. Keep the route disabled until an operator explicitly re-enables it with a
   fresh expiry and reviewed quotas.

## Evidence and known limitations

- Rust and Python authorize the same shared JSON fixture and fail closed for
  disabled, killed, revoked, expired, unknown-client, unknown-scope, and
  unauthorized-bulk cases.
- MCP exposes only local read-only evaluation; it cannot fetch, enable, or
  mutate a remote capability document.
- Fork-local Alaveteli server controls, real operator metrics, and maintainer
  sign-off are external evidence-gate dependencies. They must be completed in
  the paired repository before any upstream submission.
- Live smoke remains opt-in and must use small quotas, a bounded window, and an
  explicitly authorized test cohort.
