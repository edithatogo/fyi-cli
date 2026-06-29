# Plan: mfa-authentication-layer

## Phase 1: TOTP Core Implementation

### Task 1.1: TOTP secret generation and code generation
- [x] Add TOTP dependencies (totp-rs or manual RFC 6238 implementation)
- [x] Implement `generate_totp_secret()` - cryptographically secure random secret
- [x] Implement `generate_totp_code(secret, time_step)` - 6-digit code generation
- [x] Implement `verify_totp_code(secret, code, drift)` - verification with clock drift
- [x] Write property-based tests with proptest
- [x] Commit: `feat(security): implement TOTP secret and code generation` [a05d40b]

### Task 1.2: Provisioning URI and QR support
- [x] Implement `build_provisioning_uri(issuer, account, secret)` -> `otpauth://` URI
- [x] Add QR code display in terminal (ASCII art or URL output)
- [x] Write unit tests for URI generation
- [x] Commit: `feat(security): add TOTP provisioning URI generation` [c4740c0]

## Phase 2: Keyring Integration

### Task 2.1: Secure secret storage
- [x] Add `store_totp_secret(username, secret)` to `KeyringStore`
- [x] Add `get_totp_secret(username)` to `KeyringStore`
- [x] Add `delete_totp_secret(username)` to `KeyringStore`
- [x] Add `list_totp_secrets()` to enumerate MFA-enabled accounts
- [x] Commit: `feat(security): integrate TOTP secret storage with OS keyring` [df846a9]

### Task 2.2: Multi-key and rotation support
- [~] Support multiple MFA secrets per account (versioned)
- [ ] Implement secret rotation (re-provision with new key)
- [ ] Write integration tests for keyring MFA operations
- [ ] Commit: `feat(security): add multi-key support and secret rotation`

## Phase 3: Security Enforcement

### Task 3.1: MFA guard
- [ ] Create `MfaGuard` struct that requires verification before credential access
- [ ] Integrate with `KeyringStore::get_credential()` to gate access
- [ ] Add verified session tracking with TOTP-based expiry
- [ ] Commit: `feat(security): add MFA guard for credential access`

### Task 3.2: Brute force protection and audit logging
- [ ] Implement rate-limiting (max 5 attempts per 30s window)
- [ ] Add MFA audit events to audit logging system
- [ ] Write tests for rate-limit enforcement
- [ ] Commit: `feat(security): add brute force protection and audit logging for MFA`

## Phase 4: CLI & TUI Integration

### Task 4.1: CLI commands
- [ ] Add `fyi mfa setup` command (generates secret, shows provisioning URI)
- [ ] Add `fyi mfa verify` command (tests code verification)
- [ ] Add `fyi mfa status` command (lists MFA status per account)
- [ ] Add `fyi mfa remove` command (deletes MFA from account)
- [ ] Commit: `feat(cli): add MFA management CLI commands`

### Task 4.2: TUI and MCP integration
- [ ] Add MFA setup wizard flow in TUI
- [ ] Guard credential access in TUI with MFA prompt
- [ ] Expose MFA tools in MCP server
- [ ] Commit: `feat(tui): add MFA integration to TUI and MCP server`

### Task 4.3: Conductor review
- [ ] Run conductor-review for mfa-authentication-layer track
- [ ] Apply any fix recommendations
- [ ] Push to GitHub
- [ ] Commit: `conductor(track): complete mfa-authentication-layer after review`

## Archive
- [ ] Archive track: move to archive/ directory
