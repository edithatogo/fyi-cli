# Acquisition receipts

The network acquisition commands `import-authorities`, `discover-bodies`,
`fetch-request-page`, `discover`, and `capture` accept `--receipt PATH`.
When requested, the command emits a version `1.0.0` acquisition receipt after
the acquisition succeeds or fails. Failure receipts contain the exception type
only, never its potentially sensitive message.

Receipts use canonical UTF-8 JSON, carry a self-digest, and are validated
against the packaged Draft 2020-12 schema before an atomic replacement. A
failed validation leaves any prior receipt intact. Receipt URLs omit userinfo
and fragments and replace every query value with `REDACTED`; response headers,
cookies, authorization values, and credential material are never recorded.

The contract records:

- command and adapter identity/version;
- source and request bounds;
- start and completion timestamps;
- ordered HTTP status, byte count, payload digest, attempt count, and retry delays;
- aggregate request, byte, and retry totals;
- configured rate-limit identity and minimum interval;
- checkpoint digests before and after the command; and
- a canonical result projection's media type, byte count, and SHA-256 digest.

The `result` digest is explicitly a `canonical_result_projection`. It is a
stable, compact representation of the command result and is not necessarily a
digest of literal terminal output or an output file. File-producing consumers
must verify those files through their own manifests or digests.

The receipt path is independent of the acquisition output path. Operators
should retain both and verify `receipt_sha256` before accepting the acquisition
downstream. An acquisition failure atomically replaces the prior receipt with a
schema-valid failed receipt; interrupted or invalid receipt writes leave no
partial temporary receipt.
