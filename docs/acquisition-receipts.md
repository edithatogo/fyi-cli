# Acquisition receipts

The network acquisition commands `import-authorities`, `discover-bodies`,
`fetch-request-page`, `discover`, and `capture` accept `--receipt PATH`.
`internet-archive-cdx` requires a receipt because its only operation is a
source-network traversal.
When requested, the command emits a version `1.0.0` acquisition receipt after
the acquisition succeeds or fails. Failure receipts contain the exception type
only, never its potentially sensitive message.

Receipts use canonical UTF-8 JSON, carry a self-digest, and are validated
against the packaged Draft 2020-12 schema before immutable, atomic no-clobber
publication. Failed validation leaves any prior receipt intact. Receipt URLs
omit userinfo and fragments and replace the entire query with `REDACTED`;
response headers, cookies, and authorization values are not recorded.

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

## Immutable publication and retries

The receipt path is independent of the acquisition output path. Operators
should retain both and verify `receipt_sha256` before accepting the acquisition
downstream. Allocate a distinct receipt path for each acquisition attempt,
including failed attempts. A failure receipt never replaces existing success
or failure evidence.

Publication uses a private same-directory temporary file, file flush, and an
atomic hard link. On POSIX, the containing directory is also flushed. An
existing byte-identical, valid receipt permits an idempotent write; a different
receipt, invalid existing evidence, a symlink (including a dangling symlink),
or a directory destination is rejected. Concurrent writers proposing different
receipts cannot both publish to the same destination. A new acquisition run
usually has different timestamps, so it is not an identical receipt retry.

Filesystems that do not support the required hard link fail closed. There is
no overwrite or non-atomic copy fallback. Handled errors clean up the temporary
file. Abrupt process termination or power loss can leave a private temporary
file and is not a guarantee of automatic cleanup. If a directory flush fails
after the hard link succeeds, the call raises an error but the complete receipt
can already exist; verify that evidence before retrying the same receipt.

The offline persistence regressions cover corrupted existing data, failed
attempts, dangling symlinks, directory destinations, concurrent thread writers,
unsupported links, file/directory flush failures, and POSIX file permissions.
They do not establish cross-process crash recovery or Windows filesystem parity.
See the [verification record](../.conductor/tracks/source-adapter-boundary-20260812/receipt-persistence-verification.md).

The CDX command additionally binds the receipt to the exact pagination mode,
target host and URL pattern, capture mode, page and row caps, timestamp range,
and hash-verified checkpoint. See [Internet Archive CDX discovery](internet-archive-cdx.md).
