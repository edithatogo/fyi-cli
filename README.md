# FYI CLI

**Privacy-focused CLI tool for managing FYI.org.nz official information requests**

[![PyPI version](https://badge.fury.io/py/fyi-cli.svg)](https://badge.fury.io/py/fyi-cli)
[![Python Support](https://img.shields.io/pypi/pyversions/fyi-cli.svg)](https://pypi.org/project/fyi-cli/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/yourusername/fyi-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/yourusername/fyi-cli/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/yourusername/fyi-cli/branch/main/graph/badge.svg)](https://codecov.io/gh/yourusername/fyi-cli)

---

## 🚀 Quick Start

```bash
# Install
pip install fyi-cli

# Initialize
fyi init-db

# Create your first request
fyi register-request ministry-of-justice "OIA Request" "Request body..." --status draft

# Generate submission URL
fyi build-prefilled-url 1

# Track and manage requests
fyi list-requests
fyi dashboard --output dashboard.html
```

**Full guide:** [QUICKSTART.md](QUICKSTART.md)

---

## ✨ Features

- **🔒 Privacy-First**: All data stored locally, optional encryption
- **📊 Track Requests**: Monitor OIA requests from creation to completion
- **🤖 Automated Monitoring**: Watch FYI.org.nz for updates automatically
- **📈 Reports & Analytics**: Generate dashboards, attention reports, handover docs
- **🔐 Secure Storage**: Encrypted credentials, OS keyring integration
- **🌐 Alaveteli Compatible**: Works with any Alaveteli instance (FYI, WDTK, FDS)
- **💻 CLI + Web UI**: Command-line and web interface options
- **📦 Export Options**: JSON, CSV, HTML, PDF export capabilities

---

## 📦 Installation

### From PyPI (Recommended)

```bash
pip install fyi-cli
```

### Standalone Executables

Download from [Releases](https://github.com/yourusername/fyi-cli/releases):
- **Windows**: `fyi-cli-win.exe`
- **macOS**: `fyi-cli-macos`
- **Linux**: `fyi-cli-linux`

### From Source

```bash
git clone https://github.com/yourusername/fyi-cli.git
cd fyi-cli
pip install -e ".[dev]"
```

**Full installation guide:** [INSTALL.md](INSTALL.md)

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [QUICKSTART.md](QUICKSTART.md) | 5-minute getting started guide |
| [USER_GUIDE.md](USER_GUIDE.md) | Comprehensive user documentation |
| [INSTALL.md](INSTALL.md) | Installation guide (Windows/Mac/Linux) |
| [API_KEY_SETUP.md](API_KEY_SETUP.md) | How to get and configure API key |
| [CONFIGURATION.md](CONFIGURATION.md) | Configuration reference |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Troubleshooting guide |
| [FAQ.md](FAQ.md) | Frequently asked questions |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to contribute |

---

## 🛡️ Security

### Reporting a Vulnerability

**Please do NOT report security vulnerabilities through public GitHub issues.**

Report security issues to: **security@fyi-cli.example.com**

Or use GitHub's private vulnerability reporting:  
https://github.com/yourusername/fyi-cli/security/advisories/new

**Security Policy:** [SECURITY.md](.github/SECURITY.md)

### Security Features

- ✅ AES-256-GCM encryption for sensitive data
- ✅ PBKDF2-HMAC-SHA256 key derivation
- ✅ OS keyring integration for credential storage
- ✅ Tamper-evident audit logging
- ✅ Secure session management
- ✅ Input validation and sanitization
- ✅ Security headers (CSP, HSTS, X-Frame-Options)
- ✅ Automated security scanning (CodeQL, pip-audit, bandit)

---

## 🧪 Testing

```bash
# Run all tests
pytest

# Run with coverage
pytest --cov=fyi_system --cov-report=html

# Run specific test file
pytest tests/test_alaveteli_client.py
```

**Test Coverage:** >80% (target)

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Start for Contributors

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/fyi-cli.git
cd fyi-cli

# Set up development environment
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows
pip install -e ".[dev]"

# Run tests
pytest

# Make your changes, then submit a PR
git commit -m "feat: Add awesome feature"
git push origin feature/awesome-feature
```

### Code of Conduct

Please note that this project is released with a [Contributor Code of Conduct](CODE_OF_CONDUCT.md). By participating in this project you agree to abide by its terms.

---

## 📊 Project Status

### v1.0.0 Release Progress

| Phase | Status | Progress |
|-------|--------|----------|
| **Phase 1.1: Documentation** | ✅ Complete | 100% |
| **Phase 1.2: Packaging** | ⏳ In Progress | 0% |
| **Phase 1.3: UX Improvements** | ⏳ Pending | 0% |
| **Phase 2: Beta Release** | ⏳ Pending | 0% |
| **Phase 3: Public Release** | ⏳ Pending | 0% |

**Target Release Date:** 2026-03-30

**Release Plan:** [RELEASE_PLAN.md](RELEASE_PLAN.md)

---

## 🔗 Compatible Platforms

FYI CLI works with any Alaveteli-based platform:

| Platform | Region | URL |
|----------|--------|-----|
| **FYI.org.nz** | New Zealand | https://fyi.org.nz |
| **WhatDoTheyKnow** | United Kingdom | https://www.whatdotheyknow.com |
| **FragDenStaat** | Germany | https://fragdenstaat.de |
| **Alaveteli** | Any | Self-hosted instances |

---

## 📋 CLI Commands

### Archive Discovery

The archive commands are read-only and do not require an API key.

```bash
# Import the official FYI authority list into the local SQLite database
fyi import-authorities

# Walk the public search feed for a date window and save discovered requests
fyi discover --date-from 2024-01-01 --date-to 2024-02-01 \
  --checkpoint data/_state/discovery-2024-01.json \
  --output data/_state/discovered-2024-01.jsonl

# Probe a numeric request ID range for gaps
fyi discover --backfill-ids --id-from 1 --id-to 5000 \
  --output data/_state/backfill-1-5000.jsonl

# Compare feed discovery against the ID backfill
fyi discover-reconcile \
  --feed data/_state/discovered-2024-01.jsonl \
  --backfill data/_state/backfill-1-5000.jsonl \
  --output data/_state/discovery-reconciliation.json
```

Discovery uses a contactable User-Agent, checks `robots.txt`, and backs off on
transient `429`/`5xx` responses. Keep live runs polite: use small date windows,
resume with checkpoints, and coordinate archive work with the ethics guidance in
the sibling `fyi-archive` repo at `docs/ethics-and-compliance.md`.

Opt-in live smoke test:

```bash
FYI_LIVE_SMOKE=1 pytest -m smoke tests/test_discovery_smoke.py
```

```bash
# Database
fyi init-db                    # Initialize database
fyi config show                # Show configuration

# Requests
fyi register-request ...       # Create new request
fyi list-requests              # List all requests
fyi request-detail <id>        # View request details
fyi set-status <id> <status>   # Update status

# Submission
fyi build-prefilled-url <id>   # Generate submission URL

# Monitoring
fyi ingest-feed <url>          # Ingest RSS/Atom feed
fyi scheduler <url>            # Run continuous monitoring
fyi discover                   # Discover public FYI requests
fyi discover-reconcile         # Compare discovery JSONL outputs

# Reports
fyi dashboard --output ...     # Generate dashboard
fyi attention-report           # Generate attention report
fyi handover --output ...      # Generate handover document

# Export
fyi export-requests            # Export all requests
fyi export-bundle <id>         # Export request bundle

# Security
fyi privacy-audit              # Privacy compliance check
fyi health-check               # System health verification
```

**Full CLI reference:** See `fyi --help` or [USER_GUIDE.md](USER_GUIDE.md)

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      FYI CLI                             │
├─────────────────────────────────────────────────────────┤
│  CLI Commands  │  Web UI  │  Scheduler  │  Reports     │
├─────────────────────────────────────────────────────────┤
│              Alaveteli API Client                        │
│         (Read API + Write API support)                  │
├─────────────────────────────────────────────────────────┤
│                   SQLite Database                        │
│  (tracked_requests, authorities, feed_events, etc.)     │
├─────────────────────────────────────────────────────────┤
│              FYI.org.nz / Alaveteli API                 │
└─────────────────────────────────────────────────────────┘
```

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Built on the [Alaveteli](https://alaveteli.org/) platform by mySociety
- Inspired by the transparency work of [FYI.org.nz](https://fyi.org.nz/)
- Uses [cryptography](https://cryptography.io/) for encryption

---

## 📞 Support

- **Documentation:** https://fyi-cli.readthedocs.io/
- **Issues:** https://github.com/yourusername/fyi-cli/issues
- **Discussions:** https://github.com/yourusername/fyi-cli/discussions
- **Email:** support@fyi-cli.example.com

---

**Made with ❤️ for transparency and privacy**
