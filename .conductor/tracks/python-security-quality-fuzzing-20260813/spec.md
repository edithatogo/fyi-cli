# Python security, quality, and fuzzing gates

Protect the untrusted acquisition boundary with two complementary generative layers:
explicit Hypothesis PR tests and bounded Atheris coverage-guided fuzzing of receipt,
CDX, Wayback replay, and redaction parsers.

## Acceptance

- PR checks run Hypothesis explicitly and enforce Python lint, formatting, types, and audit.
- Atheris is installed at an exact version outside the shipped dependency graph, and
  Actions are commit-pinned with read-only permissions.
- Every fuzz run has time, per-input, memory, and job caps.
- Failures retain crash inputs and logs without credentials or live source data.
- Scheduled and manual runs are longer but remain capped at 30 minutes per target.
- Offline tests enforce the workflow and harness contracts.
- Track completion requires a successful hosted PR run; local verification alone is insufficient.
