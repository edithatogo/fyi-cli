# Frequently Asked Questions (FAQ)

**Common questions about FYI Request System**

**Version:** 1.0.0  
**Last Updated:** 2026-03-09

---

## General Questions

### What is FYI Request System?

FYI Request System is a privacy-focused tool for managing Official Information Act (OIA) requests through FYI.org.nz. It helps journalists, researchers, and privacy advocates track, monitor, and analyze their official information requests.

### Is this official software from FYI.org.nz?

No. This is a third-party tool that works with FYI.org.nz's public API. It is not affiliated with or endorsed by FYI.org.nz or Transparency International New Zealand.

### Is it free to use?

Yes! FYI Request System is completely free and open-source software.

### What platforms does it support?

- **Windows:** 10 and 11
- **macOS:** 10.15 (Catalina) and later
- **Linux:** Ubuntu 20.04+, Debian 11+, Fedora 35+

### Do I need programming experience?

No. The system includes both a command-line interface (CLI) for advanced users and a web interface for those who prefer a visual interface.

---

## Installation

### How do I install FYI Request System?

See [INSTALL.md](INSTALL.md) for detailed instructions. Quick install:

```bash
pip install fyi-cli
```

### Do I need Python installed?

Yes, Python 3.10 or higher is required. If you don't have Python, download it from https://www.python.org/downloads/

### Can I use it without installing Python?

