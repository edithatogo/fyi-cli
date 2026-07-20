# Specification: Versioned public-safe process-event export

GitHub issue: https://github.com/edithatogo/fyi-cli/issues/231  
Parent epic: https://github.com/edithatogo/foi-process/issues/36

## Overview

Extend the capture boundary with a versioned process-event projection derived from
Alaveteli `info_request_events`. The projection supplies downstream archive and mining
systems with stable identity, source ordering, timestamps, state transitions, and
provenance without publishing correspondence text or requester identity.

## Functional requirements

- Define a portable process-event contract and shared fixture version.
- Preserve source array ordering separately from timestamp ordering.
- Emit stable case, logical-event, revision, and source-event identifiers.
- Represent corrections, deletions, and resumable checkpoint positions.
- Expose attachment metadata and WARC linkage without attachment bytes.
- Keep the existing guarded EvidenceDelta contract compatible or version it explicitly.

## Privacy and rights boundary

The default projection excludes requester identity, request title, message body, OCR
text, embeddings, and attachment bytes. It may retain public platform identifiers and
source URLs where required for pragmatic traceability. A future confidential mode must
support keyed pseudonymous identifiers and keep the re-identification mapping external.

## Acceptance criteria

- A pinned capture produces byte-deterministic output on repeated export.
- Event ordering exactly matches the source timeline.
- Contract fixtures are consumed by both downstream repositories.
- Missing or malformed timestamps are represented explicitly, not guessed.
- Recursive privacy tests reject excluded fields and raw content.
- Backfill can resume without duplicate logical events.

## Out of scope

- Process mining and dashboard aggregation.
- Archive publication orchestration.
- OCR, NLP, embeddings, or raw-content publication.
