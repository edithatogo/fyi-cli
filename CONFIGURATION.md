# Configuration Reference

**Complete configuration guide for FYI Request System**

**Version:** 1.0.0  
**Last Updated:** 2026-03-09

---

## Configuration Overview

FYI Request System can be configured via:

1. **Setup Wizard** (recommended for first-time setup)
2. **CLI Commands** (for scripting and automation)
3. **Configuration File** (for advanced users)
4. **Environment Variables** (for deployments)

---

## Configuration Options

### Core Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `database` | string | `fyi_system.db` | Path to SQLite database |
| `api_key` | string | `None` | FYI.org.nz API key |
| `base_url` | string | `https://fyi.org.nz` | FYI instance URL |
| `timeout` | integer | `30` | API request timeout (seconds) |
| `verbose` | boolean | `False` | Enable verbose output |

### Privacy Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `encrypt_data` | boolean | `False` | Encrypt sensitive data |
| `sanitize_exports` | boolean | `True` | Sanitize exports by default |
| `export_profile` | string | `standard` | Export profile (`standard` or `strict`) |
| `file_permissions` | string | `600` | Default file permissions |

### Scheduler Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `scheduler_interval` | integer | `3600` | Feed check interval (seconds) |
| `scheduler_enabled` | boolean | `False` | Auto-start scheduler |
| `feed_url` | string | `None` | Default feed URL |

### Web UI Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `web_host` | string | `127.0.0.1` | Web server bind address |
| `web_port` | integer | `8000` | Web server port |
| `web_debug` | boolean | `False` | Enable debug mode |

---

## Setup Wizard

**Run the interactive setup:**

```bash
fyi-system setup
```

**Wizard prompts:**

```
Welcome to FYI Request System Setup
====================================

? Database location [fyi_system.db]: 
  > fyi_system.db

? Do you have an FYI API key? 
  > Yes

? Enter your API key: 
  > [paste key here]

? Enable data encryption? 
  > No

? Enable verbose output? 
  > No

Configuration saved successfully!
✓ Database: fyi_system.db
✓ API Key: configured
✓ Encryption: disabled
```

---

## CLI Configuration Commands

### View Configuration

```bash
# Show all settings
fyi-system config show

# Show specific setting
fyi-system config show api_key
```

**Output:**
```
Configuration:
  database: fyi_system.db
  api_key: configured (sk...ed)
  base_url: https://fyi.org.nz
  timeout: 30
  verbose: false
```

### Set Configuration

```bash
# Set a value
fyi-system config set <key> <value>

# Examples
fyi-system config set database /path/to/db.db
fyi-system config set api-key YOUR_API_KEY
fyi-system config set base_url https://fyi.org.nz
fyi-system config set timeout 60
fyi-system config set verbose true
```

### Unset Configuration

```bash
# Remove a setting (reverts to default)
fyi-system config unset <key>

# Examples
fyi-system config unset api_key
fyi-system config unset verbose
```

### Reset Configuration

```bash
# Reset all settings to defaults
fyi-system config reset

# Confirm with --force
fyi-system config reset --force
```

---

## Configuration File

### Location

| OS | Path |
|----|------|
| **Windows** | `C:\Users\<user>\.fyi-system\config.json` |
| **macOS** | `~/.fyi-system/config.json` |
| **Linux** | `~/.fyi-system/config.json` |

### File Format

```json
{
  "database": "fyi_system.db",
  "api_key": "your-api-key-here",
  "base_url": "https://fyi.org.nz",
  "timeout": 30,
  "verbose": false,
  "encrypt_data": false,
  "sanitize_exports": true,
  "export_profile": "standard",
  "file_permissions": "600",
  "scheduler_interval": 3600,
  "scheduler_enabled": false,
  "web_host": "127.0.0.1",
  "web_port": 8000,
  "web_debug": false
}
```

### Manual Editing

**1. Open configuration file:**
```bash
# Windows
notepad %USERPROFILE%\.fyi-system\config.json

# macOS/Linux
nano ~/.fyi-system/config.json
```

**2. Edit values:**
```json
{
  "database": "/custom/path/fyi_system.db",
  "timeout": 60
}
```

**3. Save and close**

**4. Verify:**
```bash
fyi-system config show
```

---

## Environment Variables

### Available Variables

