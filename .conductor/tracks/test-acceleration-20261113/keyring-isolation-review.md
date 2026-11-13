# Parallel keyring isolation review — PR #418

Related: #322. This addendum corrects the earlier assertion that isolated SQLite
fixtures meant no shared-resource tests needed isolation. Credential and encryption
tests use the same persistent keyring service names and can delete another worker's
keys. A successful parallel timing sample does not disprove that race.

## Executed regression evidence

Test-only head `231bc4482d7589be3bd4d19fdfe0ea643c6e7423` ran in hosted CI
`43420517122`, Python Tests job `131869047943`:

- 865 tests collected; 861 passed, 2 skipped, **2 failed**.
- Both new isolation cases observed the shared file
  `/home/runner/.local/share/python_keyring/keyring_pass.cfg` instead of their
  unique `tmp_path / "keyring.cfg"`.
- The failing assertion precedes the test write; the red tests cannot write into
  the ordinary keyring when isolation is missing.

## Correction and verification contract

- A function-scoped autouse fixture selects the real file backend at a unique
  per-test path for the credential, encryption and isolation test modules.
- The previous backend is restored after each test and dependent fixture cleanup.
- Lazy imports preserve the metadata-only CI lane that installs pytest alone.
- A separate four-worker CI regression executes all three modules. This is
  additive; the full serial unfiltered suite and existing gates are unchanged.
- Earlier timing results are historical, not final-head performance or race-safety
  evidence. Final-head full-suite and four-worker CI must pass before promotion.

The receipt and archive acceptance gates under #310 remain separate.
