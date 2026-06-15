# Specification: rust-cicd-publishing (Phase 4)

## Overview
This track delivers the automated build, testing, packaging, and distribution framework for the Rust rewrite of `fyi-cli`. It configures the GitHub Actions workflows and uses `cargo-dist` for cross-compiling release binaries.

## Functional Requirements
1. **GitHub Actions Workflow:**
   - Run lints (`clippy`), formatting (`rustfmt`), security checks (`cargo-deny`), and tests on every commit/PR.
   - Enforce build coverage constraints.
2. **Cross-Compilation Packaging:**
   - Configure `cargo-dist` to automatically generate tarball/zip release archives.
   - Build binaries for x86_64 and arm64 architectures on Windows, macOS, and Linux.
3. **Registry Publishing:**
   - Set up automated release triggers publishing crates to Crates.io.
   - Publish cross-compiled executables to GitHub Releases.
   - Scaffold package configurations for Homebrew (macOS) and Chocolatey/Winget (Windows).

## Non-Functional Requirements
- **Release Safety:** Automatic checks for dependency vulnerabilities.

## Acceptance Criteria
- GitHub Actions triggers on push, running linting, formatting, auditing, and testing.
- Release workflow executes compile targets for Windows, macOS, and Linux without failures.
- Crate successfully publishes to dry-run registries.
