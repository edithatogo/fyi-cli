# Project Context

This directory contains the canonical Conductor project context and management files.

---

## Product Definition

- [Product Definition](./product.md) - Vision, goals, target users, and key features
- [Product Guidelines](./product-guidelines.md) - Style, branding, UX principles, and ethics
- [Technology Stack](./tech-stack.md) - Python (active) and Rust (future) tech stacks

---

## Workflow

- [Workflow](./workflow.md) - Development workflow, TDD, quality gates, and commit guidelines
- [Code Style Guides](./code_styleguides/) - Python and Rust style guides

---

## Track Management

- [Tracks Registry](./tracks.md) - Registry of all project tracks
- [Tracks Directory](./tracks/) - Individual track specifications and plans

### Active Tracks
- **integrate-fyi-request-system-history** (COMPLETED) - Migration from 14 versioned archives

### Completed Tracks (from v14 integration)
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
    └── integrate-fyi-request-system-history/

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

### Development Commands (Python)
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
- **Dual Stack**: Python is the active implementation; Rust is a future direction
- **Migration Report**: See `handover/migration-report.md` for integration details