| Variable | Setting | Example |
|----------|---------|---------|
| `FYI_DATABASE` | `database` | `/path/to/db.db` |
| `FYI_API_KEY` | `api_key` | `your-api-key` |
| `FYI_BASE_URL` | `base_url` | `https://fyi.org.nz` |
| `FYI_TIMEOUT` | `timeout` | `60` |
| `FYI_VERBOSE` | `verbose` | `true` |
| `FYI_ENCRYPT` | `encrypt_data` | `false` |
| `FYI_WEB_PORT` | `web_port` | `8080` |

### Setting Environment Variables

**Temporary (current session):**

```bash
# Windows (PowerShell)
$env:FYI_API_KEY="your-key"

# macOS/Linux
export FYI_API_KEY="your-key"
```

**Permanent:**

```bash
# Add to shell profile
echo 'export FYI_API_KEY="your-key"' >> ~/.bashrc
source ~/.bashrc

# Or add to .env file
echo 'FYI_API_KEY=your-key' >> ~/.fyi-system/.env
```

### Precedence

Configuration is loaded in this order (last wins):

1. Default values
2. Configuration file
3. Environment variables
4. CLI flags

**Example:**
```bash
# Config file sets: timeout=30
# Environment sets: FYI_TIMEOUT=60
# CLI flag: --timeout=90

fyi-system health-check --timeout=90
# Result: timeout=90 (CLI flag wins)
```

---

## Profiles

**Use different configurations for different scenarios:**

### Create Profile

```bash
# Create work profile
fyi-system config set api-key WORK_KEY --profile work

# Create personal profile
fyi-system config set api-key PERSONAL_KEY --profile personal
```

### Switch Profile

```bash
# Switch to work profile
fyi-system config set active-profile work

# Verify
fyi-system config show
```

### List Profiles

```bash
fyi-system config list-profiles
```

**Output:**
```
Available profiles:
  * default
    work
    personal
```

### Delete Profile

```bash
fyi-system config delete-profile work
```

---

## Validation

### Validate Configuration

```bash
# Check configuration is valid
fyi-system config validate
```

**Output:**
```
✓ Configuration valid
✓ Database path: OK
✓ API key: configured
✓ All checks passed
```

### Test Connection

```bash
# Test API connection
fyi-system health-check
```

**Output:**
```
✓ FYI.org.nz: Connected
✓ API Key: Valid
✓ Database: OK
✓ All systems operational
```

---

## Backup & Restore

### Backup Configuration

```bash
# Copy configuration file
cp ~/.fyi-system/config.json ~/.fyi-system/config.json.backup

# Or export all settings
fyi-system config export > config-backup.json
```

### Restore Configuration

```bash
# Restore from backup file
cp ~/.fyi-system/config.json.backup ~/.fyi-system/config.json

# Or import settings
fyi-system config import config-backup.json
```

---

## Troubleshooting

### Configuration Not Loading

**Check file permissions:**
```bash
# Should be readable by user only
ls -la ~/.fyi-system/config.json
# Expected: -rw------- (600)

# Fix if needed
chmod 600 ~/.fyi-system/config.json
```

### Invalid JSON

**Validate JSON:**
```bash
# macOS
cat ~/.fyi-system/config.json | python -m json.tool

# Or use jq
jq . ~/.fyi-system/config.json
```

### Settings Not Persisting

**Check you're not using environment variables that override:**
```bash
# Check environment
echo $FYI_API_KEY

# If set, it will override config file
unset FYI_API_KEY
```

---

## Examples

### Example 1: Production Configuration

```json
{
  "database": "/var/lib/fyi-system/fyi_system.db",
  "api_key": "prod-key-here",
  "timeout": 60,
  "encrypt_data": true,
  "sanitize_exports": true,
  "export_profile": "strict",
  "file_permissions": "600",
  "web_host": "127.0.0.1",
  "web_port": 8000
}
```

### Example 2: Development Configuration

```json
{
  "database": "./dev.db",
  "verbose": true,
  "web_debug": true,
  "scheduler_enabled": false
}
```

### Example 3: Testing Configuration

```json
{
  "database": ":memory:",
  "api_key": "test-key",
  "base_url": "https://test.fyi.org.nz",
  "timeout": 10
}
```

---

## See Also

- [INSTALL.md](INSTALL.md) - Installation guide
- [API_KEY_SETUP.md](API_KEY_SETUP.md) - API key configuration
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Troubleshooting guide

---

**Need help?** See [FAQ.md](FAQ.md) or open an issue on GitHub.
