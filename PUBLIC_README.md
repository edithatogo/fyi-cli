# FYI Request System

A privacy-focused tool for managing official information requests through FYI.org.nz and similar platforms.

## Features

- **Request Management**: Track and manage official information requests
- **Privacy-First**: Built-in redaction, TOR/proxy support, secure data handling
- **Automated Workflows**: Feed monitoring, follow-up generation, correspondence packs
- **Local-First**: All data stored locally in SQLite, no cloud dependencies
- **CLI + Web UI**: Command-line interface and local web dashboard

## Installation

```bash
pip install -e ".[dev]"
```

## Quick Start

```bash
# Initialize database
fyi-system init-db

# Import authorities
fyi-system import-authorities data/sample_authorities.csv

# Register a new request
fyi-system register-request ministry-of-justice "Request for Documents" "I request all documents related to..."

# Start web dashboard
fyi-system serve
```

## Architecture

```
src/fyi_system/
├── cli.py           # Command-line interface
├── webapp.py        # Local web server
├── db.py            # SQLite database operations
├── fyi.py           # FYI.org.nz API integration
├── monitor.py       # Feed monitoring
├── scheduler.py     # Automated scheduling
├── reporting.py     # Report generation
├── security.py      # Privacy and security functions
└── dashboard.py     # Dashboard generation
```

## Development

```bash
# Run tests
pytest --cov=fyi_system

# Run with coverage
pytest --cov=fyi_system --cov-report=html

# Lint
ruff check .

# Type check
mypy src/
```

## Quality Standards

This is a research-grade tool with:
- >95% test coverage
- Mutation testing (>90% mutation score)
- Property-based testing with Hypothesis
- Load testing and performance benchmarks

## Privacy & Security

- All data stored locally
- Email and PII redaction
- TOR/proxy support for anonymous requests
- Secure file permissions
- No telemetry or analytics

## License

MIT License - See LICENSE file for details.

## Disclaimer

This tool is for legitimate official information requests only. Users are responsible for complying with applicable laws and FYI.org.nz terms of service.
