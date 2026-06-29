# Track: archive-health-doctor

## Objective

Surface the archive's health as machine-readable signals that `fyi-archive` can
assemble into a parity report and CI gate. This track computes the *raw* health
metrics from the captured corpus; `fyi-archive`'s `doctor` cross-references them
against the mirrors (HF/Zenodo/OSF).

## Scope

- `fyi archive-health` command producing a JSON report:
  - `freshness`: last successful capture run timestamp; last successful diff.
  - `coverage`: total discovered vs captured counts; missing request IDs (discovered
    but not yet captured); authorities with zero captures.
  - `counts`: captured record count, attachment count, total bytes, WACZ count.
  - `warnings`: stale data (> N days since last capture), large coverage gaps,
    consecutive failed runs.
- Idempotent and side-effect-free: reads existing capture/manifest/state only.
- Deterministic output so `fyi-archive` can diff it between runs to detect drift.

## Out of scope

- Cross-mirror parity computation (that's `fyi-archive doctor`, which consumes this
  report + mirror counts).
- Network probing of fyi.org.nz beyond what capture/diff already do.

## Acceptance criteria

- [x] `fyi archive-health` emits a stable JSON schema; documented in a schema file.
- [x] Freshness reflects the real last-success timestamps from `sync_state`/ledger.
- [x] Coverage gaps list the concrete missing IDs/authorities.
- [x] Tests: synthetic corpus fixtures produce expected reports; determinism check.

## Risks

- Very large gap lists → paginate/truncate with a count + sample.
