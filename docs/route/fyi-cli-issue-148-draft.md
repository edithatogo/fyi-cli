# Draft: fyi-cli endorsed-client route follow-up

This is a draft for issue #148 only. It must not be opened or sent upstream
until the Conductor evidence gate is approved.

## Proposal

Add an opt-in `fyi-endorsed-client/v1` capability negotiation path. An instance
operator publishes quotas, scopes, client cohorts, expiry, revocation, and a
kill switch. fyi-cli evaluates the document fail-closed and continues to apply
its local identity, RateLimit/Retry-After, validator, pacing, cache, trace, and
request/byte/time/concurrency/retry controls.

## Evidence attached locally

- `docs/endorsed-client-route.md`
- `docs/route/maintainer-package.md`
- `tests/fixtures/endorsed-client-route/enabled.json`
- Rust/Python parity tests and MCP read-only status tests

## Explicit non-goals

No anonymous bypass, no automatic enablement, no unbounded export fallback, no
upstream code change, and no claim that the route replaces server abuse
controls.
