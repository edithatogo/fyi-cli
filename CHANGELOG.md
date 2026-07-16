# Changelog

All notable changes to FYI Request System are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.2.1] - 2026-07-15

### Fixed

- Capture attachment links exposed by rendered Alaveteli request pages when the
  request JSON does not include attachment metadata, including WARC/WACZ and
  content-addressed derived-store records.

---

## [1.1.0](https://github.com/edithatogo/fyi-cli/compare/v1.0.0...v1.1.0) (2026-06-30)


### Features

* **cli:** add MFA management commands ([5e70f74](https://github.com/edithatogo/fyi-cli/commit/5e70f74a65d92ef25e8b7b8b0129e904547b7037))
* **dashboard:** add attachment link previews ([8b9731b](https://github.com/edithatogo/fyi-cli/commit/8b9731b7a27aa46b18846cf5d3badc62ad2b48e1))
* **dashboard:** add auto-save inline request editor ([5657128](https://github.com/edithatogo/fyi-cli/commit/5657128e148b4c9936c8ac680cdcba1ab3e8a8e2))
* **dashboard:** add bulk status update ([c5936b6](https://github.com/edithatogo/fyi-cli/commit/c5936b6a2eb21cd2670d9cde4a57acc1b192093a))
* **dashboard:** add request bulk export ([0b71841](https://github.com/edithatogo/fyi-cli/commit/0b718415612406ca06598a723cc1599c2097e7e3))
* **dashboard:** add request date range filter ([650ad63](https://github.com/edithatogo/fyi-cli/commit/650ad63a7428aae51c21a761c76a9d3ab381f8ea))
* **dashboard:** add request full-text search ([a41a4f0](https://github.com/edithatogo/fyi-cli/commit/a41a4f0aabaf396e5203adf1401aedb72aa3cc96))
* **dashboard:** add request status and authority filters ([b7e72fb](https://github.com/edithatogo/fyi-cli/commit/b7e72fbbcfa149cf4d45154018e281ce3f9f7231))
* **dashboard:** add timeline status indicators ([87180a8](https://github.com/edithatogo/fyi-cli/commit/87180a812019d1c1c2047e1ed057989200065b9e))
* **security:** add brute force protection and audit logging for MFA ([3c7f0ab](https://github.com/edithatogo/fyi-cli/commit/3c7f0aba5a0dead848349da87f3f95424035f2bf))
* **security:** add MFA guard for credential access ([c09bad6](https://github.com/edithatogo/fyi-cli/commit/c09bad68c88cb413c96775614235b7429f583ce9))
* **security:** add multi-key support and secret rotation ([2db6c4d](https://github.com/edithatogo/fyi-cli/commit/2db6c4dfd89d13265edfec0097675f1392cfd152))
* **security:** add TOTP provisioning URI generation ([c4740c0](https://github.com/edithatogo/fyi-cli/commit/c4740c0403b5dcefc605a0d6d07487d80de63d72))
* **security:** implement TOTP secret and code generation ([a05d40b](https://github.com/edithatogo/fyi-cli/commit/a05d40b00bfb2ee0ce581adf64fbb32e6b1c47cc))
* **security:** integrate TOTP secret storage with OS keyring ([df846a9](https://github.com/edithatogo/fyi-cli/commit/df846a9bf9192fdc2f8fc98d7a6eb0b8bfca9b2d))
* **sync:** add background sync scheduler with health checks ([8639d62](https://github.com/edithatogo/fyi-cli/commit/8639d62f5fa1bd19960d1880c03df131379b7b8c))
* **sync:** add conflict resolution (LWW and three-way merge) ([5cd7d88](https://github.com/edithatogo/fyi-cli/commit/5cd7d882c99f53dd3e2e1f9554ef25a5bb9807bf))
* **sync:** add conflict review and resolution interface ([1773466](https://github.com/edithatogo/fyi-cli/commit/1773466b94c7a715e934d3b86545c6ad2a6b5bfb))
* **sync:** add pull scheduler and manual trigger ([e154179](https://github.com/edithatogo/fyi-cli/commit/e154179f33c7e0cf5c1bf1d82cb00bfbe764d7df))
* **sync:** add retry logic and queue management ([e16b487](https://github.com/edithatogo/fyi-cli/commit/e16b487db02064ec79f9b40ad0df035e04fb3f42))
* **sync:** add sync dashboard and graceful offline operation ([ba85491](https://github.com/edithatogo/fyi-cli/commit/ba85491fcda0f380911ab692a2940a5e368a4541))
* **sync:** add sync metadata tracking and dirty flagging ([223cbd1](https://github.com/edithatogo/fyi-cli/commit/223cbd13d21de6c79f5e54280eba9d9b17cd35ad))
* **sync:** add sync status API, CLI command, and TUI display ([07c5320](https://github.com/edithatogo/fyi-cli/commit/07c5320e8151af494553c60b9db7802d921ed7ae))
* **sync:** implement incremental pull synchronization from FYI API ([39c2434](https://github.com/edithatogo/fyi-cli/commit/39c24343920559f76a7323ebac20eb956ac0a60c))
* **sync:** implement push synchronization for dirty records ([86b6df6](https://github.com/edithatogo/fyi-cli/commit/86b6df63de12e5e9e379500ae717d3052eff08d8))
* **tui:** add bulk operations, export triggers, and help system ([a2fa5e6](https://github.com/edithatogo/fyi-cli/commit/a2fa5e6bfbfe7e3059917f48b756ece5e547f4cf))
* **tui:** add credential manager dialog ([42d025f](https://github.com/edithatogo/fyi-cli/commit/42d025f35c47e32a0541ebc73d7967acf585fb2d))
* **tui:** add credential testing and session status display ([7e48ea1](https://github.com/edithatogo/fyi-cli/commit/7e48ea125faada28038a9528686695fd3a40d8ef))
* **tui:** add fuzzy search across all entities ([c8d389b](https://github.com/edithatogo/fyi-cli/commit/c8d389bd98d222954f58c2ec6ca36f835a043b12))
* **tui:** add keyboard shortcuts and status bar hints ([52c038c](https://github.com/edithatogo/fyi-cli/commit/52c038cd0f016f85b36bd15a445bf19d768d4f56))
* **tui:** add keyring browser dashboard ([b16c479](https://github.com/edithatogo/fyi-cli/commit/b16c4792f706666b89194e6ef474f8af819a5442))
* **tui:** add keyring management actions and security indicators ([cd50c3b](https://github.com/edithatogo/fyi-cli/commit/cd50c3b60a409a39cff4ea38f2b1648cefa7248a))
* **tui:** add markdown preview and auto-save for request editing ([f54eba9](https://github.com/edithatogo/fyi-cli/commit/f54eba9a216309acb1e16cdcabdbbbf57152cdb3))
* **tui:** add MFA integration to TUI and MCP server ([9b4db56](https://github.com/edithatogo/fyi-cli/commit/9b4db568cddda8acbfa00928f27111833fa0a965))
* **tui:** add request editor view with inline editing ([18f3d8f](https://github.com/edithatogo/fyi-cli/commit/18f3d8f7cebf7fd9a838571af3c4f32bc848c959))


### Bug Fixes

* **api:** harden sync contract errors ([e806a03](https://github.com/edithatogo/fyi-cli/commit/e806a0334336bcd5a5ead107ec3c540d0173a130))
* **ci:** split release please config and manifest ([2d5206d](https://github.com/edithatogo/fyi-cli/commit/2d5206d22e0b4f35f8a81dd4b749500f78a78f68))
* **ci:** update scalene profiling command ([d7c5547](https://github.com/edithatogo/fyi-cli/commit/d7c5547681be7eb3fad21f11b33e6b3751f9d519))
* **cli:** ignore unused request body binding ([74b8dad](https://github.com/edithatogo/fyi-cli/commit/74b8dad6ddd92eb62b9b71754c9c3373d8b46ba1))
* **dashboard:** add mobile navigation ([a19142a](https://github.com/edithatogo/fyi-cli/commit/a19142a96c1cd1e9c6b7164396d0212c0b7a0104))
* **release:** align release readiness surfaces ([b2b69a3](https://github.com/edithatogo/fyi-cli/commit/b2b69a3e0244e6a679340ebb56d589fee5dff014))
* **rust:** satisfy fmt and clippy checks ([0907e5e](https://github.com/edithatogo/fyi-cli/commit/0907e5e217c1742340cfba1940264cb2314c720e))
* **sync:** preserve metadata during remote merges ([529c42f](https://github.com/edithatogo/fyi-cli/commit/529c42f6891f88d4229393b24c61996b467f1235))


### Documentation

* **api:** add contract inventory ([ecad245](https://github.com/edithatogo/fyi-cli/commit/ecad24582fe405dd6c8637b9ab80eae0ea18153c))

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
  - `fyi setup` - Interactive setup wizard
  - `fyi config` - Configuration management
  - `fyi health-check` - System health verification
  - `fyi privacy-audit` - Privacy compliance check
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
https://github.com/edithatogo/fyi-cli/commits/main
