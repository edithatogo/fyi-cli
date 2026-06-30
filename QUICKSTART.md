# Quick Start Guide

**Get started with FYI Request System in 5 minutes!**

---

## Prerequisites

✅ FYI Request System installed (see [INSTALL.md](INSTALL.md))  
✅ FYI.org.nz account (free, sign up at https://fyi.org.nz)

---

## Step 1: Initialize Database (30 seconds)

```bash
fyi init-db
```

**Expected output:**
```
✓ Database initialized: fyi_system.db
✓ Ready to track requests
```

---

## Step 2: Configure API Key (1 minute)

### Option A: Get API Key Now (Recommended)

1. Go to https://fyi.org.nz/admin/api
2. Copy your API key
3. Store it securely:

```bash
fyi config set api-key YOUR_API_KEY_HERE
```

### Option B: Skip for Now

You can use the system without an API key. Manual submission only.

```bash
# Skip API setup for now
fyi config show
```

---

## Step 3: Import Authorities (1 minute)

```bash
# Import sample authorities
fyi import-authorities data/sample_authorities.csv
```

**Expected output:**
```
✓ Imported 50 authorities
```

**Verify:**
```bash
fyi list-authorities
```

---

## Step 4: Create Your First Request (2 minutes)

### Method 1: CLI (Fast)

```bash
# Create a new request
fyi register-request \
  ministry-of-justice \
  "Request for Departmental Spending Data" \
  "I request the following information under the Official Information Act 1982:

1. Total spending on consulting services for 2025
2. Breakdown by consulting firm
3. Contracts over $10,000

Please provide this information in electronic format." \
  --tags "spending" "official-information" \
  --status draft
```

**Expected output:**
```
✓ Request created: ID 1
✓ Status: draft
```

### Method 2: Web UI (Visual)

```bash
# Start web server
fyi serve
```

Then open http://127.0.0.1:8000 in your browser and click "New Request".

---

## Step 5: Generate Prefilled URL (30 seconds)

```bash
# Build prefilled URL for submission
fyi build-prefilled-url 1
```

**Expected output:**
```
https://fyi.org.nz/new/ministry-of-justice?title=Request+for...&body=...
```

**Open in browser:**
```bash
# Windows
start "https://fyi.org.nz/new/..."

# macOS
open "https://fyi.org.nz/new/..."

# Linux
xdg-open "https://fyi.org.nz/new/..."
```

---

## Step 6: Track Your Request (30 seconds)

```bash
# List all requests
fyi list-requests
```

**Expected output:**
```
ID  Authority            Title                          Status
1   ministry-of-justice  Request for Departmental...    draft
```

---

## 🎉 You're Done!

You've successfully:
- ✅ Initialized the database
- ✅ Configured API key (optional)
- ✅ Imported authorities
- ✅ Created your first request
- ✅ Generated submission URL
- ✅ Tracked your request

---

## What's Next?

### Learn More
- [User Guide](docs/USER_GUIDE.md) - Comprehensive features
- [CLI Reference](docs/CLI_REFERENCE.md) - All commands
- [API Setup](docs/API_KEY_SETUP.md) - Get your API key

### Common Next Steps

**Import more authorities:**
```bash
fyi import-authorities my-authorities.csv
```

**Generate dashboard:**
```bash
fyi dashboard --output dashboard.html
```

**Export all requests:**
```bash
fyi export-requests --output backup.json
```

**Get help:**
```bash
fyi --help
fyi <command> --help
```

---

## Quick Reference

| Task | Command |
|------|---------|
| Create request | `fyi register-request ...` |
| List requests | `fyi list-requests` |
| View request | `fyi request-detail <id>` |
| Update status | `fyi set-status <id> submitted` |
| Generate URL | `fyi build-prefilled-url <id>` |
| Export all | `fyi export-requests` |
| Dashboard | `fyi dashboard` |
| Help | `fyi --help` |

---

## Need Help?

- **Troubleshooting:** See [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- **FAQ:** See [FAQ.md](FAQ.md)
- **Issues:** https://github.com/edithatogo/fyi-cli/issues

---

**Happy requesting!** 🚀
