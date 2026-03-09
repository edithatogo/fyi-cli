# FYI CLI v1.0.0 - Release Summary

**Release Date:** 2026-03-09  
**Version:** 1.0.0  
**Repository:** https://github.com/edithatogo/fyi-cli

---

## 🎉 Release Highlights

FYI CLI v1.0.0 is a **production-ready**, privacy-focused command-line tool for managing Official Information Act (OIA) requests through FYI.org.nz.

### Key Features

✅ **Complete CLI Application**
- 20+ commands for request management
- Web UI for visual users
- Automated feed monitoring
- Report generation (dashboard, attention, handover)

✅ **Security Hardening**
- AES-256-GCM encryption
- OS keyring integration
- Tamper-evident audit logging
- Input validation & sanitization
- Security headers (CSP, HSTS, X-Frame-Options)

✅ **Comprehensive Testing**
- 472 automated tests
- 73% code coverage
- 22 hypothesis property-based tests
- Load testing with Locust
- Performance benchmarks

✅ **Modern Tooling**
- Typer for CLI
- Rich for terminal output
- HTTPX for async HTTP
- Pydantic for validation
- FastMCP for AI integration
- UV for fast package management

✅ **Automated Publishing**
- PyPI auto-publish on release
- TestPyPI for pre-releases
- Conda packages (Anaconda.org)
- GitHub Releases with changelog

✅ **CI/CD Pipeline**
- GitHub Actions workflows
- CodeQL security scanning
- Automated dependency updates (Renovate)
- Automated releases (Release Please)

---

## 📦 Installation

### From PyPI (Recommended)

```bash
pip install fyi-cli
```

### From TestPyPI (Pre-releases)

```bash
pip install -i https://test.pypi.org/simple/ fyi-cli
```

### From Conda

```bash
conda install -c edithatogo fyi-cli
```

### From Source

```bash
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli
pip install -e ".[dev]"
```

---

## 🚀 Quick Start

```bash
# Initialize database
fyi init-db

# Import authorities
fyi import-authorities data/sample_authorities.csv

# Create first request
fyi register-request ministry-of-justice \
  "Request for Departmental Spending Data" \
  "I request the following information..." \
  --tags spending official-information

# Generate submission URL
fyi build-prefilled-url 1

# Start web UI
fyi serve

# Generate dashboard
fyi dashboard --output dashboard.html
```

---

## 📊 Test Results

### Unit Tests
- **Total:** 472 tests
- **Passing:** 472 (100%)
- **Coverage:** 73%

### Hypothesis Tests
- **Total:** 22 property-based tests
- **Passing:** 22 (100%)
- **Examples generated:** 500+ per test

### Security Tests
- **Email redaction:** ✅ All edge cases covered
- **Input validation:** ✅ Comprehensive
- **Audit logging:** ✅ Tamper-evident

### Load Tests
- **Concurrent users:** 10, 100, 500
- **Sustained load:** 1 hour
- **Performance baselines:** Documented

---

## 🔐 Security Features

| Feature | Status | Details |
|---------|--------|---------|
| **Encryption** | ✅ Complete | AES-256-GCM, PBKDF2 |
| **Credential Storage** | ✅ Complete | OS keyring |
| **Session Management** | ✅ Complete | Timeout, invalidation |
| **Audit Logging** | ✅ Complete | Hash chaining |
| **Input Validation** | ✅ Complete | All inputs validated |
| **Security Headers** | ✅ Complete | CSP, HSTS, etc. |

---

## 📁 Project Structure

```
fyi-cli/
├── src/fyi_system/          # Main package (22 modules)
│   ├── cli.py               # CLI commands
│   ├── webapp.py            # Web UI
│   ├── security.py          # Security features
│   ├── encryption.py        # Encryption
│   ├── credentials.py       # Credential management
│   ├── sessions.py          # Session management
│   ├── audit.py             # Audit logging
│   ├── retention.py         # Data retention
│   └── ...                  # Other modules
├── tests/                   # Test suite
│   ├── test_*.py            # Unit tests
│   ├── test_hypothesis.py   # Property-based tests
│   ├── test_benchmarks.py   # Performance tests
│   └── load_test_locust.py  # Load tests
├── .github/workflows/       # CI/CD
│   ├── ci.yml               # CI pipeline
│   ├── release.yml          # Release automation
│   ├── codeql.yml           # Security scanning
│   └── ...
├── docs/                    # Documentation
│   ├── ALAVETELI_CLIENT.md
│   ├── SECURITY_CONFIG.md
│   └── ...
└── conda/                   # Conda recipe
    └── meta.yaml
```

---

## 🛠️ Development

### Setup

```bash
# Clone repository
git clone https://github.com/edithatogo/fyi-cli.git
cd fyi-cli

# Create virtual environment
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows

# Install dependencies
uv pip install -e ".[dev]"
```

### Run Tests

```bash
# All tests
pytest

# With coverage
pytest --cov=fyi_system --cov-report=html

# Load tests
locust -f tests/load_test_locust.py --headless -u 100 -r 10 --run-time 60s

# Benchmarks
pytest tests/test_benchmarks.py --benchmark-only

# Mutation testing
python mutation_test.py
```

### Code Quality

```bash
# Lint
ruff check src/ tests/

# Format
ruff format src/ tests/

# Type check
basedpyright --project pyproject.toml
```

---

## 📈 Performance Benchmarks

| Operation | Time | Memory |
|-----------|------|--------|
| Database init | <100ms | <10MB |
| Request insert | <10ms | <1MB |
| Dashboard generation | <500ms | <50MB |
| Email redaction | <1ms | <1MB |
| Concurrent reads (10x) | <100ms | <20MB |

---

## 🚧 Known Limitations

1. **Test Coverage:** 73% (target: 95% in v1.1.0)
   - webapp.py: 63% (needs more UI tests)
   - retention.py: 72% (needs more edge case tests)

2. **Mutation Testing:** Infrastructure ready, full run pending
   - Estimated time: 60+ minutes
   - Target score: >80%

3. **Conda Distribution:** Available on anaconda.org, pending conda-forge acceptance

---

## 📋 Roadmap

### v1.1.0 (Next Release)
- [ ] Increase test coverage to 95%
- [ ] Complete mutation testing
- [ ] Add more webapp tests
- [ ] Conda-forge submission
- [ ] Performance optimizations

### v1.2.0
- [ ] TOR/proxy integration
- [ ] Multi-account management UI
- [ ] Advanced search features
- [ ] Export to PDF

### v2.0.0 (Future)
- [ ] Rust reimplementation
- [ ] Native GUI
- [ ] Cloud sync option
- [ ] Mobile app

---

## 🙏 Acknowledgments

- **FYI.org.nz** - For the official information platform
- **mySociety** - For Alaveteli platform
- **Transparency International NZ** - For supporting transparency

---

## 📄 License

MIT License - See LICENSE file for details

---

## 🔗 Links

- **Repository:** https://github.com/edithatogo/fyi-cli
- **PyPI:** https://pypi.org/project/fyi-cli/
- **TestPyPI:** https://test.pypi.org/project/fyi-cli/
- **Anaconda:** https://anaconda.org/edithatogo/fyi-cli
- **Issues:** https://github.com/edithatogo/fyi-cli/issues
- **Documentation:** https://github.com/edithatogo/fyi-cli#readme

---

**Released with ❤️ for transparency and privacy in New Zealand**
