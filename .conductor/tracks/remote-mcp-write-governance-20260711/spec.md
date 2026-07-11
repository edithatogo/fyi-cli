# Specification: Remote MCP write governance

## Overview

Add remote Alaveteli write tools only after the read surface, policy foundation, and compatibility harness are complete. Writes must be explicitly enabled, reviewable before execution, idempotent where possible, replay-resistant, bounded, and fully audited.

## Functional requirements

1. Add prepare/commit flows for request creation, correspondence, bounded attachments, and state updates.
2. Require separate per-instance remote-write capability and protected credentials.
3. Issue short-lived single-use confirmation tokens bound to operation, instance, normalized payload hash, and expiry.
4. Support caller idempotency keys and durable replay detection.
5. Use `update_request_state_if_current` for optimistic state concurrency.
6. Bound attachment count, per-file bytes, total bytes, MIME metadata, path handling, duration, and retries.
7. Emit durable redacted audit records for prepare, commit, reject, replay, success, and failure.

## Non-functional requirements

- No write tool is discoverable or callable unless explicitly enabled.
- No credentials or raw attachment contents in MCP arguments, errors, logs, or traces.
- Backward compatible with all local-only and read-only tools.
- Offline deterministic tests; live write smoke requires a dedicated sandbox and explicit opt-in.

## Acceptance criteria

- Prepare never writes; commit requires a valid matching single-use confirmation.
- Duplicate or expired confirmation/idempotency attempts fail closed.
- Stale state and oversized attachment attempts produce safe structured errors before mutation.
- Unit, integration, end-to-end, property, mutation, security, and rollback tests pass.
- Operator documentation and issue #171/epic #169 evidence are complete.

## Out of scope

- Unattended autonomous remote writes.
- Bulk write operations.
- Credentials supplied by an MCP caller.
