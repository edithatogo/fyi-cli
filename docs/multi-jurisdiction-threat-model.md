# Multi-jurisdiction threat model

This note captures the highest-risk attack surfaces for the multi-jurisdiction FOI platform and the mitigation strategies implemented in the Rust core.

## Assets
- Jurisdiction catalog data and instance metadata.
- Request content and correspondence imported from FOI platforms.
- API credentials and local sync state.
- Release artifacts and provenance metadata.

## Threats
1. SSRF via user-supplied instance URLs.
2. Credential leakage across instances.
3. Data exfiltration of sensitive personal information to the wrong jurisdiction.
4. Supply-chain compromise through Rust dependencies.
5. Correlation of Tor circuits across jurisdictions.

## Mitigations
- Base URLs are validated before use; public hosts must use HTTPS, internal/private hosts are rejected, and query strings/fragments are blocked.
- Credential handling is scoped to the instance layer and the code path should avoid cross-instance reuse.
- EU/PII-related flows should minimize data retention and expose erasure hooks.
- CI now runs cargo-audit and cargo-deny to block known vulnerable dependencies and license policy violations.
- Release artifacts should be signed and accompanied by SBOM/provenance metadata once the release pipeline is fully wired.
