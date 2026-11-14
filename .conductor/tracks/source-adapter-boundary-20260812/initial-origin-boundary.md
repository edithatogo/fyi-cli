# Initial request origin boundary — issue #309

Status: regression implementation in progress; final-head CI required.

The shared SyncClient boundary must reject an initial request whose origin
(scheme, host, effective port) differs from its configured source. This check
must happen before bot-token parsing/attachment, validator-cache use, pacing
reservation, or any HTTP attempt. Redirect protection alone does not establish
this initial-request contract. A denied request must not echo the destination,
query parameters or token in its error.

The regression uses only two local mock servers and synthetic credentials.
Same-origin requests must retain their body, query and token behaviour.
The fix belongs to the existing send_guarded path, not a new HTTP client.

This increment does not claim DNS-rebinding protection, all-command receipt
coverage, Rust CDX/replay completion, WARC linkage or downstream archive parity.
Keep #309/#310/#312 open and `archive_cutover_allowed=false`.

Rollback: revert the focused origin guard and tests; no schema/data migration.
