---
title: Security Configuration Guide
description: Guide to configuring security, encryption, and credential storage.
---

## Overview

This guide provides configuration practices for encryption, authentication, session timeouts, audit logs, and input validation in the FYI Request System.

---

## 1. Encryption

### 1.1 Initial Setup

The database contents are encrypted at rest. Initialize database encryption by choosing a strong password:

```python
from fyi_system.encryption import setup_encryption

setup_encryption(
    password="your-secure-master-password",
    app_name="fyi-cli"
)
```

### 1.2 Backups

Securely export your encryption keys as encrypted blobs:

```python
from fyi_system.encryption import export_key_backup

export_key_backup(
    password="your-secure-master-password",
    output_path="key-backup.enc"
)
```

---

## 2. Keyring Integration

Credentials are kept safe in system keychains rather than environment variables or configs. Supported systems:
- Windows Credential Manager
- macOS Keychain
- Linux Secret Service (via dbus)

If no daemon is active on the host, the system falls back to a password-protected, AES-256-encrypted configuration file.
