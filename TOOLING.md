# Development Tooling Guide

**Last Updated:** 2026-03-09  
**Version:** 1.0.0

---

## Overview

FYI CLI uses modern, optimized Python development tools for linting, type checking, security scanning, and profiling.

---

## Linting: Ruff (Strict Mode)

### What is Ruff?

[Ruff](https://docs.astral.sh/ruff/) is an extremely fast Python linter written in Rust. It's a drop-in replacement for multiple tools:

- **flake8** (and all plugins)
- **pylint** (many rules)
- **isort** (import sorting)
- **pyupgrade** (syntax upgrades)
- **bandit** (security rules) ✅ **Yes, ruff includes bandit!**
- **perflint** (performance rules)
- **tryceratops** (exception handling)
- And many more...

### Why Ruff?

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
  # ... 60+ more rule sets
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

# Show violations by rule
ruff check src/ --output-format=concise
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
- `S107`: Hardcoded TLS password
- `S110`: Try-except-pass (silent failure)
- `S112`: Try-except-continue (silent failure)
- `S113`: Request without timeout
- `S301`: Pickle usage (insecure)
- `S311`: Random usage (insecure for crypto)
- `S324`: Weak hash (MD5/SHA1)
- `S404`: Subprocess usage
- `S501`: Request with verify=False
- `S601`: Paramiko shell injection
- `S603`: Subprocess without shell=True
- `S605`: Shell injection (shell=True)
- `S607`: Partial executable path
- `S701`: Jinja2 templates (XSS)

**Run security checks:**
```bash
ruff check src/ --select=S
```

---

## Type Checking: Pyright (Strict Mode)

### What is Pyright?

[Pyright](https://github.com/microsoft/pyright) is a fast type checker from Microsoft. It's the engine behind VS Code's Pylance extension.

### Why Pyright over MyPy?

| Feature | Pyright | MyPy |
|---------|---------|------|
| **Speed** | ⚡ Very fast (2-5x faster) | Slower |
| **VS Code Integration** | ✅ Native (Pylance) | Requires extension |
| **Strict Mode** | ✅ Excellent | Good |
| **Error Messages** | ✅ Clear, actionable | Sometimes cryptic |
| **Incremental** | ✅ Yes | Limited |
| **Configuration** | ✅ pyproject.toml | Separate config |

### Configuration

Pyright is configured in `pyproject.toml` with **strict mode**:

```toml
[tool.pyright]
pythonVersion = "3.10"
typeCheckingMode = "strict"
include = ["src"]
reportMissingTypeStubs = false
reportUnknownVariableType = false
# ... (relaxed strict rules for practicality)
```

### Running Pyright

```bash
# Type check
pyright --project pyproject.toml

# Show type coverage
pyright --project pyproject.toml --verifytypes fyi_system

# Watch mode
pyright --project pyproject.toml --watch
```

### Strict Mode Rules

Pyright strict mode checks:
- ✅ All function parameters must be typed
- ✅ All variables must be typed
- ✅ No `Any` types allowed (with exceptions)
- ✅ All return types must be specified
- ✅ Type narrowing must be correct
- ✅ Optional handling must be explicit
- ✅ Union types must be handled

---

## Profiling: Scalene

### What is Scalene?

[Scalene](https://github.com/plasma-umass/scalene) is a high-performance CPU, GPU, and memory profiler for Python.

### Why Scalene?

- **Fast:** Low overhead profiling
- **Detailed:** Line-by-line profiling
- **Multi-metric:** CPU + GPU + Memory
- **AI-powered:** Suggests optimizations
- **Visual:** HTML reports with annotations

### Installation

```bash
pip install scalene
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

### Alternatives Considered

| Tool | Why Not Chosen |
|------|----------------|
| **cProfile** | Too slow, no memory profiling |
| **line_profiler** | CPU only, slow |
| **memory_profiler** | Memory only, slow |
| **py-spy** | Requires sudo, sampling only |
| **austin** | Good but less features than Scalene |

**Scalene wins** for being fast, comprehensive, and CI-friendly.

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
| **Linting** | Ruff | Strict (80+ rule sets) |
| **Security** | Ruff (`S` rules) | All bandit rules included |
| **Type Checking** | Pyright | Strict mode |
| **Profiling** | Scalene | CPU + GPU + Memory |
| **Testing** | pytest | With coverage |
| **Dependencies** | pip-audit + safety | Dual scanning |

---

## Quick Reference

```bash
# Full development workflow
ruff check src/ tests/ --fix      # Lint and fix
ruff format src/ tests/           # Format code
pyright --project pyproject.toml  # Type check
pytest --cov=fyi_system           # Test with coverage
scalene tests/                    # Profile performance
pip-audit -r pyproject.toml       # Check dependencies
```

---

## Resources

- [Ruff Documentation](https://docs.astral.sh/ruff/)
- [Pyright Documentation](https://github.com/microsoft/pyright)
- [Scalene Documentation](https://github.com/plasma-umass/scalene)
- [Ruff Bandit Rules](https://docs.astral.sh/ruff/rules/#flake8-bandit-s)

---

**Questions?** See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
