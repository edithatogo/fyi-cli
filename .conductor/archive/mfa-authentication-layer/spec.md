# Specification: mfa-authentication-layer

## Overview
Implement Multi-Factor Authentication (MFA) via TOTP (Time-based One-Time Password) tokens inside the Rust security engine (`fyi-core::security`). This guards credential access and sensitive operations with a second authentication factor, following the RFC 6238 standard.

## Functional Requirements

### Phase 1: TOTP Core Implementation
1. **TOTP Secret Generation:** Cryptographically secure random secret generation (RFC 4226)
2. **TOTP Code Generation:** Generate 6-digit codes from secrets + current time (30-second window)
3. **TOTP Verification:** Verify provided codes with configurable clock drift tolerance (±1 window)
4. **QR Code Provisioning:** Generate provisioning URIs (`otpauth://`) for authenticator apps (Authy, Google Authenticator, etc.)

### Phase 2: Keyring Integration
1. **Secret Storage:** Store TOTP secrets securely via `KeyringStore` (OS keyring)
2. **Secret Retrieval:** Retrieve secrets for verification without exposing plaintext
3. **Multi-Key Support:** Support multiple MFA secrets per user/service account
4. **Secret Rotation:** Allow re-provisioning and deletion of MFA secrets

### Phase 3: Security Enforcement
1. **MFA Guard:** Require TOTP verification before credential decryption/use
2. **Session Tracking:** Track verified sessions with expiry
3. **Brute Force Protection:** Rate-limit TOTP verification attempts (max 5 attempts per 30s window)
4. **Audit Logging:** Log all MFA events (provisioning, verification, failures)

### Phase 4: CLI & TUI Integration
1. **CLI Commands:** `fyi mfa setup`, `fyi mfa verify`, `fyi mfa status`, `fyi mfa remove`
2. **TUI Integration:** MFA setup wizard in TUI, credential access guarded by MFA prompt
3. **MCP Tool Exposure:** Expose MFA tools via MCP server for AI-assisted management

## Non-Functional Requirements
- **Security:** Secrets zeroized after use; strict memory clearing
- **Compliance:** Follows RFC 6238 (TOTP) standard
- **Performance:** Code generation < 10ms, verification < 50ms
- **Usability:** Clear CLI output with QR code ASCII-art or URL for scanning

## Acceptance Criteria
- TOTP codes generated match authenticator app output
- Verification succeeds within correct window, fails outside it
- Secrets stored and retrieved from OS keyring
- MFA gate prevents credential access without valid code
- Brute force protection locks out after 5 failed attempts
- All tests pass with >90% coverage on new MFA code