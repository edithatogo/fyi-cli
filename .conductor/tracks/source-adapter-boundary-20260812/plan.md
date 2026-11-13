# Plan

## Independent workstreams

- [ ] #310: define the acquisition-receipt schema and atomic writer, then add it
  to every existing network command.
- [~] #311: implement resumable Internet Archive CDX discovery with parity
  fixtures derived from the current `fyi-archive` behavior.
- [~] #312: implement bounded, allowlisted Wayback replay with byte receipts.
  Offline implementation and adversarial parity fixtures are present; hosted
  smoke, downstream old/new parity, pinned release integration, NZ shadow
  parity, and archive cutover remain open acceptance gates.
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

## 2026-11-13 Rust receipt verification tranche

- [x] Check in Python-origin v1 receipt fixture at `conformance/acquisition/receipt-v1-python.json` with a Python-side schema and digest test.
- [~] Implement Rust offline canonical JSON and receipt verifier at `crates/fyi-core/src/acquisition_receipts.rs` with cross-language golden, tampering, unsafe URL, total consistency and ordering invariants; hosted CI remains acceptance evidence, not yet complete.
- [ ] Expand Rust verification to full JSON Schema 2020-12 equivalence, including unknown-field rejection, numeric/UTC semantics, and Unicode/float/escaping cross-runtime corpus.
- [ ] Add fail-closed, durable Rust atomic receipt writer that never silently overwrites valid immutable evidence; add power-loss and concurrency tests.
- [ ] Integrate receipts with all Rust source-network commands, and pin client & downstream `fyi-archive` consumers after shadow parity; existing #310 remains **open**.
- [ ] Verify WARC/WACZ raw byte capture linkage and publish independently reproducible signed manifests.

The new Rust verifier is read-only and does not claim write-path or network acquisition parity. Keep #310 active and do not alter archive cutover gate.
