# Non-Alaveteli FOI platform landscape

## Overview

Maintain a source-backed map of non-Alaveteli public-information request platforms and identify which expose safe, documented integration surfaces.

## Requirements

- Cover civic platforms, official government portals, and significant regional services.
- Record jurisdiction, legal regime, platform ownership, software lineage, public API, authentication, write semantics, attachments, rate limits, privacy/identity requirements, and terms.
- Distinguish API-supported, prefilled/manual, scrape-only, inaccessible, and deprecated targets.
- Maintain explicit exclusion evidence for Alaveteli deployments so adapters are not duplicated.
- Produce a ranked shortlist based on user value, technical feasibility, legal/operational risk, and maintenance cost.

## Acceptance criteria

- `docs/foi-platform-landscape.md` has a dated evidence register and confidence level for every candidate.
- At least MuckRock, FragDenStaat, FOIA.gov, USCIS FOIA/PA, Canada ATIP, and India RTI are assessed.
- Every proposed adapter has a go/no-go rationale and a bounded next experiment.
- Research updates are reproducible from primary sources and do not claim unsupported API capabilities.

## Documentation observation contract (2026-11-17)

- The Rust maintenance adapter observes one of six reviewed, compiled documentation
  URLs only after an explicit live opt-in. It uses the existing source-network
  guard, one GET, a 30-second acquisition deadline and a 2 MiB response ceiling.
- DNS targets must be public and pinned. Redirects, credentials, cookies, ambient
  proxies, decompression, cache reuse and automatic retries are disabled.
- A successful capture binds the exact raw body and a canonical observation to an
  acquisition receipt v1. A fresh private directory receives the observation
  marker last. Failed, incomplete, deprecated or malformed responses cannot
  publish a successful observation or replace earlier evidence.
- Public versioned metadata stores source identity, retrieval date, media type,
  byte length and SHA-256. Source bodies remain in an operator-controlled bundle
  because documentation can contain page tokens, identity-shaped examples or
  material with uncertain redistribution rights.
- Baseline provenance states its actual acquisition method. The 2026-11-17
  research baseline used credential-free HTTPS through the managed proxy with
  normal TLS verification; it is not a guarded-adapter capture. Baseline-body
  verification checks those retained bytes without claiming receipt validation.
- The offline verifier distinguishes metadata validation from actual byte and
  receipt verification. Supplied observations must cover all six sources; wrong
  identities, duplicate JSON keys, unsafe paths, missing/tampered bytes, stale or
  unsupported timestamps, and inconsistent receipts fail closed.
- Byte/media changes require review. Neither equal hashes nor retrieved source
  documentation prove deployed API behaviour. A new USCIS schema URL or changes
  outside the selected documents are found by the existing 30/60-day primary
  source review, not inferred by a frozen URL monitor.
- CI uses synthetic, socket-free tests. Building an observer artifact is allowed;
  contacting live documentation sources from CI is not part of this contract.

## Out of scope

- Automated submission to a government portal without explicit provider policy and operator consent.
- Scraping protected or private surfaces.
- Replacing jurisdiction-specific legal advice with software defaults.

