# Private local request transfer v1

`requests-v1-transfer.json` is the normative synthetic example. This format is
the retained local management domain, not a whole-database backup. No command
fetches or submits remote data. The complete payload is sensitive: it includes
request bodies and original linked evidence. Export is an explicit private read.

## Envelope and identity

All keys are required. Unknown keys, duplicate keys, missing keys, trailing
content and incorrect JSON types are rejected. Nullable keys must explicitly
contain either their stated value type or null. Inputs are UTF-8 JSON bounded
to 16 MiB. `format` is exactly `fyi-local-requests`; `schema_version` is integer
1. Future or unknown versions are rejected before mutation.

| Field | Contract |
|---|---|
| `authorities` | Existing three-field `slug`, `name`, nullable `url` model; unique slugs |
| `requests` | Existing twelve-field local request model; unique positive signed 64-bit local IDs; explicit nullable fields |
| `events` | Local history: `id`, `tracked_request_id`, `kind`, nullable `old_status`, `new_status`, `occurred_at`; positive IDs; kind `created` or `status_changed` |
| `feed_events` | Eleven original Python fields; only records linked by non-null local `tracked_request_id` to an exported request |
| `request_snapshots` | Five original Python fields; only records whose external `fyi_request_id` is represented by an exported request |
| `sequences` | Four integer high-water marks: `tracked_requests`, `local_request_events`, `feed_events`, `request_snapshots`; each nonnegative and at least its greatest exported ID |

Local and external IDs are separate namespaces. Every native request refers to
an authority in the document. Every local history/feed parent is present in the
document. Native creation/status events must be internally valid. Source raw
JSON is retained as text, including malformed historical payloads; transfer
does not claim that source contents are valid or verified evidence.

Export takes one read transaction. Authorities sort by slug and records by ID.
There is no volatile export timestamp. Original strings, nulls and IDs remain
unchanged. Include allocator high-water values even if the highest allocated
record was deleted or belongs to an omitted unrelated source record. Repeated
export of unchanged state has identical bytes.

New files use the existing atomic no-clobber publication library and Unix mode
0600. An identical existing destination succeeds unchanged; a different file,
directory or symlink fails without replacement. Windows permission behavior is
governed by the existing filesystem ACL; POSIX mode is not claimed there.

## Import policy

The default transaction inserts missing keyed records, accepts identical records
as no-ops and rejects conflicts. Duplicate document IDs are always invalid.
The acknowledgement counts input requests, including identical no-op records.
No synthetic history, timestamps or ID remapping are introduced.

`--replace` replaces the local request set and local event ledger atomically.
It preserves omitted authority catalogue rows; supplied authority changes are
allowed only in this explicit mode. Source evidence remains immutable:
existing feed/snapshot IDs accept identical contents only. Replacement must
not orphan retained linked feeds, rebind their request to a different external
identity, detach previously linked snapshots, or reparent an event ID. A
validation failure or later SQL error rolls the complete transaction back.
Unrelated source records, caches, run logs and every remote/sync table remain
unchanged. ID sequences never decrease, including when replacing with an empty
request set.

Legacy Python arrays are explicitly recognized separately. All twelve fields
and original positive IDs are required; references must match existing
authorities. They do not include authority or source/history provenance.
They may be imported idempotently without remapping IDs. Legacy-array
replacement is rejected if it would discard existing local history or detach
linked source evidence; use the versioned document for those operations.
Python's append-with-new-IDs behavior is intentionally not reproduced.

## Existing databases

Ordinary commands require a current verified native schema. `init-db` may
initialize an empty database or explicitly upgrade a known native migration
prefix. It continues to refuse unrecognized legacy data.

`init-db --migrate-legacy --db PATH` requires an existing regular nonsymlink
file. It recognizes the complete known unversioned Python layout, including
all eight tables, default declarations, keys, allocator state and references.
Unsupported versions, malformed layouts, unexplained hybrids, invalid
references and unknown triggers fail before adoption. The original tables and
data are retained in place. Rust's migration ledger and additional native
tables are created through the existing embedded migrator inside one immediate
transaction on the same connection. Migration does not invent history.

Repeated migration of a valid current database succeeds without changes.
Linked legacy feed/snapshot projections become readable in native local
timeline/detail. Timestamp strings are preserved, while ordering uses parsed
chronology. No derived follow-up, analysis or recommendation report is implied.
