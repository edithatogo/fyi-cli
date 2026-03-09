# FYI Request System - User Guide

**Complete guide to using FYI Request System**

**Version:** 1.0.0  
**Last Updated:** 2026-03-09

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Managing Requests](#managing-requests)
4. [Working with Authorities](#working-with-authorities)
5. [Feed Monitoring](#feed-monitoring)
6. [Reporting & Analytics](#reporting--analytics)
7. [Export & Backup](#export--backup)
8. [Web Interface](#web-interface)
9. [Advanced Features](#advanced-features)
10. [Best Practices](#best-practices)

---

## Introduction

### What is FYI Request System?

FYI Request System is a privacy-focused tool for managing Official Information Act (OIA) requests through FYI.org.nz. It helps you:

- **Track** requests from creation to completion
- **Monitor** FYI.org.nz for new requests and responses
- **Generate** reports and dashboards
- **Export** request data for analysis
- **Maintain privacy** with local-first architecture

### Key Features

| Feature | Description |
|---------|-------------|
| **Local Database** | All data stored locally in SQLite |
| **Privacy First** | Optional TOR/proxy support, no cloud sync |
| **CLI + Web UI** | Command-line and web interface options |
| **Automated Monitoring** | Watch FYI.org.nz for updates |
| **Reporting** | Generate attention reports, handover docs |
| **Export** | JSON, CSV, HTML, PDF export options |
| **Security** | Encrypted storage, secure credential management |

### System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   FYI Request System                     │
├─────────────────────────────────────────────────────────┤
│  CLI Commands  │  Web UI  │  Scheduler  │  Reports     │
├─────────────────────────────────────────────────────────┤
│                   SQLite Database                        │
│  (tracked_requests, authorities, feed_events, etc.)     │
├─────────────────────────────────────────────────────────┤
│                   FYI.org.nz API                         │
│              (Read + Write API support)                  │
└─────────────────────────────────────────────────────────┘
```

---

## Getting Started

### Installation

See [INSTALL.md](INSTALL.md) for detailed installation instructions.

**Quick install:**
```bash
pip install fyi-request-system
```

### Initial Setup

**1. Initialize database:**
```bash
fyi-system init-db
```

**2. Run setup wizard:**
```bash
fyi-system setup
```

**3. Import authorities:**
```bash
fyi-system import-authorities data/sample_authorities.csv
```

### Configuration

**View current settings:**
```bash
fyi-system config show
```

**Set API key:**
```bash
fyi-system config set api-key YOUR_API_KEY
```

**Change database location:**
```bash
fyi-system config set database /path/to/database.db
```

---

## Managing Requests

### Creating Requests

#### Method 1: CLI

```bash
fyi-system register-request \
  <authority-slug> \
  "Request Title" \
  "Request body text..." \
  --tags "tag1" "tag2" \
  --status draft
```

**Example:**
```bash
fyi-system register-request \
  ministry-of-justice \
  "Request for Legal Advice Spending" \
  "I request information about spending on legal advice..." \
  --tags "spending" "legal" \
  --status draft
```

#### Method 2: Web UI

```bash
fyi-system serve
# Open http://127.0.0.1:8000/requests/new
```

#### Method 3: Prefilled URL

```bash
# Generate URL for manual submission
fyi-system build-prefilled-url <id>

# Or build from scratch
fyi-system build-prefilled-url \
  ministry-of-justice \
  "Title" \
  "Body text..." \
  --tags "tag1" "tag2"
```

### Viewing Requests

**List all requests:**
```bash
fyi-system list-requests
```

**Filter by status:**
```bash
fyi-system list-requests --status submitted
```

**Search:**
```bash
fyi-system list-requests --search "spending"
```

**View details:**
```bash
fyi-system request-detail <id>
```

**View timeline:**
```bash
fyi-system request-timeline <id>
```

### Updating Requests

**Update status:**
```bash
fyi-system set-status <id> <status>

# Statuses: draft, submitted, waiting_response, successful, rejected
```

**Edit request:**
```bash
# Via web UI
fyi-system serve
# Navigate to request and click "Edit"
```

### Deleting Requests

```bash
# Mark as deleted (soft delete)
fyi-system set-status <id> deleted

# Permanently delete (use with caution!)
# Note: No built-in permanent delete - edit database directly if needed
```

---

## Working with Authorities

### Importing Authorities

**From CSV:**
```bash
fyi-system import-authorities authorities.csv
```

**CSV format:**
```csv
slug,name,url
ministry-of-justice,Ministry of Justice,https://fyi.org.nz/body/ministry-of-justice
treasury,The Treasury,https://fyi.org.nz/body/treasury
```

### Listing Authorities

**All authorities:**
```bash
fyi-system list-authorities
```

**Search authorities:**
```bash
fyi-system list-authorities --search "ministry"
```

### Adding Authorities Manually

**Via web UI:**
```bash
fyi-system serve
# Navigate to /authorities and click "Add"
```

**Via database (advanced):**
```sql
INSERT INTO authorities (slug, name, url)
VALUES ('new-authority', 'New Authority Name', 'https://fyi.org.nz/body/...');
```

---

## Feed Monitoring

### Manual Feed Ingestion

```bash
fyi-system ingest-feed https://www.fyi.org.nz/request/latest.rss
```

### Automated Monitoring

**Run scheduler (continuous):**
```bash
fyi-system scheduler https://www.fyi.org.nz/request/latest.rss
```

**Run once:**
```bash
fyi-system run-cycle https://www.fyi.org.nz/request/latest.rss
```

**With custom interval:**
```bash
fyi-system scheduler \
  https://www.fyi.org.nz/request/latest.rss \
  --interval-seconds 3600  # Check every hour
```

### Reconciling Feed Events

```bash
# Match feed events to tracked requests
fyi-system reconcile-events
```

---

## Reporting & Analytics

### Attention Report

**Generate JSON report:**
```bash
fyi-system attention-report --output attention.json
```

**View report:**
```bash
cat attention.json | jq .
```

### Dashboard

**Generate HTML dashboard:**
```bash
fyi-system dashboard --output dashboard.html
```

**Generate JSON data:**
```bash
fyi-system dashboard --json-output dashboard.json
```

**Open in browser:**
```bash
# Windows
start dashboard.html

# macOS
open dashboard.html

# Linux
xdg-open dashboard.html
```

### Handover Document

**Generate markdown handover:**
```bash
fyi-system handover --output handover.md
```

### Triage Report

**Generate triage report:**
```bash
fyi-system triage-report --output triage.json
```

**View as JSON:**
```bash
fyi-system triage-report
```

### Next Best Action

**Get recommendation for specific request:**
```bash
fyi-system next-best-action <id>
```

**With tone:**
```bash
fyi-system next-best-action <id> --tone formal
```

---

## Export & Backup

### Export All Requests

**JSON export:**
```bash
fyi-system export-requests --output requests.json
```

**Import back:**
```bash
fyi-system import-requests requests.json
```

### Export Single Request

```bash
fyi-system export-request <id> --output request-<id>.json
```

### Export Bundle

**Complete request bundle:**
```bash
fyi-system export-bundle <id> --output-dir bundle-<id>
```

**With sanitization:**
```bash
fyi-system export-bundle <id> --output-dir bundle-<id>
# Sanitization is on by default
```

**Without sanitization:**
```bash
fyi-system export-bundle <id> --output-dir bundle-<id> --no-sanitize
```

### Backup Strategy

**Recommended backup schedule:**
```bash
# Daily: Export all requests
fyi-system export-requests --output backups/requests-$(date +%Y%m%d).json

# Weekly: Export bundles for active requests
fyi-system export-bundle <id> --output-dir backups/week-$(date +%Y%m%d)

# Monthly: Full database backup
cp fyi_system.db backups/fyi-system-$(date +%Y%m).db
```

---

## Web Interface

### Starting the Server

```bash
fyi-system serve
# or with custom port
fyi-system serve --port 8080
```

### Web UI Features

| Page | URL | Description |
|------|-----|-------------|
| Dashboard | `/` | Overview and statistics |
| Requests | `/requests` | List and search requests |
| New Request | `/requests/new` | Create new request |
| Request Detail | `/requests/<id>` | View/edit request |
| Authorities | `/authorities` | List authorities |
| Import | `/authorities/import` | Import CSV |

### Security

**Default binding:** 127.0.0.1 (localhost only)

**Custom binding:**
```bash
fyi-system serve --host 0.0.0.0  # Accessible from network
```

**Security headers:** Automatically applied (CSP, HSTS, etc.)

---

## Advanced Features

### Privacy Audit

```bash
fyi-system privacy-audit --output privacy-audit.json
```

**Checks:**
- Database file permissions
- Web server binding
- Export sanitization
- Credential storage

### Correspondence Pack

```bash
fyi-system correspondence-pack <id> --output pack.md
```

**Includes:**
- Request timeline
- Recommended actions
- Draft correspondence
- Strategy options

### Follow-up Drafts

```bash
fyi-system follow-up-draft <id>
```

**With variants:**
```bash
fyi-system follow-up-variants <id>
```

### Attachment Manifest

```bash
# JSON manifest
fyi-system attachment-manifest <id>

# CSV manifest
fyi-system attachment-manifest-csv <id>
```

### Show Settings

```bash
fyi-system show-settings
```

---

## Best Practices

### Request Management

1. **Use descriptive titles** - Makes searching easier
2. **Tag consistently** - Use tags like `spending`, `contracts`, `2026`
3. **Update status promptly** - Keep track of request state
4. **Export regularly** - Backup before major changes

### Security

1. **Use strong API key storage** - Use `fyi-system config set`
2. **Enable encryption** - Run `fyi-system setup` for encryption
3. **Regular privacy audits** - Run `fyi-system privacy-audit` monthly
4. **Sanitize exports** - Use `--profile strict` for sensitive data

### Performance

1. **Limit feed polling** - Check every 1-4 hours, not every minute
2. **Archive old requests** - Export and remove completed requests
3. **Use search** - Don't list all requests if you have many
4. **Run scheduler separately** - Don't run in same terminal as UI

### Workflow

1. **Morning:** Check attention report
2. **Weekly:** Run triage report
3. **Monthly:** Export all requests
4. **Quarterly:** Privacy audit

---

## Troubleshooting

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues.

---

## Getting Help

- **Quick Start:** [QUICKSTART.md](QUICKSTART.md)
- **Installation:** [INSTALL.md](INSTALL.md)
- **FAQ:** [FAQ.md](FAQ.md)
- **Issues:** https://github.com/yourusername/fyi-request-system/issues

---

**Happy requesting!** 🚀
