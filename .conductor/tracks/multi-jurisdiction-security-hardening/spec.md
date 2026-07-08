# Specification: multi-jurisdiction-security-hardening

## Overview
This track implements comprehensive security hardening for the multi-jurisdiction platform. It addresses security concerns raised by user-configurable instances, including SSRF prevention, credential isolation, GDPR compliance, supply chain security, anonymity features, and fuzzing.

## Functional Requirements
1. **SSRF / URL Validation:**
   - User-supplied instance URLs are a real SSRF (Server-Side Request Forgery) vector
   - Implement allowlist validation for instance base URLs
   - Strict scheme validation (https:// only)
   - Host validation against internal network ranges
   - Prevent DNS rebinding attacks
   - Block access to internal/private IP ranges (RFC 1918, RFC 4193)
2. **Per-Instance Credential Isolation:**
   - Ensure no cross-instance API key leakage
   - Keyring namespace enforcement
   - Test credential isolation boundaries
   - Audit credential access paths
3. **GDPR/PII Handling:**
   - Data minimization for EU instances
   - Retention policy enforcement
   - Right-to-erasure implementation
   - Privacy impact assessments
   - Consent management
4. **Supply Chain Security:**
   - Integrate `cargo-deny` in CI for license/advisory checks
   - Integrate `cargo-audit` for vulnerability scanning
   - Generate SBOM (Software Bill of Materials) in CycloneDX format
   - Sign release artifacts with **sigstore/cosign**
   - Implement SLSA provenance for builds
5. **Anonymity Hardening:**
   - Per-instance Tor circuit isolation
   - Prevent circuit correlation across instances
   - Leak canary tests (detect non-Tor traffic)
   - DNS leak prevention
6. **Fuzzing:**
   - Integrate `cargo-fuzz` for API parsers
   - Fuzz feed parsers (Atom, RSS, JSON)
   - Fuzz HTTP response handlers
   - Continuous fuzzing in CI
7. **Threat Modeling:**
   - Document threat model for multi-jurisdiction platform
   - Identify attack surfaces
   - Risk assessment and mitigation strategies
   - Optional: External penetration test checklist

## Non-Functional Requirements
- **Security:** Zero high-severity vulnerabilities in production
- **Performance:** Security checks add <50ms latency
- **Maintainability:** Security gates automated in CI/CD
- **Compliance:** GDPR-compliant for EU instances

## Acceptance Criteria
- SSRF prevention tested and verified (cannot reach internal IPs)
- Credential isolation tests passing
- GDPR compliance implemented for EU instances
- cargo-deny and cargo-audit integrated in CI
- SBOM generation automated
- Signed release artifacts with cosign
- Per-instance Tor circuits isolated
- Leak canary tests detect non-Tor traffic
- Fuzzing infrastructure running in CI
- Threat model documented
- All security tests passing

## Out of Scope
- Security audits by third parties (can be done later)
- Bug bounty program setup
- Real-time threat monitoring (future enhancement)

## Dependencies
- Depends on: `jurisdiction-abstraction-core` (track 2)

## Success Metrics
- **Vulnerability Count:** Zero high-severity issues
- **SSRF Prevention:** 100% of internal IP ranges blocked
- **Credential Isolation:** Zero cross-instance leaks
- **Fuzzing Coverage:** 1M+ executions per target
