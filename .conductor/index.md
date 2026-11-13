# Project Context

This directory contains the canonical Conductor project context and management files.

---

## Product Definition

- [Product Definition](./product.md) - Vision, goals, target users, and key features
- [Product Guidelines](./product-guidelines.md) - Style, branding, UX principles, and ethics
- [Technology Stack](./tech-stack.md) - Rust (primary) and Python (legacy) tech stacks

---

## Workflow

- [Workflow](./workflow.md) - Development workflow, TDD, quality gates, and commit guidelines
- [Code Style Guides](./code_styleguides/) - Python and Rust style guides

---

## Track Management

- [Tracks Registry](./tracks.md) - Registry of all project tracks
- [Tracks Directory](./tracks/) - Individual track specifications and plans

### Active Tracks
- **frontier-rust-convergence-20261113** — Frontier capability contract, Rust convergence, 3.15/1.99 compatibility gates, CI/harness and Conductor self-repair (planning; not yet implemented)
- **quality-frontier-testing-cicd-20261113** — Quality frontier: testing and CI/CD gaps (DST, MT, contract; pinned actions, dependency review, workflow lint) for issue #280 (completed via PR #414)
- **solo-maintainer-security-20261113** — Quality frontier: security and solo-maintainer context (additive ruleset, AGENTS.md, one-command validation + drift receipt, Renovate preset inheritance, Codecov OIDC verification) for issue #281
- **source-adapter-boundary-20260812** - Exclusive receipt-producing network boundary for Alaveteli, Archive, Wayback and approved indexes
- **ai-plugin-registry-submissions_20260712** - Prepare, submit, verify and monitor fyi-mcp in AI plugin and MCP registries
- **package-registry-submissions_20260712** - Reproducible package, container and developer distribution submissions
- **foi-platform-landscape_20260712** - Source-backed map of non-Alaveteli FOI platforms, with Phase 3 drift monitoring still open
- **jurisdiction-capture-completion-20260721** - Bounded jurisdiction capture increments; live runs remain operator-authorised

### Completed Tracks
- **python-security-quality-fuzzing-20260813** - Hypothesis gate, bounded Atheris fuzzing, and zero-vulnerability Python security baseline (PR #390)
- **remote-mcp-observability-security-20260711** - Default-deny policy, observability, and operator controls
- **remote-mcp-read-surface-20260711** - Guarded read-only remote SyncClient tools
- **remote-mcp-contract-harness-20260711** - Compatibility contracts and aggressive layered harnesses
- **remote-mcp-write-governance-20260711** - Governed prepare/commit remote writes
- **resource-aware-autonomous-agent** - Header-aware adaptive pacing, load memory, plan reflection, identity hygiene, filesystem cache, and local traces for sustainable Alaveteli access
- **research-grade-quality** - >95% test coverage, >90% mutation score, and load/performance testing baseline
- **integrate-fyi-cli-history** - Migration from 14 versioned archives
- **webapp-coverage-95** - Test coverage expansion for the web interface
- **rust-core-migration** - Rust rewrite Phase 1 (Contracts, DB, Cryptography, Tor)
- **rust-mcp-tui-implementation** - Rust rewrite Phase 2 (CLI, MCP daemon, Ratatui TUI)
- **rust-quality-hardening** - Rust rewrite Phase 3 (Property tests, E2E tests, dhat profiling)
- **rust-cicd-publishing** - Rust rewrite Phase 4 (CI/CD workflows, cargo-dist cross-compilation)
- **fyi-phase-1** through **fyi-phase-14** - Original project implementation phases


---

## Project Structure

```
.conductor/                 # This directory (canonical Conductor structure)
├── product.md             # Product definition
├── product-guidelines.md  # Product guidelines
├── tech-stack.md          # Technology stack (Python + Rust)
├── workflow.md            # Development workflow
├── index.md               # This file
├── tracks.md              # Tracks registry
├── code_styleguides/      # Code style guides
│   ├── python.md
│   └── rust.md
└── tracks/                # Individual tracks
    ├── fyi-phase-1/ through fyi-phase-14/
    └── integrate-fyi-cli-history/

archive/                    # Archived source materials
└── upstream-zips/         # Original 14 zip files

src/fyi_system/            # Python implementation
tests/                     # Test suite
skills/                    # Gemini CLI skills
prompts/                   # Prompt templates
handover/                  # Documentation
data/                      # Sample data
scripts/                   # Utility scripts
```

---

## Commands

### Conductor Commands
- `/conductor:setup` - Run initial project setup
- `/conductor:newTrack <name>` - Create a new track
- `/conductor:implement <track-id>` - Implement a track
- `/conductor:status` - Check project status

### Development Commands (Rust primary)
```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pwsh -NoProfile -File scripts/Invoke-MsvcPortable.ps1 cargo test --target x86_64-pc-windows-msvc -p fyi-core
```

### Development Commands (Python legacy)
```bash
# Install dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Run tests with coverage
pytest --cov=fyi_system --cov-report=html

# Lint
ruff check .

# Format
ruff format .

# Type check
mypy src/
```

---

## Notes

- **Canonical Structure**: `.conductor/` is the single source of truth for Conductor files
- **Integration Complete**: The project has been integrated from 14 versioned archives (v14 canonical)
- **Dual Stack**: Rust is the primary implementation; Python remains a supported legacy and compatibility surface
- **Migration Report**: See `handover/migration-report.md` for integration details
