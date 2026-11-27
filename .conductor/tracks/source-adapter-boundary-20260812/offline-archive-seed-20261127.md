# Offline archive seed evidence — 27 November 2026

## Merged handoff

FYI CLI #519 merged as `12be16891da810256908b5fbf2865dea8c3ff4fb` on
`master`. FYI Archive #448 merged as
`12de4fe51158ee43673d681aaafdc7bf813ffd75` on `main`. The archive
workflow builds both Rust commands from that exact CLI source revision.
[FYI CLI #519](https://github.com/edithatogo/fyi-cli/pull/519) ·
[FYI Archive #448](https://github.com/edithatogo/fyi-archive/pull/448)

The merged archive workflow passed all 21 tests on `main` with networking disabled.
Fictional case 701 on `source.invalid` covers request JSON, an HTML
page and a binary attachment. The test verifies the three receipts, WARC record
IDs and payload bytes, three timeline events, one linked attachment, archive
package fixity and checkpoint restoration after a follow-up deletion. The
dependency audit on the approved PR commit found zero known vulnerabilities.
[Fixture and replay instructions](https://github.com/edithatogo/fyi-archive/blob/12de4fe51158ee43673d681aaafdc7bf813ffd75/docs/offline-seed-handoff.md) ·
[Archive main CI](https://github.com/edithatogo/fyi-archive/actions/runs/45909710899/job/140214842238) ·
[Dependency audit](https://github.com/edithatogo/fyi-archive/actions/runs/45908495766/job/140210903997)

## Offline shadow comparison

Both paths used the merged archive orchestration, projection and packaging on
Python 3.15.2. The default seed ran the legacy Python `capture` command and
event exporter from the archive's lock at
`bc789a845e8b4e8b53d007c658be8070cb17e70e`. An HTTP mock supplied the unchanged
`offline-seed-v1` bodies and a fictional numeric-to-slug redirect for the
legacy lookup. The explicitly selected importer and event exporter used the Rust
CLI built from `12be1689`. Each path used its own ledger, checkpoint and package
store. A network-denied local rerun of the merged handoff suite passed all 21 tests.

| Check | Case 701 result |
| --- | --- |
| Queue and capture | Each seed completed the same case with zero failures. Both preserved the exact HTTP 200 bodies for request JSON (507 bytes) and HTML (65 bytes). The legacy seed counted the derived directory's reported size (192 bytes on this host). The Rust path counted 13,066 bytes of retained evidence and derived files. |
| Attachment | The inline item contains a URL and media type, and the HTML has no attachment link. The legacy capture requires a name for an inline item and skipped the fetch. Its exporter listed the attachment with a null size and WARC reference. Rust verified and packaged the 32-byte payload and its WARC record. The exported attachment IDs differ. |
| Events | Both projected the same case and three activities with identical order and timestamps. All three event IDs differ. The legacy event provenance uses the numeric lookup URI. |
| Receipts and packages | Rust verified the response, result, receipt and indexed WARC linkage for all three resources. The legacy WACZ contained two sequentially readable responses and an empty CDXJ index. Both revision 1 stores passed package and index verification. |
| Revisions and checkpoints | Both fresh exports started at revision 1. The checkpoint bytes differ. Feeding a copy of the legacy checkpoint to Rust emitted deletes for all three legacy IDs and three upserts under new IDs. |

## Local rollback

Removing the follow-up in the Rust working copy emitted one revision 2 delete
and reduced the active projection to two events. Restoring the legacy snapshot
returned all 37 files to their original SHA-256 digests, including the ledger,
projection, verified package store and event checkpoint. The legacy seed then
skipped the completed case with zero capture calls. The exporter emitted zero
events and preserved the attachment row. It advanced its internal checkpoint
revisions from 1 to 2 on that unchanged input.

The comparison report, before and after file inventories, checkpoints and replay
scripts are retained in the ignored local directory
`outputs/archive-shadow-parity-20261127`. The report in `run-04/comparison.json`
has SHA-256 `02da29b6e9e00572da913142f7345e76858aa68abe72cdad21de4fec38206f12`.

Exact parity remains open. The local restore covers one fictional queue item.
The wider NZ shadow, production rollback, release pinning and separately
authorised hosted reads remain open. `archive_cutover_allowed=false`.
[Integration gates](plan.md#integration) ·
[Archive shadow gate](https://github.com/edithatogo/fyi-archive/issues/378)
