# Development Tooling Guide

**Last Updated:** 2026-03-09  
**Version:** 1.0.0

---

## Overview

FYI CLI uses modern, optimized Python development tools for building, linting, type checking, security scanning, and profiling.

---

## Core Libraries

### CLI Framework: Typer

[**Typer**](https://typer.tiangolo.com/) - Build great CLIs. Fast to code. One less thing to worry about.

**Why Typer:**
- Type hints = CLI arguments (automatic validation)
- Auto-completion support (bash, zsh, fish, PowerShell)
- Beautiful help messages
- Based on Starlette/FastAPI patterns

**Usage:**
```python
import typer

app = typer.Typer()

@app.command()
def main(name: str = typer.Argument(..., help="Your name")):
    typer.echo(f"Hello {name}")
```

---

### Terminal Output: Rich

[**Rich**](https://rich.readthedocs.io/) - Write rich text and beautiful formatting in the terminal.

**Why Rich:**
- Tables, syntax highlighting, markdown
- Progress bars, spinners, status
- Tracebacks with syntax highlighting
- Emoji support 🎉

**Usage:**
```python
from rich.console import Console
from rich.table import Table

console = Console()
console.print("[bold green]Success![/bold green]")

table = Table(title="Requests")
table.add_column("ID", style="cyan")
table.add_column("Title", style="magenta")
console.print(table)
```

---

### HTTP Client: HTTPX

[**HTTPX**](https://www.python-httpx.org/) - A fully featured HTTP client for Python 3.

**Why HTTPX (over requests):**
- ✅ Async support
- ✅ HTTP/2 support
- ✅ Type hints
- ✅ Better error handling
- ✅ Modern API

**Usage:**
```python
import httpx

# Sync
with httpx.Client() as client:
    response = client.get('https://fyi.org.nz')

# Async
async with httpx.AsyncClient() as client:
    response = await client.get('https://fyi.org.nz')
```

---

### Data Validation: Pydantic

[**Pydantic**](https://docs.pydantic.dev/) - Data validation using Python type hints.

**Why Pydantic:**
- Fast (written in Rust)
- Type-safe
- JSON schema generation
- Settings management

**Usage:**
```python
from pydantic import BaseModel, HttpUrl

class Request(BaseModel):
    id: int
    title: str
    url: HttpUrl

request = Request(id=1, title="Test", url="https://fyi.org.nz")
print(request.model_dump())
```

---

### MCP Server: FastMCP

[**FastMCP**](https://github.com/jlowin/fastmcp) - Build MCP (Model Context Protocol) servers quickly.

**Why FastMCP:**
- Simple API
- Type-safe tools
- Resource management
- AI assistant integration

**Usage:**
```python
from fastmcp import FastMCP

mcp = FastMCP("FYI CLI")

@mcp.tool()
def search_requests(query: str) -> list:
    """Search FYI requests."""
    return [...]
```

---

### Package Manager: uv

[**uv**](https://github.com/astral-sh/uv) - An extremely fast Python package installer and resolver.

**Why uv:**
- ⚡ 10-100x faster than pip
- ✅ pip-compatible
- ✅ Virtualenv management
- ✅ Dependency resolution
- ✅ Written in Rust

**Usage:**
```bash
# Install dependencies
uv pip install -e ".[dev]"

# Create virtual environment
uv venv

# Run tool
uv run pytest
```

---

## Linting: Ruff (Strict Mode)

### What is Ruff?

[Ruff](https://docs.astral.sh/ruff/) is an extremely fast Python linter written in Rust.

**Replaces:**
- ✅ flake8 (and all plugins)
- ✅ pylint (many rules)
- ✅ isort (import sorting)
- ✅ pyupgrade (syntax upgrades)
- ✅ **bandit** (security rules) ✅ **Yes, ruff includes bandit!**
- ✅ perflint (performance rules)
- ✅ tryceratops (exception handling)
- ✅ And 30+ more...

**Why Ruff:**
- **Speed:** 10-100x faster than traditional linters
- **All-in-one:** Replaces 30+ linting tools
- **Security:** Includes all bandit security rules (rule code `S`)
- **Compatible:** Works with existing flake8 plugins
- **Auto-fix:** Automatically fixes many issues

### Configuration

Ruff is configured in `pyproject.toml` with **strict mode** enabled:

```toml
[tool.ruff.lint]
select = [
  "E",      # pycodestyle errors
  "W",      # pycodestyle warnings
  "F",      # Pyflakes
  "I",      # isort
  "S",      # flake8-bandit (security) ← Bandit is included!
  "B",      # flake8-bugbear
  # ... 80+ more rule sets
]
```

### Running Ruff

```bash
# Lint with auto-fix
ruff check src/ tests/ --fix

# Format code
ruff format src/ tests/

# Show statistics
ruff check src/ --statistics

# Security checks only
ruff check src/ --select=S
```

### Security Rules (Bandit Replacement)

**Yes, ruff has taken over bandit!** All bandit security rules are available via the `S` rule code:

```toml
"S",      # flake8-bandit (security)
```

Common security checks:
- `S101`: Assert used (remove in production)
- `S105`: Hardcoded password
- `S106`: Hardcoded API key
- `S113`: Request without timeout
- `S301`: Pickle usage (insecure)
- `S311`: Random usage (insecure for crypto)
- `S324`: Weak hash (MD5/SHA1)
- `S404`: Subprocess usage
- `S501`: Request with verify=False
- `S603`: Subprocess without shell=True
- `S605`: Shell injection (shell=True)
- `S701`: Jinja2 templates (XSS)

**Run security checks:**
```bash
ruff check src/ --select=S
```

---

## Type Checking: BasedPyright (Strict Mode)

### What is BasedPyright?

[**BasedPyright**](https://github.com/DetachHead/basedpyright) is a stricter fork of Pyright with additional checks.

**Why BasedPyright over Pyright/Mypy:**

| Feature | BasedPyright | Pyright | MyPy |
|---------|--------------|---------|------|
| **Speed** | ⚡ Very fast | Fast | Slower |
| **Strictness** | ✅✅ Maximum | ✅ Strict | ✅ Strict |
| **VS Code Integration** | ✅ Native (Pylance) | ✅ Native | Extension |
| **Extra Checks** | ✅ Many | Standard | Standard |
| **Unreachable Code** | ✅ Detects | ❌ | ❌ |
| **Implicit Concat** | ✅ Catches | ❌ | ❌ |

### Configuration

BasedPyright is configured in `pyproject.toml` with **strict mode**:

```toml
[tool.basedpyright]
pythonVersion = "3.10"
typeCheckingMode = "strict"
# ... (all pyright options)

# BasedPyright specific (stricter checks)
reportImplicitStringConcatenation = true
reportInconsistentConstructor = true
reportMissingSuperCall = true
reportUnreachable = true
```

### Running BasedPyright

```bash
# Type check
basedpyright --project pyproject.toml

# Show type coverage
basedpyright --project pyproject.toml --verifytypes fyi_system

# Watch mode
basedpyright --project pyproject.toml --watch
```

### Strict Mode Rules

BasedPyright strict mode checks:
- ✅ All function parameters must be typed
- ✅ All variables must be typed
- ✅ No `Any` types allowed (with exceptions)
- ✅ All return types must be specified
- ✅ Type narrowing must be correct
- ✅ Optional handling must be explicit
- ✅ Union types must be handled
- ✅ Unreachable code detection
- ✅ Implicit string concatenation
- ✅ Missing super() calls
- ✅ Inconsistent constructors

---

## Profiling: Scalene

### What is Scalene?

[Scalene](https://github.com/plasma-umass/scalene) is a high-performance CPU, GPU, and memory profiler for Python.

**Why Scalene:**
- **Fast:** Low overhead profiling (<5%)
- **Detailed:** Line-by-line profiling
- **Multi-metric:** CPU + GPU + Memory
- **AI-powered:** Suggests optimizations
- **Visual:** HTML reports with annotations

### Installation

```bash
pip install scalene
# or
uv pip install scalene
```

### Running Scalene

```bash
# Profile a script
scalene script.py

# Profile with all metrics
scalene --profile-all script.py

# Output to file
scalene --output profile.html script.py

# Profile tests
scalene tests/
```

### Example Output

```
                                    Memory usage: ▃ (max: 15.00MB, growth rate:  10%)
   fyi_system/cli.py: % of time =  15.00% out of   10.00s.
   Line #    Hits    Time %    Memory %    Copy %    Code
   45        100     25.0      10.0        5.0       def main():
   46        50      50.0      80.0        90.0          data = load_large_file()  # ← Hotspot!
   47        50      25.0      10.0        5.0          process(data)
```

### CI Integration

Scalene runs on every PR via `.github/workflows/profiling.yml`:

```yaml
- name: Run Scalene profiler
  run: |
    scalene --profile-all --output-profile profile.json tests/
```

---

## Dependency Management: Renovate

### What is Renovate?

[Renovate](https://docs.renovatebot.com/) - Automated dependency updates.

**Why Renovate:**
- ✅ Automated PRs for updates
- ✅ Security vulnerability alerts
- ✅ Grouped updates (linters, pytest, etc.)
- ✅ Schedule updates (weekly)
- ✅ Semantic commits
- ✅ Auto-merge for minor/patch

### Configuration

Configured in `renovate.json`:

```json
{
  "extends": [
    "config:recommended",
    ":automergeMinor",
    ":enableVulnerabilityAlerts"
  ],
  "packageRules": [
    {
      "matchPackageNames": ["ruff", "basedpyright"],
      "groupName": "linters"
    },
    {
      "matchPackageNames": ["pytest", "pytest-*"],
      "groupName": "pytest"
    }
  ],
  "schedule": ["before 6am on Monday"],
  "timezone": "Pacific/Auckland"
}
```

### How It Works

1. **Scans** dependencies daily
2. **Creates PRs** for updates
3. **Groups** related updates
4. **Auto-merges** minor/patch updates
5. **Alerts** on security vulnerabilities

---

## Release Automation: Release Please

### What is Release Please?

[Release Please](https://github.com/googleapis/release-please) - Automated releases based on conventional commits.

**Why Release Please:**
- ✅ Semantic versioning
- ✅ Automatic changelog generation
- ✅ PR-based releases
- ✅ Multi-language support
- ✅ GitHub-native

### Configuration

Configured in `.release-please-manifest.json`:

```json
{
  "packages": {
    ".": {
      "release-type": "python",
      "bump-minor-pre-major": true,
      "changelog-sections": [
        {"type": "feat", "section": "Features"},
        {"type": "fix", "section": "Bug Fixes"},
        {"type": "perf", "section": "Performance"}
      ]
    }
  }
}
```

### How It Works

1. **Monitors** commits on main branch
2. **Parses** conventional commits
3. **Creates** release PR with changelog
4. **On merge:** Creates GitHub release + tag
5. **Triggers** deployment workflow

### Commit Convention

```
feat: Add new feature
fix: Fix bug
perf: Improve performance
docs: Update documentation
refactor: Code refactoring
chore: Maintenance tasks
```

---

## Testing: pytest

### Configuration

```toml
[tool.pytest.ini_options]
pythonpath = ["src"]
testpaths = ["tests"]
addopts = "-v --tb=short"
```

### Running Tests

```bash
# Run all tests
pytest

# Run with coverage
pytest --cov=fyi_system --cov-report=html

# Run specific test
pytest tests/test_module.py::TestClass::test_method

# Run with profiling
pytest --profile-svg
```

---

## Security Scanning

### Dependency Scanning

We use **multiple** tools for comprehensive security:

1. **Ruff (`S` rules)** - Static analysis security checks
2. **pip-audit** - Known vulnerability scanning
3. **safety** - Additional vulnerability database

```bash
# Ruff security checks
ruff check src/ --select=S

# Dependency scanning
pip-audit -r pyproject.toml
safety check --full-report
```

### Why Multiple Tools?

- **Ruff:** Catches insecure code patterns
- **pip-audit:** Checks against PyPI vulnerability database
- **safety:** Additional vulnerability database (commercial + free)

**Defense in depth** - each tool catches different issues.

---

## Summary

| Task | Tool | Mode |
|------|------|------|
| **CLI Framework** | Typer | Type-safe CLI |
| **Terminal Output** | Rich | Beautiful formatting |
| **HTTP Client** | HTTPX | Async + HTTP/2 |
| **Data Validation** | Pydantic | Type-safe validation |
| **MCP Server** | FastMCP | AI integration |
| **Package Manager** | uv | 10-100x faster than pip |
| **Linting** | Ruff | Strict (80+ rule sets) |
| **Security** | Ruff (`S` rules) | All bandit rules included |
| **Type Checking** | BasedPyright | Strict mode (stricter than pyright) |
| **Profiling** | Scalene | CPU + GPU + Memory |
| **Testing** | pytest | With coverage |
| **Dependencies** | Renovate | Automated updates |
| **Releases** | Release Please | Automated versioning |

---

## Packaging assets (multi-registry)

Draft installers and MCP catalog packages live under `packaging/`.  
CI job **`packaging-assets`** (`.github/workflows/ci.yml`) verifies they exist and mention the current crate version — pure Python, no cargo.

```bash
# From repo root
python scripts/verify_packaging_assets.py
python scripts/verify_packaging_assets.py --json
python scripts/verify_packaging_assets.py --expected-version 0.1.2
```

Release orchestration (tags, GHCR, external catalogs, graceful failure):  
[`docs/release-multi-registry.md`](docs/release-multi-registry.md)  
Status matrix: [`docs/registry-distribution-matrix.md`](docs/registry-distribution-matrix.md)

---

## Quick Reference

```bash
# Install with uv
uv pip install -e ".[dev]"

# Full development workflow
ruff check src/ tests/ --fix      # Lint and fix
ruff format src/ tests/           # Format code
basedpyright --project pyproject.toml  # Type check
pytest --cov=fyi_system           # Test with coverage
scalene tests/                    # Profile performance
pip-audit -r pyproject.toml       # Check dependencies

# Packaging / multi-registry asset check (no cargo)
python scripts/verify_packaging_assets.py

# Build package
python -m build

# Run with uv
uv run fyi --help
```

---

## Resources

- [Typer Documentation](https://typer.tiangolo.com/)
- [Rich Documentation](https://rich.readthedocs.io/)
- [HTTPX Documentation](https://www.python-httpx.org/)
- [Pydantic Documentation](https://docs.pydantic.dev/)
- [FastMCP Documentation](https://github.com/jlowin/fastmcp)
- [uv Documentation](https://github.com/astral-sh/uv)
- [Ruff Documentation](https://docs.astral.sh/ruff/)
- [BasedPyright Documentation](https://github.com/DetachHead/basedpyright)
- [Scalene Documentation](https://github.com/plasma-umass/scalene)
- [Renovate Documentation](https://docs.renovatebot.com/)
- [Release Please Documentation](https://github.com/googleapis/release-please)

---

**Questions?** See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
