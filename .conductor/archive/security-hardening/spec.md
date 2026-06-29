# Track Specification: Security Hardening

## Overview
Implement comprehensive security hardening for the FYI Request System to protect sensitive official information request data, ensure privacy compliance, and meet security best practices for handling government communications.

## Current State
- **Database:** SQLite with no encryption at rest
- **Credentials:** Stored in environment variables or config files
- **Sessions:** Basic session handling without timeouts
- **Audit Logging:** Minimal or non-existent
- **Data Retention:** No formal policies or automated deletion

## Security Requirements

### 1. Encryption at Rest
**Priority:** CRITICAL

**Requirements:**
- [ ] SQLite database encryption using SQLCipher or similar
- [ ] Encryption key management (secure storage, rotation)
- [ ] Encrypted backups
- [ ] Secure key derivation from user password

**Acceptance Criteria:**
- ✅ Database file unreadable without decryption key
- ✅ All sensitive fields encrypted (requests, notes, credentials)
- ✅ Key derivation uses strong KDF (Argon2 or scrypt)
- ✅ Performance impact <10%

### 2. Secure Credential Storage
**Priority:** CRITICAL

**Requirements:**
- [ ] Integration with OS keyring (keyring library)
- [ ] No plaintext credentials in config files
- [ ] Support for multiple credential sets (multi-account)
- [ ] Secure credential deletion

**Acceptance Criteria:**
- ✅ Credentials stored in OS keyring (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- ✅ Config files contain only references, not actual credentials
- ✅ Credentials never logged or printed

### 3. Session Management
**Priority:** HIGH

**Requirements:**
- [ ] Session timeout (configurable, default 30 minutes)
- [ ] Session invalidation on logout
- [ ] Concurrent session limits
- [ ] Session binding to IP/user-agent (optional)

**Acceptance Criteria:**
- ✅ Sessions expire after inactivity
- ✅ Logout invalidates session immediately
- ✅ Session tokens are cryptographically secure

### 4. Audit Logging
**Priority:** HIGH

**Requirements:**
- [ ] Log all authentication events (login, logout, failed attempts)
- [ ] Log data access (request view, export, modification)
- [ ] Log security events (session timeout, privilege changes)
- [ ] Tamper-evident logs (append-only, cryptographic hashing)
- [ ] Log retention policies

**Acceptance Criteria:**
- ✅ All security-relevant events logged
- ✅ Logs include timestamp, user, action, result
- ✅ Logs protected from modification
- ✅ Log export for compliance review

### 5. Data Retention & Deletion
**Priority:** MEDIUM

**Requirements:**
- [ ] Configurable retention periods
- [ ] Automated deletion of expired data
- [ ] Secure deletion (cryptographic erasure)
- [ ] Retention policy documentation
- [ ] Export before deletion option

**Acceptance Criteria:**
- ✅ Data automatically deleted after retention period
- ✅ Deletion is irreversible (secure wipe)
- ✅ Users can configure retention periods
- ✅ Export functionality before deletion

### 6. Input Validation & Sanitization
**Priority:** HIGH

**Requirements:**
- [ ] SQL injection prevention (parameterized queries - already in place)
- [ ] XSS prevention in web UI (HTML escaping)
- [ ] CSRF protection for web forms
- [ ] Input validation for all user inputs
- [ ] File upload validation (if applicable)

**Acceptance Criteria:**
- ✅ All inputs validated and sanitized
- ✅ CSRF tokens on all forms
- ✅ Output properly escaped

### 7. Security Headers & HTTPS
**Priority:** MEDIUM

**Requirements:**
- [ ] HTTPS enforcement for web UI
- [ ] Security headers (CSP, HSTS, X-Frame-Options, etc.)
- [ ] Certificate validation for API calls
- [ ] TLS 1.3 for all network communications

**Acceptance Criteria:**
- ✅ Web UI requires HTTPS in production
- ✅ All security headers present
- ✅ No mixed content warnings

## Non-Functional Requirements

### Performance
- Encryption overhead: <10% performance impact
- Session validation: <5ms per request
- Audit logging: <10ms per event

### Compliance
- NZ Privacy Act 2020 compliance
- OWASP Top 10 mitigation
- GDPR data protection principles (if applicable)

### Usability
- Security features should not significantly impact UX
- Clear error messages for security failures
- Easy-to-understand privacy settings

## Out of Scope
- Multi-factor authentication (future enhancement)
- Hardware security module (HSM) integration
- Advanced threat detection
- Penetration testing (should be done separately)

## Dependencies
- **keyring** - OS keyring integration
- **cryptography** - Encryption primitives
- **passlib** - Password hashing and KDF
- **SQLCipher** (optional) - Database encryption

## Success Metrics
- ✅ All CRITICAL and HIGH requirements implemented
- ✅ Security audit passes with no critical findings
- ✅ OWASP ZAP scan shows no high/critical vulnerabilities
- ✅ Performance impact <10%
- ✅ All security tests passing

## Risks & Mitigations

### Risk: Encryption Key Loss
**Impact:** HIGH - Data permanently inaccessible  
**Mitigation:** Key backup procedure, key recovery documentation

### Risk: Performance Degradation
**Impact:** MEDIUM - User experience affected  
**Mitigation:** Benchmarking, optimization, optional encryption levels

### Risk: Keyring Compatibility Issues
**Impact:** MEDIUM - Some systems unsupported  
**Mitigation:** Fallback to encrypted file storage, clear error messages

## Timeline Estimate
- **Phase 1 (Encryption):** 3-4 days
- **Phase 2 (Credentials & Sessions):** 2-3 days
- **Phase 3 (Audit Logging):** 2-3 days
- **Phase 4 (Data Retention):** 1-2 days
- **Total:** 8-12 days

## Priority
**CRITICAL** - Security is fundamental for handling sensitive official information requests.
