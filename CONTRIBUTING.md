# Contributing to FYI Request System

**Thank you for your interest in contributing!**

This document provides guidelines and instructions for contributing to the project.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [How to Contribute](#how-to-contribute)
4. [Development Setup](#development-setup)
5. [Coding Standards](#coding-standards)
6. [Testing](#testing)
7. [Submitting Changes](#submitting-changes)
8. [Code Review](#code-review)
9. [Documentation](#documentation)

---

## Code of Conduct

### Our Pledge

We pledge to make participation in our project a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, gender identity and expression, level of experience, nationality, personal appearance, race, religion, or sexual identity and orientation.

### Our Standards

Examples of behavior that contributes to creating a positive environment:

- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members

Examples of unacceptable behavior:

- The use of sexualized language or imagery and unwelcome sexual attention
- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate

### Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported privately through GitHub repository owner contact channels.

---

## Getting Started

### Where to Start

1. **Check existing issues:** https://github.com/edithatogo/fyi-cli/issues
2. **Look for labels:**
   - `good first issue` - Good for newcomers
   - `help wanted` - Extra attention needed
   - `bug` - Something isn't working
   - `enhancement` - New feature or request
3. **Read the documentation:**
   - [USER_GUIDE.md](USER_GUIDE.md)
   - [INSTALL.md](INSTALL.md)
   - [QUICKSTART.md](QUICKSTART.md)

### Questions?

- **General questions:** Use [GitHub Discussions](https://github.com/edithatogo/fyi-cli/discussions)
- **Bug reports:** Use [GitHub Issues](https://github.com/edithatogo/fyi-cli/issues)
- **Security issues:** See [SECURITY.md](.github/SECURITY.md)

---

## How to Contribute

### Types of Contributions

#### 1. Report Bugs

**Before reporting:**
- Check if the bug has already been reported
- Try to reproduce with the latest version

**When reporting:**
- Use a clear and descriptive title
- Describe the exact steps to reproduce
- Include expected vs actual behavior
- Provide system information (OS, Python version)
- Include logs if available

#### 2. Suggest Features

**Before suggesting:**
- Check if the feature has already been suggested
- Consider if it aligns with project goals

**When suggesting:**
- Use a clear and descriptive title
- Provide a detailed description
- Explain the use case
- Discuss alternatives considered

#### 3. Submit Code

**Before submitting:**
- Check if similar work is in progress
- Discuss large changes in an issue first

**When submitting:**
- Follow the coding standards
- Include tests
- Update documentation
- Write clear commit messages

#### 4. Improve Documentation

**Areas that need help:**
- Typos and clarifications
- Missing examples
- Translation to other languages
- Tutorials and guides

#### 5. Review Pull Requests

- Provide constructive feedback
- Test the changes if possible
- Check coding standards
- Verify tests pass

---

## Development Setup

`fyi-cli` is a **Rust workspace** (`crates/fyi-core`, `crates/fyi-cli`, `crates/fyi-mcp`) plus a
legacy `fyi_system` Python package kept as a reference/parity oracle. New feature work happens in
the Rust workspace; the Python code is not being extended.

### Prerequisites

- Rust (stable toolchain) via [rustup](https://rustup.rs/)
- Git
- Python 3.10+ (only needed for the legacy `fyi_system` package and its tests)

### Clone the Repository

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli
```

### Rust Workspace Setup

```bash
# Build the whole workspace
cargo build --workspace --all-features

# Format, lint, and test (the same gates CI runs)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

### Legacy Python Setup (optional)

```bash
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows
pip install -e ".[dev]"
pytest
```

### Working with Conductor Tracks

Larger, multi-step work is planned as [Conductor](https://conductor.build) tracks under
`.conductor/tracks/<slug>/` (spec + plan per track), registered in `.conductor/tracks.md` and
mirrored to GitHub epic/sub-issues. If you're picking up a tracked piece of work, check the
relevant track's `plan.md` for scope and open sub-issues before starting.

---

## Coding Standards

### Python Style

We follow [Google Python Style Guide](https://google.github.io/styleguide/pyguide.html) with these additions:

#### Naming Conventions

```python
# Classes: PascalCase
class AlaveteliClient:
    pass

# Functions and variables: snake_case
def build_prefilled_url():
    pass

# Constants: UPPER_CASE
DEFAULT_TIMEOUT = 30

# Private members: _prefix
_internal_cache = {}
```

#### Type Hints

Use type hints for all public functions:

```python
from typing import Optional, List, Dict

def get_request(request_id: int) -> Optional[AlaveteliRequest]:
    """Get request by ID."""
    pass

def search_requests(
    query: str,
    tags: Optional[List[str]] = None
) -> Dict[str, Any]:
    """Search requests."""
    pass
```

#### Docstrings

Use Google-style docstrings:

```python
def create_request(
    title: str,
    body: str,
    tags: Optional[List[str]] = None
) -> Dict[str, Any]:
    """Create a new request.
    
    Args:
        title: Request title
        body: Request body content
        tags: Optional tags for the request
    
    Returns:
        Dictionary with 'url' and 'id' of new request
    
    Raises:
        AlaveteliAPIError: If API key not provided
    """
    pass
```

#### Error Handling

```python
# Good: Specific exception handling
try:
    response = requests.get(url, timeout=timeout)
    response.raise_for_status()
except requests.exceptions.Timeout:
    raise AlaveteliAPIError("Request timed out", 408)
except requests.exceptions.HTTPError as e:
    raise AlaveteliAPIError(f"API error: {e}", e.response.status_code)

# Bad: Catching all exceptions
try:
    do_something()
except Exception:
    pass  # Never do this!
```

### Git Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting (no code change)
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(alaveteli): Add comprehensive API client

- Implement Read API support
- Implement Write API support
- Add attachment handling
- Add health check

Closes #123
```

```
fix(security): Fix encryption key derivation

- Use constant-time comparison
- Add key rotation support

Fixes #456
```

---

## Testing

### Rust Workspace (primary, release-gating)

```bash
# Run all tests
cargo test --workspace --all-features

# Run tests for a single crate
cargo test -p fyi-core

# Run a specific test
cargo test -p fyi-core sync::tests::test_pull_feed
```

### Legacy Python Tests (optional, `fyi_system` reference only)

### Running Tests

```bash
# Run all tests
pytest

# Run with coverage
pytest --cov=fyi_system --cov-report=html

# Run specific test file
pytest tests/test_alaveteli_client.py

# Run specific test
pytest tests/test_alaveteli_client.py::TestReadAPI::test_get_request
```

### Writing Tests

**Test file naming:**
```
tests/test_<module>.py
```

**Test class naming:**
```python
class TestAlaveteliClient:
    pass
```

**Test function naming:**
```python
def test_get_request_success():
    pass

def test_get_request_not_found():
    pass
```

**Use fixtures:**
```python
@pytest.fixture
def client():
    return AlaveteliClient(base_url='https://test.org')

def test_get_request(client):
    result = client.get_request(123)
    assert result is not None
```

**Mock external services:**
```python
@patch('fyi_system.alaveteli_client.requests.Session.get')
def test_get_request(mock_get, client):
    mock_response = Mock()
    mock_response.json.return_value = {'id': 123}
    mock_get.return_value = mock_response
    
    result = client.get_request(123)
    assert result.id == 123
```

### Coverage Requirements

- **Overall:** >80%
- **New features:** >90%
- **Critical code:** >95%

Check coverage:
```bash
pytest --cov=fyi_system --cov-report=term-missing --cov-fail-under=80
```

---

## Submitting Changes

### Pull Request Process

1. **Fork the repository**

2. **Create a branch:**
   ```bash
   git checkout -b feature/my-feature
   # or
   git checkout -b fix/my-bugfix
   ```

3. **Make your changes:**
   - Follow coding standards
   - Add tests
   - Update documentation

4. **Commit your changes:**
   ```bash
   git add .
   git commit -m "feat(scope): Add my feature"
   ```

5. **Push to your fork:**
   ```bash
   git push origin feature/my-feature
   ```

6. **Open a Pull Request:**
   - Use the PR template
   - Link related issues
   - Describe your changes
   - Add screenshots if applicable

### Pull Request Template

Opening a PR automatically applies [.github/PULL_REQUEST_TEMPLATE.md](.github/PULL_REQUEST_TEMPLATE.md) — fill it in rather than deleting it.

---

## Code Review

### Reviewer Guidelines

- Be respectful and constructive
- Focus on the code, not the person
- Explain the reasoning behind suggestions
- Acknowledge good work
- Use suggestions when possible

### Review Criteria

- **Correctness:** Does the code work as intended?
- **Security:** Are there any security issues?
- **Performance:** Is the code efficient?
- **Maintainability:** Is the code easy to understand and modify?
- **Testing:** Are there adequate tests?
- **Documentation:** Is the code documented?

### Approval Process

- All PRs require at least 1 approval
- Large changes may require 2+ approvals
- CI must pass before merging
- Maintainer has final approval

---

## Documentation

### Documentation Standards

- Use clear, concise language
- Include examples
- Keep it up-to-date
- Use markdown formatting
- Add cross-references

### Documentation Files

| File | Purpose |
|------|---------|
| `README.md` | Project overview |
| `INSTALL.md` | Installation guide |
| `QUICKSTART.md` | Getting started |
| `USER_GUIDE.md` | User documentation |
| `API_KEY_SETUP.md` | API configuration |
| `CONFIGURATION.md` | Configuration reference |
| `TROUBLESHOOTING.md` | Troubleshooting guide |
| `FAQ.md` | Frequently asked questions |
| `CONTRIBUTING.md` | This file |
| `CHANGELOG.md` | Version history |
| `SECURITY.md` | Security policy |

### Building Documentation

```bash
# If using Sphinx or similar
cd docs
make html

# Open in browser
open _build/html/index.html  # macOS
xdg-open _build/html/index.html  # Linux
start _build/html\index.html  # Windows
```

---

## Release Process

### Version Numbering

We use [Semantic Versioning](https://semver.org/):

- **MAJOR.MINOR.PATCH** (e.g., 1.0.0)
- **MAJOR:** Breaking changes
- **MINOR:** New features (backward compatible)
- **PATCH:** Bug fixes

### Release Checklist

- [ ] All tests pass
- [ ] Coverage requirements met
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version number updated
- [ ] Release notes written
- [ ] Tagged release
- [ ] Published to PyPI
- [ ] Executables built
- [ ] Announcement made

---

## Getting Help

- **Documentation:** See docs/ directory
- **Issues:** https://github.com/edithatogo/fyi-cli/issues
- **Discussions:** https://github.com/edithatogo/fyi-cli/discussions

---

## Thank You!

Your contributions make this project better for everyone. We appreciate your time and effort!

**Happy coding!** 🚀