Yes! Standalone executables are available for Windows, macOS, and Linux. See [INSTALL.md](INSTALL.md#method-3-standalone-executable-no-python-required)

### How much disk space does it need?

- **Installation:** ~100 MB
- **Database:** Varies (typically 10-50 MB for active users)
- **Exports:** Varies (plan for 100-500 MB for backups)

### Can I install it on a network drive?

Yes, but performance may be slower. Local installation is recommended.

---

## API Key

### Do I need an API key?

**No, but it's recommended.** You can use the system without an API key for:
- Tracking requests locally
- Generating prefilled URLs
- Monitoring public feeds
- Generating reports

An API key is **required** for:
- Creating requests automatically
- Adding correspondence via API
- Bulk operations

See [API_KEY_SETUP.md](API_KEY_SETUP.md) for details.

### How do I get an API key?

API keys are available from FYI.org.nz:
1. Go to https://fyi.org.nz/admin/api
2. Log in with your authority account
3. Generate a new API key

If you don't have admin access, contact api@fyi.org.nz

### Is my API key secure?

Yes. API keys are stored:
- Encrypted in your local configuration
- Never sent to third parties
- Only used to communicate with FYI.org.nz

### Can I use the same API key on multiple computers?

Yes, but we recommend generating separate keys for each device for security.

### What if my API key is compromised?

1. Revoke the key in FYI admin panel
2. Generate a new key
3. Update your configuration:
   ```bash
   fyi-system config set api-key NEW_KEY
   ```

---

## Usage

### How do I create my first request?

See [QUICKSTART.md](QUICKSTART.md) for a 5-minute guide. Quick start:

```bash
fyi-system register-request \
  ministry-of-justice \
  "Request Title" \
  "Request body..." \
  --status draft
```

### Can I import existing requests from FYI.org.nz?

Yes! Use the feed monitoring feature:

```bash
fyi-system ingest-feed https://www.fyi.org.nz/request/latest.rss
```

### How do I track multiple requests?

All requests are stored in your local database. List them with:

```bash
fyi-system list-requests
```

### Can I export my data?

Yes! Multiple export formats:

```bash
# JSON export
fyi-system export-requests --output requests.json

# Single request bundle
fyi-system export-bundle 1 --output-dir bundle-1

# Dashboard (HTML)
fyi-system dashboard --output dashboard.html
```

### How do I backup my data?

```bash
# Export all requests
fyi-system export-requests --output backup-$(date +%Y%m%d).json

# Copy database file
cp fyi_system.db backup-$(date +%Y%m%d).db
```

---

## Privacy & Security

### Is my data stored locally or in the cloud?

**100% local.** All data is stored in a SQLite database on your computer. Nothing is sent to the cloud.

### Is my data encrypted?

Optional. Enable encryption during setup:

```bash
fyi-system setup
# Select "Enable encryption: Yes"
```

### Who can access my requests?

Only you. The database is stored locally with restricted file permissions.

### Does the system phone home?

No. The system only communicates with:
- FYI.org.nz (if you configure an API key)
- Your local database

No analytics, no telemetry, no tracking.

### Can I use it with TOR or a proxy?

Yes! Configure proxy in your environment:

```bash
export HTTP_PROXY="socks5://127.0.0.1:9050"
export HTTPS_PROXY="socks5://127.0.0.1:9050"
```

### How do I audit privacy settings?

```bash
fyi-system privacy-audit
```

---

## Technical Questions

### What database does it use?

SQLite - a lightweight, file-based database that requires no server.

### Can I access the database directly?

Yes! Use any SQLite client:

```bash
sqlite3 fyi_system.db

# List tables
.tables

# Query requests
SELECT * FROM tracked_requests;
```

### Can I integrate it with other tools?

Yes! The system provides:
- Command-line interface (CLI)
- Python API (`from fyi_system import ...`)
- JSON exports for integration

### How often should I run the scheduler?

Recommended: Every 1-4 hours

```bash
fyi-system scheduler https://www.fyi.org.nz/request/latest.rss --interval-seconds 3600
```

### Can I run multiple instances?

Yes, but use different database files:

```bash
fyi-system init-db --db /path/to/db1.db
fyi-system init-db --db /path/to/db2.db
```

### Does it work offline?

Partially. You can:
- ✅ View existing requests
- ✅ Generate reports
- ✅ Export data

But you cannot:
- ❌ Create new requests (requires FYI.org.nz)
- ❌ Monitor feeds (requires internet)

---

## Troubleshooting

### The command `fyi-system` is not found

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md#command-not-found-fyi-system)

### I get "Database locked" errors

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md#database-not-found-or-database-locked)

### API requests fail with "401 Unauthorized"

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md#invalid-api-key-or-401-unauthorized)

### The web UI won't load

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md#web-ui-issues)

### How do I enable debug mode?

```bash
fyi-system config set verbose true
fyi-system <command> --verbose
```

### Where are logs stored?

- **Windows:** `%USERPROFILE%\.fyi-system\logs\`
- **macOS/Linux:** `~/.fyi-system/logs/`

---

## Development

### Can I contribute to the project?

Yes! See [CONTRIBUTING.md](CONTRIBUTING.md) for how to contribute.

### Where is the source code?

https://github.com/yourusername/fyi-cli

### What license is it under?

MIT License - free to use, modify, and distribute.

### How do I report a bug?

1. Go to https://github.com/yourusername/fyi-cli/issues
2. Click "New Issue"
3. Fill in the template

Or email: support@fyi-cli.example.com

### Can I request a feature?

Yes! Use GitHub Issues or the discussion forum.

---

## Comparison

### How does this compare to FYI.org.nz?

| Feature | FYI.org.nz | FYI Request System |
|---------|------------|-------------------|
| **Request submission** | ✅ Web form | ✅ CLI + Web UI |
| **Request tracking** | ✅ Online | ✅ Local |
| **Privacy** | ⚠️ Cloud-based | ✅ Local-only |
| **Offline access** | ❌ No | ✅ Yes |
| **Bulk operations** | ⚠️ Limited | ✅ Yes |
| **Custom reports** | ❌ No | ✅ Yes |
| **Data export** | ⚠️ Limited | ✅ Multiple formats |

**Best used together:** Use FYI.org.nz for submission, FYI Request System for tracking and analysis.

---

## Support

### How do I get help?

1. **Documentation:** Start with [USER_GUIDE.md](USER_GUIDE.md)
2. **Troubleshooting:** See [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
3. **FAQ:** You're reading it!
4. **GitHub Issues:** https://github.com/yourusername/fyi-cli/issues
5. **Email:** support@fyi-cli.example.com

### Is there a user community?

Yes! Join the discussion forum:
https://github.com/yourusername/fyi-cli/discussions

### Do you offer training?

Self-paced training is available in the documentation. For organizational training, contact support.

### What if I find a security vulnerability?

See [SECURITY.md](SECURITY.md) for responsible disclosure process.

---

## Updates

### How do I update to a new version?

```bash
pip install --upgrade fyi-cli
```

### Will my data be preserved during updates?

Yes! Database migrations are automatic and backward-compatible.

### How do I know what changed?

See [CHANGELOG.md](CHANGELOG.md) for version history.

### How often are updates released?

- **Major releases:** Every 6 months
- **Minor releases:** Monthly
- **Bug fixes:** As needed

---

## Legal

### Is this legal to use?

Yes. The system uses FYI.org.nz's public API and complies with their terms of service.

### Does this work outside New Zealand?

Yes! The system works with any Alaveteli-based platform:
- FYI.org.nz (New Zealand)
- WhatDoTheyKnow.com (UK)
- FragDenStaat.de (Germany)
- And many others

### Can authorities use this?

Yes! Many authorities use it to track incoming requests.

### Do I need to disclose my use of this tool?

No. Your use of this tool is private.

---

**Still have questions?** Contact support or join the community forum!
