# Plan: multi-jurisdiction-security-hardening

## Phase 1: SSRF Prevention & URL Validation

### 1.1 URL Validation Framework
- [ ] Task: Create URL validation module
- [ ] Task: Implement scheme allowlist (https:// only)
- [ ] Task: Validate host against internal IP ranges (RFC 1918, RFC 4193, localhost)
- [ ] Task: Block loopback (127.0.0.0/8, ::1)
- [ ] Task: Block link-local (169.254.0.0/16, fe80::/10)
- [ ] Task: Write tests for SSRF prevention
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: URL Validation' (Protocol in workflow.md)

### 1.2 DNS Rebinding Prevention
- [ ] Task: Implement DNS lookup validation
- [ ] Task: Re-validate resolved IPs before connection
- [ ] Task: Test DNS rebinding attack scenarios
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: DNS Rebinding' (Protocol in workflow.md)

### 1.3 Instance URL Auditing
- [ ] Task: Audit all instance base_url uses
- [ ] Task: Apply validation to user-supplied URLs
- [ ] Task: Test with malicious URL attempts
- [ ] Task: Document SSRF mitigations
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: Auditing' (Protocol in workflow.md)

## Phase 2: Credential Isolation

### 2.1 Keyring Namespace Enforcement
- [ ] Task: Audit keyring access patterns
- [ ] Task: Enforce instance_id namespacing
- [ ] Task: Test cross-instance key access attempts
- [ ] Task: Write tests for credential boundaries
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Keyring' (Protocol in workflow.md)

### 2.2 Credential Leakage Testing
- [ ] Task: Create credential leak detection tests
- [ ] Task: Test credential access from wrong instance
- [ ] Task: Verify no credential exposure in logs
- [ ] Task: Test concurrent multi-instance credential access
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Leak Testing' (Protocol in workflow.md)

## Phase 3: GDPR/PII Compliance (EU Instances)

### 3.1 Data Minimization
- [ ] Task: Audit data collection for EU instances
- [ ] Task: Remove unnecessary PII storage
- [ ] Task: Implement minimal retention policies
- [ ] Task: Test data minimization enforcement
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Minimization' (Protocol in workflow.md)

### 3.2 Right to Erasure
- [ ] Task: Implement data deletion for EU users
- [ ] Task: Add "forget me" functionality
- [ ] Task: Test complete data removal
- [ ] Task: Verify no residual PII after erasure
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: Erasure' (Protocol in workflow.md)

### 3.3 Privacy Impact Assessment
- [ ] Task: Conduct PIA for multi-jurisdiction platform
- [ ] Task: Document data flows per instance
- [ ] Task: Identify privacy risks
- [ ] Task: Implement risk mitigations
- [ ] Task: Create GDPR compliance checklist
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: PIA' (Protocol in workflow.md)

## Phase 4: Supply Chain Security

### 4.1 Dependency Scanning
- [ ] Task: Integrate `cargo-deny` in CI
- [ ] Task: Configure license and advisory checks
- [ ] Task: Add `cargo-audit` to CI pipeline
- [ ] Task: Set up automated vulnerability alerts
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Scanning' (Protocol in workflow.md)

### 4.2 SBOM Generation
- [ ] Task: Integrate SBOM generation (CycloneDX format)
- [ ] Task: Generate SBOM in release workflow
- [ ] Task: Include dependencies and licenses
- [ ] Task: Publish SBOM with releases
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: SBOM' (Protocol in workflow.md)

### 4.3 Artifact Signing
- [ ] Task: Set up sigstore/cosign for artifact signing
- [ ] Task: Sign release binaries with cosign
- [ ] Task: Generate SLSA provenance
- [ ] Task: Document signature verification process
- [ ] Task: Test signature verification
- [ ] Task: Conductor - User Manual Verification 'Phase 4.3: Signing' (Protocol in workflow.md)

## Phase 5: Anonymity & Tor Hardening

### 5.1 Per-Instance Circuit Isolation
- [ ] Task: Implement Tor circuit isolation per instance
- [ ] Task: Prevent circuit sharing across instances
- [ ] Task: Test circuit isolation
- [ ] Task: Verify no circuit correlation
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: Circuits' (Protocol in workflow.md)

### 5.2 Leak Detection
- [ ] Task: Create leak canary tests
- [ ] Task: Detect non-Tor traffic leaks
- [ ] Task: Test DNS leak prevention
- [ ] Task: Verify all traffic routes through Tor
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: Leaks' (Protocol in workflow.md)

### 5.3 Anonymity Testing
- [ ] Task: Test IP address anonymization
- [ ] Task: Verify headers don't leak identity
- [ ] Task: Test with multiple instances concurrently
- [ ] Task: Document anonymity guarantees
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: Testing' (Protocol in workflow.md)

## Phase 6: Fuzzing Infrastructure

### 6.1 Cargo-Fuzz Integration
- [ ] Task: Add `cargo-fuzz` to dev dependencies
- [ ] Task: Create fuzz targets for API parsers
- [ ] Task: Fuzz Alaveteli JSON response parser
- [ ] Task: Fuzz Atom feed parser
- [ ] Task: Fuzz RSS feed parser
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Fuzz Setup' (Protocol in workflow.md)

### 6.2 HTTP Response Fuzzing
- [ ] Task: Create fuzz target for HTTP responses
- [ ] Task: Fuzz error response handling
- [ ] Task: Fuzz malformed data handling
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: HTTP Fuzzing' (Protocol in workflow.md)

### 6.3 Continuous Fuzzing
- [ ] Task: Integrate fuzzing in CI
- [ ] Task: Run fuzzing on every PR
- [ ] Task: Set coverage goals (1M+ executions)
- [ ] Task: Document fuzzing findings and fixes
- [ ] Task: Conductor - User Manual Verification 'Phase 6.3: CI Fuzzing' (Protocol in workflow.md)

## Phase 7: Threat Modeling & Documentation

### 7.1 Threat Model
- [ ] Task: Document threat model for multi-jurisdiction platform
- [ ] Task: Identify attack surfaces (SSRF, credential leak, Tor bypass, etc.)
- [ ] Task: Assess risk for each threat
- [ ] Task: Document mitigations
- [ ] Task: Conductor - User Manual Verification 'Phase 7.1: Threat Model' (Protocol in workflow.md)

### 7.2 Security Documentation
- [ ] Task: Document security architecture
- [ ] Task: Create security best practices guide
- [ ] Task: Document SSRF prevention strategy
- [ ] Task: Document anonymity guarantees
- [ ] Task: Create incident response plan template
- [ ] Task: Conductor - User Manual Verification 'Phase 7.2: Security Docs' (Protocol in workflow.md)

### 7.3 Penetration Test Checklist
- [ ] Task: Create external pentest checklist
- [ ] Task: Document test scenarios for future audits
- [ ] Task: List known security boundaries
- [ ] Task: Conductor - User Manual Verification 'Phase 7.3: Pentest Checklist' (Protocol in workflow.md)

## Phase 8: Integration & Verification

### 8.1 Security Test Suite
- [ ] Task: Create comprehensive security test suite
- [ ] Task: Test all SSRF prevention scenarios
- [ ] Task: Test credential isolation boundaries
- [ ] Task: Test Tor anonymity features
- [ ] Task: Run all security tests in CI
- [ ] Task: Conductor - User Manual Verification 'Phase 8.1: Test Suite' (Protocol in workflow.md)

### 8.2 Security Audit
- [ ] Task: Internal security audit of codebase
- [ ] Task: Review all credential handling
- [ ] Task: Review all network operations
- [ ] Task: Review all user input handling
- [ ] Task: Document audit findings
- [ ] Task: Conductor - User Manual Verification 'Phase 8.2: Audit' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] Zero high-severity vulnerabilities
- [ ] SSRF prevention verified
- [ ] Credential isolation tested
- [ ] GDPR compliance implemented
- [ ] Supply chain security gates in CI
- [ ] Tor anonymity hardened
- [ ] Fuzzing infrastructure operational
- [ ] Threat model documented
- [ ] All security tests passing

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
