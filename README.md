# FYI CLI

**Privacy-focused CLI tool for managing FYI.org.nz official information requests**

[![PyPI version](https://badge.fury.io/py/fyi-cli.svg)](https://badge.fury.io/py/fyi-cli)
[![Python Support](https://img.shields.io/pypi/pyversions/fyi-cli.svg)](https://pypi.org/project/fyi-cli/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/edithatogo/fyi-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/edithatogo/fyi-cli/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/edithatogo/fyi-cli/branch/master/graph/badge.svg)](https://codecov.io/gh/edithatogo/fyi-cli)
[![smithery badge](https://smithery.ai/badge/edithatogo/fyi-mcp)](https://smithery.ai/servers/edithatogo/fyi-mcp)

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

Download from [Releases](https://github.com/edithatogo/fyi-cli/releases):
- **Windows**: `fyi-cli-win.exe`
- **macOS**: `fyi-cli-macos`
- **Linux**: `fyi-cli-linux`

### From Source

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli
pip install -e ".[dev]"
```

**Full installation guide:** [INSTALL.md](INSTALL.md)

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [docs/cli-entrypoints-audit.md](docs/cli-entrypoints-audit.md) | Canonical cross-reference for Python CLI, Rust CLI, and Rust MCP surfaces |
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

Use GitHub's private vulnerability reporting:
https://github.com/edithatogo/fyi-cli/security/advisories/new

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
# Rust release checks used by this repository
cargo +stable-x86_64-pc-windows-gnu fmt --all -- --check
cargo +stable-x86_64-pc-windows-gnu clippy --workspace --all-targets --all-features -- -D warnings
cargo +stable-x86_64-pc-windows-gnu test --workspace --all-features

# Python legacy/support checks
.\.venv\Scripts\python.exe -m pytest tests/test_release_readiness.py

# Opt-in live smoke test
FYI_LIVE_SMOKE=1 .\.venv\Scripts\python.exe -m pytest -m smoke tests/test_discovery_smoke.py
```

**Test Coverage:** Rust workspace checks are the release gate. Python support tests remain available for legacy docs and archive tooling.

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

### GitHub Projects

The project board for this repository is
[fyi-cli Conductor Roadmap](https://github.com/users/edithatogo/projects/6).
It mirrors the local `.conductor/tracks.md` registry and tracks completed
Conductor work, release-readiness items, and external MCP registry follow-ups.

The umbrella
[Rare Insights on Open Policy from Aotearoa](https://github.com/users/edithatogo/projects/4)
project is synchronized at the item level. GitHub Projects does not support
nested projects, so synchronization is handled by
[`scripts/sync_github_projects.py`](scripts/sync_github_projects.py) and the
manual/scheduled [`project-sync`](.github/workflows/project-sync.yml) workflow.

```bash
# Preview synchronization without changing either project
.\.venv\Scripts\python.exe scripts\sync_github_projects.py --dry-run

# Apply synchronization from fyi-cli project 6 into RIOPA project 4
.\.venv\Scripts\python.exe scripts\sync_github_projects.py
```

The workflow requires a repository secret named `PROJECT_SYNC_TOKEN` with
GitHub Projects access for user-level ProjectsV2 writes. The sync is
conservative: it adds missing `fyi-cli` issue/PR items to RIOPA, copies shared
status values, sets the RIOPA mirror source to `other` unless a `fyi-cli` option
exists, and never deletes umbrella-board items.

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

For concurrent workers, point `discover` and `backfill` at the same SQLite
database with `--db`. That enables the shared cross-worker limiter, which
reserves one aggregate request slot across processes and records both normal
reservations and transient-failure backoff events. Inspect the current state
with `fyi rate-limit-status --db fyi_system.db`.

### CLI and MCP surfaces

The full command and server cross-reference is maintained in [docs/cli-entrypoints-audit.md](docs/cli-entrypoints-audit.md). It covers the Python CLI entrypoints (`fyi`, `fyi-cli`, `fyi-system`), the Rust CLI binary (`fyi-cli`), and the Rust MCP server (`fyi-mcp`) with its published registry pages.

Opt-in live smoke test:

```bash
FYI_LIVE_SMOKE=1 pytest -m smoke tests/test_discovery_smoke.py
```

### Faithful Archive Capture

`fyi capture` stores the public request JSON, rendered HTML, and attachments as
WARC records, deduplicates attachment bytes by SHA-256, and maintains a derived
request view for downstream dataset tooling.

```bash
fyi capture 12345 \
  --data-dir data \
  --dist-dir dist \
  --max-bytes 500000000 \
  --max-runtime-minutes 30
```

Capture layout:

```text
data/
  warc/<runid>-<request>.warc.gz
  attachments/<sha-prefix>/<sha256>
  raw/requests/<authority>/<request_id>/
    request.json
    page.html
    attachments.json
    snapshot_meta.json
dist/
  site_snapshots/<YYYYMMDD>.wacz
```

Each daily WACZ is appendable: subsequent captures add another WARC segment under
`archive/` and merge the resource metadata in `datapackage.json`. Replay tooling
that supports WACZ/WARC can open the package from `dist/site_snapshots/`; for
low-level inspection, unzip it and read the WARC segments with `warcio`.

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
- **Issues:** https://github.com/edithatogo/fyi-cli/issues
- **Discussions:** https://github.com/edithatogo/fyi-cli/discussions

---

**Made with ❤️ for transparency and privacy**
