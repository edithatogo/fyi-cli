# Bounded replay conformance corpus

`oracle-v1.json` is a frozen, self-digested synthetic corpus produced from the
Python replay reference under CPython 3.15.1. Its provenance pins the reference
source and selection schema with SHA-256. The checked-in expected bytes are
evidence: tests do not regenerate them or change their digest to hide drift.
The legacy reference application is not extended by this qualification work.

The Rust integration suite and Python conformance suite both execute the same
30 shared replay cases. They compare canonical results, checkpoint bytes,
stored payloads, requested URLs and bounds, and response observations. They also
reject 24 malformed selections and 10 invalid configurations. The Python suite
checks the fixture's source/schema provenance before it can establish current
reference conformance.

## Intentional differences

Two further cases record known legacy behavior separately from shared parity:

| Case | Python reference | Rust acceptance |
| --- | --- | --- |
| Compressed response with encoded Content-Length | Rejects because it compares the encoded declaration with decoded payload length. | Compares Content-Length with encoded bytes, then independently verifies the decoded payload against approval. |
| Re-signed checkpoint with object metadata that differs from approval | Accepts the recorded legacy mismatch. | Rebinds resumed objects to the approved row and rejects the mismatch. |

The Python tests preserve those observed differences. A change to either case
requires explicit contract review; matching a known legacy defect is not a Rust
acceptance criterion. Rust-only bounds, normalized transport URLs and other
adversarial regressions are tested alongside the shared corpus.

## Offline verification

Run the existing repository environment and locked Rust dependencies:

```bash
.venv/bin/python -m pytest -q tests/test_internet_archive_replay_conformance.py
cargo test --locked --offline -p fyi-core --test internet_archive_replay
cargo test --locked --offline -p fyi-core --lib internet_archive_replay::
cargo test --locked --offline -p fyi-core --lib sync::tests::replay_
cargo test --locked --offline -p fyi-cli --test replay_evidence_gate_tests
```

The Python tests block accidental default HTTP transport use. Rust responses and
the HTTP/2 retry regression use in-process fixtures, including a duplex memory
stream; no DNS lookup or socket connection is required. The existing Python CI
runs the reference tests, and the receipt portability workflow runs the focused
Rust replay, transport, evidence and CLI tests on Linux, macOS and Windows.

`.gitattributes` preserves the corpus and raw replay fixture bytes through Git
check-in and checkout. The checkout regression exercises LF, CRLF and binary
payloads under `core.autocrlf=true`.

Passing this finite corpus establishes these offline contracts only. It does
not establish live archive behavior, arbitrary Python/Rust equivalence, a
released downstream integration, NZ shadow parity or permission for archive
cutover. Those gates remain open under #309 and #312.
