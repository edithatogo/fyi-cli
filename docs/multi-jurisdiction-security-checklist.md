# Multi-jurisdiction security checklist

The multi-jurisdiction rollout introduces new risk surfaces for remote instances and cross-jurisdiction data handling.

## Immediate controls
- Validate configured instance URLs before use and reject non-HTTPS or non-public hosts.
- Preserve per-instance credential isolation in the OS keyring.
- Minimize PII retention for EU and community instances.
- Require signed release artifacts and provenance to be published alongside binary releases.

## Future hardening
- Add Tor circuit isolation and leak-canary tests for non-default instances.
- Introduce cargo-deny and cargo-audit gates in CI.
- Add fuzz targets for parser and feed handling code.
