# Security Configuration Guide

## Overview

This document provides comprehensive security configuration for the FYI Request System, covering encryption, authentication, session management, audit logging, and input validation.

---

## 1. Encryption Configuration

### 1.1 Initial Setup

```python
from fyi_system.encryption import setup_encryption

# Setup encryption with a strong password
# IMPORTANT: Store this password securely - it cannot be recovered!
setup_encryption(
    password="your-secure-master-password",
    app_name="fyi-cli"
)
```

### 1.2 Password Requirements

- **Minimum length:** 12 characters
- **Recommended:** 20+ characters
- **Include:** Uppercase, lowercase, numbers, symbols
- **Store in:** Password manager or secure location

### 1.3 Key Backup

```python
from fyi_system.encryption import export_key_backup, import_key_backup

# Export encrypted backup
export_key_backup(
    password="your-secure-master-password",
    output_path="secure-location/key-backup.enc"
)

# Import backup (e.g., after system restore)
import_key_backup(
    encrypted_key_path="secure-location/key-backup.enc",
    password="your-secure-master-password"
)
```

### 1.4 Key Rotation (Future)

```python
# TODO: Implement key rotation
# 1. Generate new master key
# 2. Re-encrypt all data with new key
# 3. Securely delete old key
# 4. Update backup
```

---

## 2. Credential Management

### 2.1 Storing FYI Credentials

```python
from fyi_system.credentials import save_fyi_credentials

# Store credentials securely
save_fyi_credentials(
    account_id="primary-account",
    email="journalist@example.com",
    api_token="your-fyi-api-token",
    base_url="https://fyi.org.nz",
    notes="Primary journalist account"
)
```

### 2.2 Retrieving Credentials

```python
from fyi_system.credentials import get_fyi_credentials

# Get credentials
creds = get_fyi_credentials("primary-account")
if creds:
    email = creds.email
    api_token = creds.api_token
    base_url = creds.base_url
```

### 2.3 Multi-Account Management

```python
from fyi_system.credentials import list_fyi_accounts, delete_fyi_credentials

# List all accounts
accounts = list_fyi_accounts()
for acc in accounts:
    print(f"Account: {acc['account_id']}")

# Delete specific account
delete_fyi_credentials("old-account")

# Delete all accounts (use with caution!)
from fyi_system.credentials import CredentialManager
manager = CredentialManager()
manager.delete_all_credentials()
```

---

## 3. Session Management

### 3.1 Configuration

```python
from fyi_system.sessions import SessionManager

# Create session manager with custom settings
session_manager = SessionManager(
    db_path='fyi_system.db',
    timeout_minutes=30,  # Session timeout
    max_concurrent_sessions=5  # Max sessions per user
)
```

### 3.2 Session Lifecycle

```python
# Create session on login
session = session_manager.create_session(
    user_id=user.id,
    ip_address=request.remote_addr,
    user_agent=request.headers.get('User-Agent')
)

# Validate session on each request
session = session_manager.validate_session(session_id)
if not session:
    # Session expired or invalid
    redirect('/login')

# Invalidate on logout
session_manager.invalidate_session(session_id)

# Logout all devices
session_manager.invalidate_all_user_sessions(user_id)
```

### 3.3 Session Cleanup

```python
# Clean up expired sessions (run periodically)
session_manager.cleanup_expired_sessions()

# Get session statistics
stats = session_manager.get_session_stats()
print(f"Active sessions: {stats['active_sessions']}")
print(f"Active users: {stats['active_users']}")
print(f"Expired sessions: {stats['expired_sessions']}")
```

---

## 4. Audit Logging

### 4.1 Configuration

```python
from fyi_system.audit import AuditLogger, get_audit_logger

# Create audit logger
audit_logger = AuditLogger(db_path='fyi_system.db')

# Or use default
audit_logger = get_audit_logger()
```

### 4.2 Logging Events

```python
# Authentication events
audit_logger.log_auth_success(user_id, ip_address="192.168.1.1")
audit_logger.log_auth_failure(user_id, reason="Invalid password")
audit_logger.log_logout(user_id)

# Data access events
audit_logger.log_data_access(
    user_id=user_id,
    resource_type="tracked_request",
    resource_id=request_id,
    action="view"
)

# Security events
audit_logger.log_permission_denied(
    user_id=user_id,
    resource_type="admin_panel",
    resource_id="settings"
)
```

### 4.3 Integrity Verification

```python
# Verify audit log integrity
result = audit_logger.verify_integrity()

if result["valid"]:
    print("✓ Audit log integrity verified")
else:
    print(f"✗ Integrity compromised: {result['broken_chains']}")
```

### 4.4 Export Audit Log

```python
import time

# Export last 24 hours
now = time.time()
audit_logger.export_events(
    output_path="audit_export.json",
    start_time=now - 86400,
    end_time=now
)
```

---

## 5. Data Retention

### 5.1 Default Retention Periods

| Resource Type | Retention Period | Delete Method |
|--------------|------------------|---------------|
| Sessions | 30 days | Simple |
| Feed Events | 180 days | Simple |
| Run Log | 365 days | Simple |
| Tracked Requests | 1095 days (3 years) | Secure |
| Audit Logs | 2555 days (7 years) | Secure + Export |

### 5.2 Cleanup Execution

```python
from fyi_system.retention import cleanup_expired, get_retention_manager

# Dry run - see what would be deleted
stats = cleanup_expired(dry_run=True)
print(f"Would delete: {stats['resources_deleted']}")

# Execute cleanup with exports
stats = cleanup_expired(
    export_dir='secure-exports/',
    dry_run=False
)
print(f"Deleted: {stats['resources_deleted']}")
print(f"Exported: {stats['exports_created']}")
```

