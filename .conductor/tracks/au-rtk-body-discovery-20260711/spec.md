# Specification: AU RightToKnow body discovery

## Goal

Provide a read-only, auditable body-catalog interface for downstream AU taxonomy
work. The command must emit one JSON object per line with stable `url_name`,
`name`, and `tags` fields, while preserving robots-aware fetching, contactable
identity, and the shared per-instance rate limiter.

## Acceptance criteria

- `discover-bodies --format jsonl` emits valid JSONL records with `tags` always
  represented as a list.
- Missing or malformed catalog identity rows are excluded from JSONL output.
- The CLI exposes a named shared limiter so AU callers can coordinate with
  other archive workers without changing concurrency.
- Existing auditable JSON output remains compatible.
- Tests cover tag encodings, malformed rows, JSONL output, and CLI parsing.
