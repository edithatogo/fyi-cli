# Changelog

All notable changes to FYI Request System are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Planned for v1.0.0
- Complete documentation suite
- PyPI package release
- Standalone executables (Windows, macOS, Linux)
- Security hardening
- Performance optimizations

---

## [1.0.0] - 2026-03-30

### Added
- **Alaveteli API Client** - Full Read + Write API support
  - Create requests programmatically
  - Add correspondence with attachments
  - Update request states
  - Compatible with all Alaveteli instances
  
- **Security Features**
  - AES-256-GCM encryption for sensitive data
  - PBKDF2-HMAC-SHA256 key derivation
  - OS keyring integration for credential storage
  - Tamper-evident audit logging with hash chaining
  - Secure session management with timeout
  - Data retention policies with secure deletion
  - CSRF protection for web forms
  - Input validation and sanitization
  - Security headers (CSP, HSTS, X-Frame-Options)

- **Documentation**
  - INSTALL.md - Installation guide
  - QUICKSTART.md - 5-minute getting started
  - USER_GUIDE.md - Comprehensive user guide
  - API_KEY_SETUP.md - API key configuration
  - CONFIGURATION.md - Configuration reference
  - TROUBLESHOOTING.md - Troubleshooting guide
  - FAQ.md - Frequently asked questions
  - ALAVETELI_CLIENT.md - API client documentation

- **Testing**
  - 500+ unit tests
  - Integration tests
  - Security verification tests
  - Cross-platform compatibility tests

- **CLI Commands**
  - `fyi-system setup` - Interactive setup wizard
  - `fyi-system config` - Configuration management
  - `fyi-system health-check` - System health verification
  - `fyi-system privacy-audit` - Privacy compliance check
  - All existing commands enhanced with better error messages

### Changed
- Improved error messages to be user-friendly
- Enhanced progress indicators for long operations
- Better help text with examples for all commands
- Performance optimizations for large datasets

### Fixed
- Unicode encoding issues in error messages
- Database locking issues with concurrent access
- API rate limiting handling
- File permission issues on all platforms

### Security
- Zero critical vulnerabilities (verified by pip-audit)
- All dependencies scanned and up-to-date
- Security policy established (SECURITY.md)

---

## [0.14.0] - 2026-03-09

### Added
- Security hardening track completed (8 phases)
- Encryption infrastructure (Phase 1)
- Secure credential storage (Phase 2)
- Session management (Phase 3)
- Audit logging (Phase 4)
- Data retention policies (Phase 5)
- Input validation & security headers (Phase 6)
- Security verification suite (Phase 7)
- Code review fixes (Phase 8)

### Changed
- Renamed from "FYI Request System" to support multiple Alaveteli instances
- Improved test coverage to 83%

### Fixed
- Email redaction edge cases
- Filename sanitization issues
- Various bug fixes from code review

---

## [0.13.0] - 2026-03-08

### Added
- Testing infrastructure track
- 280+ automated tests
- 36 API contract tests
- 22 E2E CLI tests
- Mutation testing infrastructure
- Load testing framework

### Changed
- Coverage improved from 62% to 80%
- Fixed 135 linting errors
- Fixed 3 type checking errors

---

## [0.12.0] - 2026-03-08

### Added
- Integration of FYI request system history from v1-v14 archives
- Preserved provenance and versioning
- Merged all previous development tracks

---

## [0.11.0] - 2026-03-07

### Added
- Web application improvements
- Next-best-action recommendations
- Correspondence pack generation
- Request detail route enhancements

---

## [0.10.0] - 2026-03-06

### Added
- Export bundle functionality
- Direct "Open recommended draft" actions
- Improved snapshot parsing

---

## [0.9.0] - 2026-03-05

### Added
- Privacy and security enhancements
- File/directory permissions for database and outputs
- `privacy-audit` command
- `show-settings` command
- Bundled exports with sanitization
- Security headers for web UI

---

## [0.8.0] - 2026-03-04

### Added
- Follow-up generation improvements
- Triage report enhancements
- Response analysis features

---

## [0.7.0] - 2026-03-03

### Added
- Dashboard generation
- Attention report improvements
- Handover document generation

---

## [0.6.0] - 2026-03-02

### Added
- Feed monitoring improvements
- Scheduler enhancements
- Reconciliation features

---

## [0.5.0] - 2026-03-01

### Added
- Request timeline tracking
- Status update functionality
- Export/import functionality

---

## [0.4.0] - 2026-02-28

### Added
- Authority management
- CSV import for authorities
- Request registration improvements

---

## [0.3.0] - 2026-02-27

### Added
- Web interface improvements
- Request list filtering
- Search functionality

---

## [0.2.0] - 2026-02-26

### Added
- Basic web interface
- Database initialization
- Core CLI commands

---

## [0.1.0] - 2026-02-25

### Added
- Initial release
- Basic CLI functionality
- SQLite database backend
- FYI.org.nz integration

---

## Release Notes

### Version Numbering

- **Major version** (1.x.x): Breaking changes or major new features
- **Minor version** (x.1.x): New features, backward compatible
- **Patch version** (x.x.1): Bug fixes and minor improvements

### Support

- **v1.x.x** - Current stable (supported)
- **v0.14.x** - Previous stable (security fixes only)
- **v0.x.x** - Legacy (unsupported)

### Upgrade Path

**From v0.x to v1.0:**
- Database migrations are automatic
- Configuration is backward compatible
- No manual intervention required

---

**For detailed commit history:**
https://github.com/yourusername/fyi-cli/commits/main
