# Remote MCP security and operations

Remote Alaveteli capabilities are disabled by default; local tools remain available. The MCP
server accepts an explicit, credential-free HTTPS allowlist through
`FYI_MCP_REMOTE_INSTANCES`:

```text
FYI_MCP_REMOTE_INSTANCES=nz=https://foi-repository.example
FYI_MCP_REMOTE_READ=nz
# Leave unset unless governed writes are explicitly approved.
FYI_MCP_REMOTE_WRITE=
```

The configuration is fail-closed. Malformed instance IDs, wildcard hosts,
credentials, non-HTTPS URLs, duplicate IDs, and zero budgets stop startup.
Read and write capabilities are independent; enabling read never enables
write. Remote budgets are bounded per instance for request count, response
bytes, and wall-clock duration.

Each instance ID selects its protected bot-token environment variable by ASCII
uppercasing the ID and replacing `-` with `_`. For example, `north-foi` uses
`FYI_MCP_REMOTE_BOT_TOKEN_NORTH_FOI`. These existing names are preserved, but
they must be unique across the complete allowlist: `north-foi` and `north_foi`,
or `NZ` and `nz`, cannot coexist. The check also applies to read-only or disabled
entries so a later capability change cannot introduce an ambiguous mapping.
Credential lookup and startup validation use the same name derivation.

An ambiguous configuration fails before database creation or protocol output;
its diagnostic does not echo IDs, endpoints, paths or token values. To migrate
such a configuration, choose distinct instance IDs, update the corresponding
allowlist/read/write settings and protected credential names, then restart.
MCP callers must use those exact configured IDs; ID matching is still case
sensitive. No token value is accepted as an MCP argument.

The MCP request counter measures admitted tool operations, including failures;
raw HTTP retries within an operation belong to the source transport's separate
limits. One operation may be active per instance. The response budget measures
serialized MCP result bytes before publication, and does not establish a raw
download or heap allocation bound.

## Operator controls

The policy boundary exposes a kill switch, degraded read-only mode, and a
three-failure circuit breaker with deterministic recovery.

Set `FYI_MCP_REMOTE_DEGRADED=true` (or `1`) when starting the MCP subprocess
to suppress remote writes while keeping configured remote reads and local tools
available. Degraded discovery omits the three remote write tools; direct calls
still pass through the policy and confirmation gates. The setting grants no
additional read or write capabilities.

The accepted values are exactly `true`, `1`, `false`, and `0`. An absent setting
defaults to normal mode. Empty, misspelled, whitespace-padded or non-Unicode
values fail startup before database creation or protocol output. The error names
the setting and accepted values without repeating its contents; startup telemetry
records only the resulting boolean alongside the existing remote-enabled flag.

Recovery is **restart-based**: stop the subprocess, set the value to `false` or
`0` (or remove it), and launch a new process. Environment changes do not alter
an already-running process, and there is no caller-accessible mode-toggle tool.
The configured per-instance capabilities still apply after recovery. Prepare a
fresh confirmation after restarting. Existing attempted idempotency scopes stay
blocked by the durable journal; a restart does not authorize their redispatch.
The SDK acceptance tests exercise this startup/restart path, not a live in-process
operator-control channel or a successful upstream write.

Operators should
disable the process or clear `FYI_MCP_REMOTE_INSTANCES` before rotating
credentials or investigating an upstream incident. Restarting the local MCP
process resets transient circuit and budget state; durable request data stays
in the local SQLite database.

## Privacy and retention

Read-operation completion logs contain server-generated correlation IDs, an
operation name, configured instance ID and coarse outcome. The reusable policy
audit-event structure additionally supports a schema version and error class;
its token normalization is not a substitute for excluding private values.
Neither logs nor audit structures may receive credentials, authorization
headers, URLs with query strings, request bodies, attachments, PII, or upstream
response content. Production writes also emit durable fixed-label events described
below. General diagnostic retention follows the operator's incident-response
policy; write reservations have a separate safety requirement and must not be
expired or removed while their scopes could be retried.

## Threat model

The boundary assumes an untrusted MCP caller, hostile or malformed upstream
responses, accidental operator misconfiguration, and a compromised local
workspace. The allowlist and existing `SyncClient` SSRF checks constrain
destinations; resource-aware pacing and budgets constrain load; redacted
structured events constrain disclosure; kill switch and circuit breaker
constrain blast radius. Remote writes require the separate confirmation,
idempotency, and replay-protection flow. They also require a protected
per-instance bot token (`FYI_MCP_REMOTE_BOT_TOKEN_<INSTANCE_ID>`) when the
upstream instance needs one; token values are never included in responses or
telemetry.

