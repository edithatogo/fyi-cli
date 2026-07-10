# FYI CLI

**A multi-jurisdiction Freedom-of-Information (FOI) client for Alaveteli-based platforms** — track,
submit, and archive official information requests across New Zealand, Australia, the UK, and any
other [Alaveteli](https://alaveteli.org/) deployment (foi-repository, WhatDoTheyKnow, FragDenStaat, and
more), from a single privacy-focused CLI.

[![PyPI version](https://badge.fury.io/py/fyi-cli.svg)](https://badge.fury.io/py/fyi-cli)
[![Crates.io](https://img.shields.io/crates/v/fyi-cli.svg)](https://crates.io/crates/fyi-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/edithatogo/fyi-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/edithatogo/fyi-cli/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/edithatogo/fyi-cli/branch/master/graph/badge.svg)](https://codecov.io/gh/edithatogo/fyi-cli)
[![smithery badge](https://smithery.ai/badge/edithatogo/fyi-mcp)](https://smithery.ai/server/@edithatogo/fyi-mcp)
[![Glama](https://glama.ai/mcp/servers/edithatogo/fyi-cli/badges/score.svg)](https://glama.ai/mcp/servers/edithatogo/fyi-cli)
[![MCP Registry](https://img.shields.io/badge/MCP_Registry-io.github.edithatogo%2Ffyi--mcp-blue)](https://registry.modelcontextprotocol.io/)

The project's core is a **Rust workspace** (`fyi-core`, `fyi-cli`, `fyi-mcp`); a legacy Python
package (`fyi_system`) remains available as a reference implementation but is no longer extended.

### Where fyi-cli / fyi-mcp is listed

| Registry / catalog | Package / server | Status | Link |
|--------------------|------------------|--------|------|
| **Official MCP Registry** | `io.github.edithatogo/fyi-mcp` @ `0.1.2` | Live | [registry.modelcontextprotocol.io](https://registry.modelcontextprotocol.io/) · [`server.json`](server.json) |
| **Glama** | `edithatogo/fyi-cli` (`fyi-mcp`) | Live | [glama.ai/mcp/servers/edithatogo/fyi-cli](https://glama.ai/mcp/servers/edithatogo/fyi-cli) · [scores](https://glama.ai/mcp/servers/edithatogo/fyi-cli/score) |
| **Smithery** | `edithatogo/fyi-mcp` | Live (score pending) | [smithery.ai/server/@edithatogo/fyi-mcp](https://smithery.ai/server/@edithatogo/fyi-mcp) |
| **PyPI** | `fyi-cli` (legacy Python package) | Live | [pypi.org/project/fyi-cli](https://pypi.org/project/fyi-cli/) |
| **Crates.io** | `fyi-cli` / workspace crates | Published / release flow | [crates.io/crates/fyi-cli](https://crates.io/crates/fyi-cli) |
| **GitHub Releases** | CLI + MCPB assets | Live | [Releases](https://github.com/edithatogo/fyi-cli/releases) |
| **Awesome-MCP-Servers** | Legal section entry | PR submitted | [PR #9693](https://github.com/punkpeye/awesome-mcp-servers/pull/9693) |
| **GitHub curated MCP** (`github.com/mcp`) | — | Blocked external (manual onboarding) | Tracked in [#32](https://github.com/edithatogo/fyi-cli/issues/32) |

**Draft / not yet submitted:** Scoop, WinGet, Homebrew, Chocolatey, cargo-binstall metadata, GHCR multi-arch workflow, PulseMCP, mcp.so, Docker MCP Catalog, mcp-get, OpenTools, AUR, nixpkgs, Snap, Flatpak, asdf/mise, Debian/PPA, Fedora/COPR.

Full status matrix (including planned channels): [`docs/registry-distribution-matrix.md`](docs/registry-distribution-matrix.md).

---

## 🚀 Quick Start

```bash
# Install (Rust workspace, recommended)
cargo install --path crates/fyi-cli
# or: cargo install fyi-cli   (once published to crates.io)

# Initialize
fyi-cli init-db

# Create your first request
fyi-cli register-request ministry-of-justice "OIA Request" "Request body..." --status draft

# Track and manage requests
fyi-cli list-requests
```

**Full guide:** [QUICKSTART.md](QUICKSTART.md)

---

## ✨ Features

- **🌐 Multi-Jurisdiction**: Works with any Alaveteli deployment — foi-repository (NZ), WhatDoTheyKnow
  (UK), FragDenStaat (Germany), righttoknow.org.au (Australia), and more, with jurisdiction-aware
  metadata and localization on the way
- **🔒 Privacy-First**: All data stored locally, optional encryption, Tor support
- **📊 Track Requests**: Monitor FOI/OIA requests from creation to completion
- **🤖 Automated Monitoring**: Watch instances for updates automatically
- **📈 Reports & Analytics**: Generate dashboards, attention reports, handover docs
- **🔐 Secure Storage**: Encrypted credentials, OS keyring integration
- **💻 CLI + MCP**: Command-line tool and an [MCP server](crates/fyi-mcp) for AI assistants
- **📦 Export Options**: JSON, CSV, HTML export capabilities
- **🗄️ Faithful Archiving**: WARC/WACZ capture of requests, correspondence, and attachments

---

## 📦 Installation

**Full guide (Rust-first, all channels):** **[INSTALL.md](INSTALL.md)** · version **0.1.2** · homepage [github.com/edithatogo/fyi-cli](https://github.com/edithatogo/fyi-cli)

| Path | Command / location | Notes |
|------|--------------------|--------|
| **Cargo (clone)** | `cargo install --path crates/fyi-cli` | Recommended |
| **Cargo (crates.io)** | `cargo install fyi-cli` | When published |
| **cargo-binstall** | `cargo binstall fyi-cli` | **assets-ready** — needs matching release assets |
| **GitHub Releases** | [Releases](https://github.com/edithatogo/fyi-cli/releases) | Prebuilt CLI / MCPB; check asset names per tag |
| **PyPI (legacy)** | `pip install fyi-cli` | Python `fyi_system`; not extended |
| **MCP binary** | `cargo install --path crates/fyi-mcp` | Or MCPB from Releases |
| **MCP registries** | Official / Glama / Smithery | **live** — see table above |
| **Containers** | `ghcr.io/edithatogo/fyi-mcp` | **assets-ready** — [docs/containers.md](docs/containers.md) |
| **Scoop, WinGet, Homebrew, Chocolatey, AUR, nix, Snap, Flatpak, Debian, Fedora, asdf, mise** | Drafts under [`packaging/`](packaging/) | **Not claimed live** — [docs/installation-package-managers.md](docs/installation-package-managers.md) |

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli
cargo install --path crates/fyi-cli --locked
fyi-cli --help
```

Live listings and draft status: **[Where fyi-cli / fyi-mcp is listed](#where-fyi-cli--fyi-mcp-is-listed)** · full matrix: [`docs/registry-distribution-matrix.md`](docs/registry-distribution-matrix.md) · MCP catalog checklist: [`packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md`](packaging/mcp-catalogs/SUBMISSION_CHECKLIST.md).

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Docs site](https://edithatogo.github.io/fyi-cli/) | Full documentation (Astro + Starlight) |
| [docs/cli-entrypoints-audit.md](docs/cli-entrypoints-audit.md) | Canonical cross-reference for Python CLI, Rust CLI, and Rust MCP surfaces |
| [docs/upstream-relations.md](docs/upstream-relations.md) | Alaveteli/instance operator etiquette and outreach log |
| [docs/registry-distribution-matrix.md](docs/registry-distribution-matrix.md) | MCP catalogs, package managers, containers status |
| [docs/external-registry-followups.md](docs/external-registry-followups.md) | Smithery score + GitHub MCP onboarding playbook |
| [docs/installation-package-managers.md](docs/installation-package-managers.md) | Draft package-manager install notes (Scoop, WinGet, brew, …) |
| [docs/containers.md](docs/containers.md) | `fyi-mcp` container / GHCR |
| [QUICKSTART.md](QUICKSTART.md) | 5-minute getting started guide |
| [USER_GUIDE.md](USER_GUIDE.md) | Comprehensive user documentation |
| [INSTALL.md](INSTALL.md) | Installation guide (Rust-first; all channels) |
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

# Build & test the Rust workspace
cargo build --workspace --all-features
cargo test --workspace --all-features

# Make your changes, then submit a PR
git commit -m "feat: Add awesome feature"
git push origin feature/awesome-feature
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development setup, including the legacy
Python `fyi_system` package.

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

### Project Status

This project is actively evolving from an NZ-only tool into a multi-jurisdiction FOI/Alaveteli
platform built on the Rust workspace. Current work is tracked as
[Conductor](https://conductor.build) tracks under `.conductor/tracks.md`, mirrored to GitHub epic
issues (#37-#46) and their sub-issues. See the project board below for live status, and
[CHANGELOG.md](CHANGELOG.md) for released versions.

---

## 🔗 Compatible Platforms

FYI CLI works with any Alaveteli-based platform. The embedded jurisdiction catalog currently
includes:

| Platform | Region | URL |
|----------|--------|-----|
| **foi-repository** | New Zealand | foi-repository |
| **RightToKnow** | Australia | https://www.righttoknow.org.au |
| **WhatDoTheyKnow** | United Kingdom | https://www.whatdotheyknow.com |
| **MyRightToKnow** | Ireland | https://www.myrighttoknow.org |
| **Ma Da Da (CADA)** | France | https://www.madada.fr |
| **Tu Derecho a Saber** | Spain | https://www.tuderechoasaber.es |
| **FragDenStaat** | Germany | https://fragdenstaat.de |
| **Alaveteli** | Any | Self-hosted instances |

See `.conductor/tracks/jurisdiction-*` for the rollout plan covering additional instances and
non-English localization.

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

## Read-only authority discovery

`discover-bodies` keeps `--base-url` as the instance/capture URL. Use the
separate `--catalog-url` option only when the authority CSV is hosted at an
explicit alternate URL:

```bash
fyi discover-bodies --base-url https://www.righttoknow.org.au \
  --catalog-url https://catalog.example/au-authorities.csv --output bodies.json
```

The JSON output includes the effective catalog URL, whether the default or an
override was used, retrieval time, HTTP status, row count, and a SHA-256 of the
raw CSV payload. The override is read-only and still uses the catalog host's
robots policy, contactable User-Agent, retry backoff, and shared rate limiter.
Treat an override URL as trusted configuration: it can redirect discovery to a
different authority set and should be reviewed and recorded alongside archive
provenance.

Discovery uses a contactable User-Agent, checks `robots.txt`, and backs off on
transient `429`/`5xx` responses. Keep live runs polite: use small date windows,
resume with checkpoints, and coordinate archive work with the ethics guidance in
the sibling `fyi-archive` repo at `docs/ethics-and-compliance.md`.

The Rust `fyi-core` network path is the primary resource-aware implementation:
it parses `RateLimit-*`/`Retry-After`, adapts pacing, enforces request/bytes/time
guardrails, persists load memory, caches safe GETs, and can emit secret-redacted
JSONL traces. Python remains a deliberately compatible discovery/capture path
and follows the same identity and retry etiquette.

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
┌───────────────────────────────────────────────────────────────┐
│                         fyi-cli workspace                      │
├───────────────────┬───────────────────────┬───────────────────┤
│   fyi-cli (bin)   │   fyi-mcp (MCP server) │  legacy fyi_system │
│  CLI commands,     │  JSON-RPC over stdio,   │  (Python, reference │
│  TUI dashboard      │  tools for AI assistants│  implementation)   │
├───────────────────┴───────────────────────┴───────────────────┤
│                          fyi-core (lib)                         │
│   Alaveteli API client · Tor client · encryption/keyring ·       │
│   sync engine · jurisdiction/instance registry (in progress)     │
├─────────────────────────────────────────────────────────────────┤
│                        SQLite database                          │
│      (requests, correspondence, authorities, sync_metadata)     │
├─────────────────────────────────────────────────────────────────┤
│      Alaveteli instances: foi-repository · WhatDoTheyKnow ·      │
│      FragDenStaat · righttoknow.org.au · any self-hosted deploy  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Built on the [Alaveteli](https://alaveteli.org/) platform by mySociety
- Inspired by the transparency work of foi-repository
- Uses [cryptography](https://cryptography.io/) for encryption

---

## 📞 Support

- **Documentation:** https://edithatogo.github.io/fyi-cli/
- **Issues:** https://github.com/edithatogo/fyi-cli/issues
- **Discussions:** https://github.com/edithatogo/fyi-cli/discussions

---

**Made with ❤️ for transparency and privacy**
