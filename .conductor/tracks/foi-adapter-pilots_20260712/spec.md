# Non-Alaveteli FOI adapter pilots

## Overview

Implement the lowest-risk, highest-value non-Alaveteli provider adapters after the landscape track confirms current contracts.

## Initial candidates

1. MuckRock read-only API adapter.
2. FragDenStaat read-only API adapter.
3. FOIA.gov agency/catalog read-only adapter, with submission disabled by default.
4. USCIS FOIA/PA adapter only as a separately gated, identity-aware pilot if production access is granted.

## Requirements

- Reuse the Rust `FoiProvider` and instance registry boundaries.
- Preserve source provenance, legal jurisdiction, provider IDs, timestamps, raw response hashes, and rate-limit metadata.
- Add offline fixtures before live tests; live tests are opt-in, bounded, and read-only by default.
- Require explicit capability flags for search, request retrieval, correspondence, attachments, status, and writes.
- Reject unsupported write operations rather than silently falling back to scraping.
- Include property, contract, integration, edge, compatibility, security, and regression coverage.

## Acceptance criteria

- MuckRock and FragDenStaat read-only adapters pass offline contract fixtures and bounded opt-in smoke tests.
- FOIA.gov is represented without unsafe assumptions about agency delivery.
- USCIS remains disabled unless its OAuth, sandbox, consent, and production requirements are satisfied.
- Documentation explains legal and operational differences between providers.

## Out of scope

- Automated portal scraping.
- Bulk filing, identity-sensitive requests, paid request credits, or CAPTCHA bypass.
- Alaveteli provider changes.

