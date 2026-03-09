# Plan: Security Hardening

## Phase 1: Encryption at Rest

### 1.1 Database Encryption Setup
- [ ] Task: Install cryptography library
- [ ] Task: Research SQLCipher vs application-layer encryption
- [ ] Task: Design encryption architecture
- [ ] Task: Create encryption utilities module

### 1.2 Field-Level Encryption
- [ ] Task: Encrypt tracked_requests table sensitive fields
- [ ] Task: Encrypt authorities table sensitive fields
- [ ] Task: Add encryption/decryption helpers to db.py
- [ ] Task: Test encryption doesn't break existing queries

### 1.3 Key Management
- [ ] Task: Implement key derivation (Argon2)
- [ ] Task: Create key storage mechanism
- [ ] Task: Add key rotation support
- [ ] Task: Document key backup procedure

### 1.4 Encrypted Backups
- [ ] Task: Create encrypted backup function
- [ ] Task: Test backup/restore cycle
- [ ] Task: Document backup procedure

## Phase 2: Secure Credential Storage

### 2.1 Keyring Integration
- [x] Task: Install keyring library
- [x] Task: Create credential storage abstraction
- [x] Task: Implement FYI API credential storage 6c138ee
- [x] Task: Test on Windows (target platform) 6c138ee

### 2.2 Multi-Account Credentials
- [x] Task: Support multiple credential sets 6c138ee
- [ ] Task: Add credential selection UI (CLI/web)
- [x] Task: Test credential switching 6c138ee

### 2.3 Secure Deletion
- [x] Task: Implement secure credential deletion 6c138ee
- [x] Task: Test credentials removed from keyring 6c138ee
- [x] Task: Clear credential caches 6c138ee

## Phase 3: Session Management

### 3.1 Session Timeout
- [x] Task: Add session timeout configuration 1899cba
- [x] Task: Implement inactivity tracking 1899cba
- [x] Task: Add timeout warning (optional)
- [x] Task: Test timeout behavior 1899cba

### 3.2 Session Invalidation
- [x] Task: Implement logout invalidation 1899cba
- [x] Task: Add session revocation API 1899cba
- [x] Task: Test concurrent session handling 1899cba

### 3.3 Secure Session Tokens
- [x] Task: Use secrets module for token generation 1899cba
- [x] Task: Add token expiration 1899cba
- [x] Task: Test token security 1899cba

## Phase 4: Audit Logging

### 4.1 Log Infrastructure
- [x] Task: Create audit log database table 969c30c
- [x] Task: Design log schema (timestamp, user, action, result, details) 969c30c
- [x] Task: Implement append-only log writer 969c30c
- [x] Task: Add log rotation

### 4.2 Event Logging
- [x] Task: Log authentication events (login, logout, failures) 969c30c
- [x] Task: Log data access (view, create, update, delete) 969c30c
- [x] Task: Log security events (timeout, errors) 969c30c
- [x] Task: Test log completeness 969c30c

### 4.3 Log Protection
- [x] Task: Implement tamper-evident logging (hash chaining) 969c30c
- [x] Task: Add log export function 969c30c
- [x] Task: Test log integrity verification 969c30c

### 4.4 Log Retention
- [ ] Task: Configure log retention period
- [ ] Task: Implement automatic log cleanup
- [ ] Task: Test retention enforcement

## Phase 5: Data Retention & Deletion

### 5.1 Retention Policies
- [x] Task: Design retention policy configuration df0fbe7
- [x] Task: Add retention settings to config df0fbe7
- [x] Task: Document retention recommendations df0fbe7

### 5.2 Automated Deletion
- [x] Task: Create scheduled deletion job df0fbe7
- [x] Task: Implement retention period checking df0fbe7
- [x] Task: Test automatic deletion df0fbe7

### 5.3 Secure Deletion
- [x] Task: Implement secure deletion (overwrite before delete) df0fbe7
- [x] Task: Test data cannot be recovered df0fbe7
- [x] Task: Document secure deletion process df0fbe7

### 5.4 Export Before Delete
- [x] Task: Add export option to deletion workflow df0fbe7
- [x] Task: Test export includes all related data df0fbe7
- [x] Task: Verify export is complete df0fbe7

## Phase 6: Input Validation & Security Headers

### 6.1 CSRF Protection
- [ ] Task: Implement CSRF token generation
- [ ] Task: Add CSRF validation to web forms
- [ ] Task: Test CSRF protection

### 6.2 Enhanced Input Validation
- [ ] Task: Review all user inputs
- [ ] Task: Add validation for request data
- [ ] Task: Add validation for search queries
- [ ] Test validation rejects malicious input

### 6.3 Security Headers
- [ ] Task: Add Content-Security-Policy header
- [ ] Task: Add Strict-Transport-Security header
- [ ] Task: Add X-Frame-Options header
- [ ] Task: Test headers with security scanner

## Phase 7: Testing & Verification

### 7.1 Security Tests
- [x] Task: Write tests for encryption/decryption
- [x] Task: Write tests for credential storage
- [x] Task: Write tests for session management
- [x] Task: Write tests for audit logging

### 7.2 Security Scanning
- [ ] Task: Run OWASP ZAP scan
- [ ] Task: Fix any high/critical findings
- [ ] Task: Document scan results

### 7.3 Performance Testing
- [x] Task: Benchmark encryption overhead
- [x] Task: Test session validation performance
- [x] Task: Verify <10% performance impact

### 7.4 Documentation
- [ ] Task: Write security configuration guide
- [ ] Task: Document key management procedures
- [ ] Task: Create security troubleshooting guide
- [ ] Task: Update SECURITY.md

## Phase 8: Code Review Fixes

### 8.1 Performance Optimization
- [x] Task: Remove per-operation key derivation eb300d2
- [x] Task: Use master key directly for encryption eb300d2

### 8.2 Security Enhancements
- [x] Task: Add password verification with stored hash eb300d2
- [x] Task: Add key backup/restore functionality eb300d2
- [x] Task: EncryptedField raises exception on missing key eb300d2

### 8.3 Test Coverage
- [x] Task: Add password verification test eb300d2
- [x] Task: Add key backup/restore test eb300d2

---

## Completion Criteria
- [ ] All CRITICAL requirements implemented
- [ ] All HIGH requirements implemented
- [ ] Security tests passing
- [ ] OWASP ZAP scan clean
- [ ] Performance impact <10%
- [ ] Documentation complete

## Track History
- **2026-03-09**: Track created from WORK_ANALYSIS.md recommendations
- **2026-03-09**: Phase 1 complete - encryption infrastructure added (5f08d88)
- **2026-03-09**: Code review fixes applied (eb300d2)
- **2026-03-09**: Phase 2 complete - secure credential storage (6c138ee)
- **2026-03-09**: Phase 3 complete - secure session management (1899cba)
- **2026-03-09**: Phase 4 complete - tamper-evident audit logging (969c30c)
- **2026-03-09**: Phase 5 complete - data retention & secure deletion (df0fbe7)
