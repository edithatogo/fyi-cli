# Implementation Plan: AU RightToKnow live smoke

## Phase 1 — Bounded sensor

- [x] Add opt-in AU discover-plus-capture smoke test.
- [x] Bound pages, IDs, pacing, runtime, bytes, and output location.
- [x] Preserve default offline CI and document execution.
- [x] Run focused offline checks and harness validation.

## Phase 2 — Handover

- [x] Open a focused PR closing GitHub issue #136 (PR #187).
- [x] Record live execution outcome: RightToKnow returned HTTP 403 before capture; the smoke reports an explicit external-availability skip.