### 5.3 Custom Retention Policies

```python
from fyi_system.retention import RetentionPolicy, get_retention_manager

manager = get_retention_manager()

# Create custom policy
custom_policy = RetentionPolicy(
    policy_name="temp-data",
    resource_type="temporary",
    retention_days=7,  # 1 week
    delete_method="secure",
    export_before_delete=False,
    enabled=True
)

manager.set_policy(custom_policy)
```

---

## 6. Input Validation & CSRF Protection

### 6.1 CSRF Token Generation

```python
from fyi_system.security_middleware import generate_csrf_token

# Generate token for form
token = generate_csrf_token()

# Include in HTML form
# <input type="hidden" name="csrf_token" value="{{ token }}">
```

### 6.2 Input Validation

```python
from fyi_system.security_middleware import InputValidator

# Validate email
if not InputValidator.validate_email(email):
    raise ValueError("Invalid email")

# Validate URL
if not InputValidator.validate_url(url):
    raise ValueError("Invalid URL")

# Sanitize HTML (XSS prevention)
safe_content = InputValidator.sanitize_html(user_content)

# Sanitize filename (path traversal prevention)
safe_filename = InputValidator.sanitize_filename(original_filename)
```

### 6.3 Security Headers

```python
from fyi_system.security_middleware import SecurityHeaders

# Apply all security headers to response
SecurityHeaders.apply_headers(handler)

# Headers applied:
# - Content-Security-Policy
# - X-Content-Type-Options: nosniff
# - X-Frame-Options: DENY
# - X-XSS-Protection
# - Referrer-Policy
# - Cache-Control: no-store
# - Strict-Transport-Security
```

---

## 7. Security Best Practices

### 7.1 Password Management

- ✅ Use a password manager (e.g., Bitwarden, 1Password)
- ✅ Enable multi-factor authentication where available
- ✅ Never reuse passwords across services
- ✅ Rotate passwords periodically (every 90 days)

### 7.2 Key Management

- ✅ Store encryption password in password manager
- ✅ Create encrypted key backup
- ✅ Store backup in separate secure location
- ✅ Test backup restoration periodically

### 7.3 Session Security

- ✅ Use HTTPS in production
- ✅ Set appropriate session timeout (30 minutes recommended)
- ✅ Limit concurrent sessions (5 recommended)
- ✅ Invalidate sessions on password change

### 7.4 Audit Compliance

- ✅ Review audit logs regularly
- ✅ Export logs for long-term storage
- ✅ Verify log integrity monthly
- ✅ Retain logs for compliance period (7 years recommended)

### 7.5 Data Retention

- ✅ Configure appropriate retention periods
- ✅ Export data before deletion
- ✅ Use secure deletion for sensitive data
- ✅ Document retention policies

---

## 8. Troubleshooting

### 8.1 Encryption Issues

**Problem:** "Encryption key not found"

**Solution:**
```python
from fyi_system.encryption import setup_encryption

# Re-run setup with correct password
setup_encryption(password="your-correct-password")
```

**Problem:** "Decryption failed"

**Solution:**
- Verify correct password is being used
- Check if key backup exists and restore if needed
- Verify keyring access (Windows Credential Manager)

### 8.2 Session Issues

**Problem:** Sessions expiring too quickly

**Solution:**
```python
# Increase timeout
session_manager = SessionManager(timeout_minutes=60)
```

**Problem:** Too many concurrent sessions

**Solution:**
```python
# Increase limit
session_manager = SessionManager(max_concurrent_sessions=10)
```

### 8.3 Audit Log Issues

**Problem:** Integrity verification failed

**Solution:**
- This indicates potential tampering
- Investigate immediately
- Check access logs
- Consider security incident response

### 8.4 CSRF Issues

**Problem:** "CSRF token missing or invalid"

**Solution:**
- Ensure token is included in form: `<input type="hidden" name="csrf_token">`
- Ensure token is included in header: `X-CSRF-Token`
- Regenerate token if session expired

---

## 9. Security Checklist

### Initial Setup
- [ ] Encryption configured with strong password
- [ ] Key backup created and stored securely
- [ ] FYI credentials stored in keyring
- [ ] Session timeout configured
- [ ] Audit logging enabled
- [ ] Retention policies configured
- [ ] Security headers configured

### Ongoing Maintenance
- [ ] Review audit logs weekly
- [ ] Verify audit log integrity monthly
- [ ] Clean up expired data monthly
- [ ] Review retention policies quarterly
- [ ] Test key backup restoration annually
- [ ] Update security configuration as needed

### Incident Response
- [ ] Document security incidents
- [ ] Preserve audit logs
- [ ] Rotate compromised credentials
- [ ] Review and update security measures

---

## 10. Compliance Notes

### NZ Privacy Act 2020

This system supports compliance with:
- **IPP 1:** Purpose of collection (audit logged)
- **IPP 5:** Storage and security (encrypted at rest)
- **IPP 9:** Retention (configurable retention policies)
- **IPP 10:** Limits on use (access logged and audited)

### Recommended Retention Periods

- **Active requests:** 3 years from closure
- **Audit logs:** 7 years (compliance)
- **Session data:** 30 days (security)
- **Feed events:** 180 days (operational)

---

**Document Version:** 1.0  
**Last Updated:** 2026-03-09  
**Next Review:** 2026-06-09
