# Specification: AU RightToKnow live smoke

## Goal

Provide an explicit, bounded live smoke sensor for the AU RightToKnow
discovery/capture contract without adding network work to default CI.

## Acceptance criteria

- The smoke is skipped unless `FYI_LIVE_SMOKE=1`.
- It discovers at most one page with the default one-second pacing and a named
  shared limiter.
- It captures no more than five discovered request IDs, read-only, into a
  temporary test directory with byte and runtime caps.
- Existing default CI remains offline and the operator command is documented.
