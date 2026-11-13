# Receipt persistence verification — 13 November 2026

Related: #310; PR #413. This records a bounded verification increment, not
completion of the source-adapter programme or permission for archive cutover.

## Offline regression evidence

Added `tests/test_acquisition_receipt_persistence.py` (15 parametrised cases).
The receipt module, package initializer and packaged schema were reconstructed
from the GitHub connector in an isolated directory because the local execution
environment could not resolve GitHub. Each source file was checked against its
Git blob SHA before execution; no validator or filesystem publication code was
stubbed. The tests use the actual local filesystem and no live HTTP.

- Python: 3.13.5 on Linux.
- Baseline commit: `b178dbeab3c77dda0d8c3ddb22371c421fa083b7`.
- Baseline receipt-module blob: `dcc1798e5d7915e8789906047ed7ed5b6abaa402`.
- Reviewed PR head: `06bc3df3a77dbe5168537827fa97078788ad32ad`.
- Reviewed receipt-module blob: `d8bbc49137a4ec6164bdea185c5485e5f1628529`.
- Schema blob: `cd79b9153c89be58ac3899bf24cbcfc5598113a8`.
- Command: `python -m pytest tests/test_acquisition_receipt_persistence.py -q --tb=short`.
- Baseline result: **12 failed, 3 passed**. Failures demonstrate overwritten
  existing evidence, concurrent distinct writers both succeeding, and missing
  hard-link/directory-flush failure semantics.
- Reviewed writer result: **15 passed**; repeated after test-formatting edits.

Coverage includes invalid existing JSON and UTF-8, failure receipts preserving
success evidence, dangling symlinks, directory destinations, concurrent thread
writers, unsupported hard links, file/directory fsync failures and POSIX mode
0600. The post-publication directory-fsync test verifies that complete evidence
can already exist despite the reported error and permits an identical retry.

## Scope and remaining acceptance

This was a focused module test, not a local run of the entire repository suite.
The final PR head must obtain its own hosted CI results after incorporating
these tests and refreshing from master. The offline AI security workflow was
queued without an assigned `ai-security-offline` self-hosted runner during
review; that is not a passing scan. No scanner or branch protection was disabled.

Cross-process crash/power-loss recovery, Windows filesystem parity, complete
network-command coverage and downstream archive shadow parity remain open.
The documentation now distinguishes handled-error cleanup from abrupt process
termination and no longer claims failed acquisitions replace existing receipts.
