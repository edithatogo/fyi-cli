# Plan

## Independent workstreams

- [ ] #310: define the acquisition-receipt schema and atomic writer, then add it
  to every existing network command.
- [ ] #311: implement resumable Internet Archive CDX discovery with parity
  fixtures derived from the current `fyi-archive` behavior.
- [ ] #312: implement bounded, allowlisted Wayback replay with byte receipts.
- [ ] #313: add source probing, shared capture pacing, `Retry-After`, and
  checkpointed numeric-ID discovery.
- [ ] #314: add narrowly allowlisted external historical-index retrieval.

These workstreams may proceed in parallel because they share only the receipt
contract. Merge #310 first or use its fixture branch as the temporary base.

## Integration

- [ ] Publish a pinned `fyi-cli` release/commit containing the adapters.
- [ ] Convert `fyi-archive` source-facing scripts into thin command wrappers.
- [ ] Add a static boundary test rejecting new source HTTP clients outside
  `fyi-cli` while allowing archive/publication provider APIs.
- [ ] Run offline output parity and hosted bounded smoke tests.
- [ ] Run NZ shadow parity for queues, captures, events, attachments, revisions,
  receipts, and checkpoints before removing legacy transports.
