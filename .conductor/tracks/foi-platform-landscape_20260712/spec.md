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

## Out of scope

- Automated submission to a government portal without explicit provider policy and operator consent.
- Scraping protected or private surfaces.
- Replacing jurisdiction-specific legal advice with software defaults.