Attachment writes additionally require `FYI_MCP_ATTACHMENT_ROOT`. Paths are
relative to that canonical directory; symlinks, traversal, unsupported MIME
types, more than eight files, files over 50 MiB, and aggregate payloads over
100 MiB are rejected before any remote request is sent.

The confirmation cache is bounded and process-local: unused tokens expire in
300 seconds; accepted tokens bind the exact preparation; invalid tokens do not
consume dispatch allowance. Completed/uncertain writes are retained until
process exit, with 128 total entries and 64 KiB serialized preparation/result
limits. Completed replay does not spend another request allowance. A separate
durable reservation prevents the same scope from dispatching after restart.

Attachment path checks reject observed symlinks before canonicalization.
Because the source API subsequently reopens paths, content changes between
preparation and submission and filesystem races remain unverified. The MCP
contract track records this boundary as a closure dependency.

## Durable write state and recovery

Write-enabled configurations must set `FYI_MCP_REMOTE_WRITE_STATE_DIR` to an
absolute directory on reliable local storage. This also applies when starting
temporarily in degraded mode. Read-only configurations need no journal. A missing
setting or unusable journal stops startup before the application database is
created. The setting is protected operator configuration, never an MCP argument.

Use one stable directory for every process serving the same configured instance
IDs. Its existing parent must be trusted and must already exist; the server does
not create arbitrary parent paths. On Unix, the server creates the directory
with mode `0700` and files with `0600`, rejects symlinks at the directory/file
boundary and rejects group/other permissions. Other platforms inherit the
operator's directory ACL; Windows/macOS runtime and ACL qualification remain
open. These checks do not defend against an attacker controlling the service
account, its trusted parent directory or its backups.

The v1 directory contains `remote-write-v1.sqlite3` and
`remote-write-v1.marker`. Keep both together. An incomplete pair, changed marker
or key, unknown format, failed SQLite integrity check or oversized database is
rejected, never silently recreated. Back up while all writers are stopped using
a protected consistent copy; do not copy an active database file by itself.

Before source I/O, one SQLite transaction reserves the exact instance,
operation and caller idempotency key and appends a `commit` event. Separate
connections serialize reservation admission. Success changes the reservation to
`succeeded`; failure changes it to `uncertain`. Cancellation or process exit can
leave it `reserved`. All three states continue to deny another dispatch.
Preparation alone does not reserve a dispatch, and confirmations remain
short-lived process-local capabilities.

The journal stores only a keyed, length-delimited HMAC-SHA256 scope tag, fixed
operation/event/outcome labels, sequence numbers and UTC millisecond timestamps.
Its random per-journal key is private local metadata. It does not store raw
instance IDs, idempotency keys, confirmation or bot tokens, paths, request bodies,
attachments, results or upstream errors. Events cover `prepare`, `commit`,
`reject`, `replay`, `success` and `failure`; rejected arguments are never recorded.
Sequence order is authoritative within a journal; timestamps can move with the
system clock. The log is not a tamper-evident audit service.

The journal admits at most 10,000 attempted scopes and 100,000 events, with a
64 MiB SQLite main-database page limit and one connection per gate. SQLite uses
full synchronization and a rollback journal. These storage bounds do not claim a
whole-process heap limit or include every filesystem temporary/backup file.
Capacity or storage failure blocks preparation/dispatch/result publication.
The code never evicts attempted keys, rotates the journal, or retries a write
because journal persistence failed. No power-loss or filesystem fault tolerance
beyond the tested SQLite transaction/restart behavior is claimed.

After a lost result, timeout, restart or journal failure:

1. Disable remote writes and stop all processes using the journal.
2. Preserve the protected database/marker pair and available client receipts.
3. Reconcile the upstream request using an authorized read-only path. A local
   `reserved`/`uncertain` state cannot establish whether the server applied it.
4. Restore writes only after deciding the outcome. Never change an instance ID,
   choose another state directory, delete both files, or issue a fresh key merely
   to bypass a replay denial. These operator actions reset the local namespace.

There is deliberately no automated tombstone deletion or uncertain-write retry.
After restart a recorded success is also denied, since private results are not
stored for replay. Rollback to a process-local-only binary would remove the
durability guarantee: keep writes disabled during such a rollback and retain the
journal. This local contract does not establish remote idempotency, atomic
upstream compare-and-set or immutable attachment bytes.

Issue: https://github.com/edithatogo/fyi-cli/issues/172
