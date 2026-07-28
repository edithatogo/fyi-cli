# Non-Alaveteli FOI platform landscape

Research snapshot: 2026-07-12. This inventory deliberately excludes Alaveteli deployments such as FYI, WhatDoTheyKnow, RightToKnow, and the paired Alaveteli work. It distinguishes public APIs from websites that merely provide a web form.

## Candidate platforms

| Platform | Jurisdiction / role | Evidence surface | Adapter difficulty | Recommendation |
|---|---|---|---|---|
| MuckRock | United States public-records requests | Public v2 API exposes requests, communications, files, agencies, jurisdictions, users, organizations, and projects; authenticated filing is supported and may consume paid request credits. | **Low-medium** for read-only sync; medium-high for writes | First pilot. Implement read-only discovery, request, correspondence, attachment, and status mapping before any write path. |
| FragDenStaat / Froide | Germany and EU access-to-information requests | Official API root, OpenAPI schema, request and public-body endpoints, OAuth app support, and prefilled request URLs. | **Medium**; schema and German/EU legal semantics need an adapter | Second pilot. Start with read-only API contract fixtures and explicit German/EU instance metadata. |
| FOIA.gov | United States federal FOIA portal | Official API exposes agency components and supports portal-to-agency request submission; API-key and agency-specific security constraints apply. | **Medium-high**; centralized discovery but decentralized agency delivery | Read-only agency/catalog adapter first; submission only through a separately approved, agency-scoped workflow. |
| USCIS FOIA/PA API | USCIS A-File and subject-specific requests | Official OAuth client-credentials API supports case creation and status, with sandbox onboarding, required UI language, identity/consent fields, and production prerequisites. | **High / narrow** | Defer to a separate opt-in adapter; do not generalize as a national FOIA provider. |
| Government portals without documented APIs | Canada ATIP, India RTI, and agency/state portals | Official portals provide forms and case handling, but the research pass found no stable public general-purpose API suitable for safe automation. | **High / site-specific** | Track as discovery-only or prefilled/manual routes, not scraping targets. |

## Explicit exclusions

- Alaveteli deployments remain covered by the existing jurisdiction/provider model and paired Alaveteli work.
- A website being searchable does not authorize scraping, submission, or bypassing anti-bot controls.
- Government APIs with identity, payment, consent, or agency-specific terms require an explicit provider policy and operator confirmation before writes.

## Evidence register

- MuckRock API: <https://www.muckrock.com/api/>; Python client request contract: <https://python-muckrock.readthedocs.io/en/latest/requests.html>
- FragDenStaat API and OpenAPI schema: <https://fragdenstaat.de/en/api/>; platform overview: <https://fragdenstaat.de/en/about-us/>
- FOIA.gov portal API: <https://www.foia.gov/swagger.html>; agency API contract: <https://www.foia.gov/developer/agency-api/>
- USCIS FOIA/PA API: <https://developer.uscis.gov/api/foia-request-and-status>
- Canada ATIP context: <https://www.canada.ca/en/treasury-board-secretariat/news/2022/07/canada-launches-updated-access-to-information-and-privacy-online-request-service-platform.html>
- India RTI portal context: <https://services.india.gov.in/service/detail/submit-a-new-rti-request-online-1>

## Integration decision

The lowest-risk expansion is a read-only `FoiProvider` adapter layer for MuckRock and FragDenStaat. FOIA.gov and USCIS should remain separately governed because their APIs create legal, identity, consent, billing, or agency-delivery obligations that do not fit an unrestricted bulk client.

## Capability matrix for adapter work

| Capability | MuckRock | FragDenStaat | FOIA.gov | USCIS |
|---|---:|---:|---:|---:|
| Public request search/retrieval | Yes | Yes | Agency/catalog focused | Case/status focused |
| Public-body/agency discovery | Yes | Yes | Yes | No general catalog |
| Correspondence and attachments | Yes | API surface to verify in fixtures | Agency-dependent | Case-specific |
| Auth required for reads | No for documented public endpoints | OAuth available for account operations | API key for portal API | OAuth client credentials |
| Write capability | Yes; account/credit implications | OAuth/account and jurisdiction rules | Delivery is agency-specific | Identity/consent and production approval |
| Recommended first mode | Read-only | Read-only | Read-only discovery | Disabled evaluation |
| Risk gate | Rate limit and paid credits | GDPR/legal semantics | Agency contract variance | Identity, consent, sandbox, production approval |

The adapter track must treat this matrix as a contract checklist, not as an assumption that every endpoint is interchangeable.

## Operational rollout contract for non-Alaveteli adapters

The current adapter fleet is intentionally narrow:

- **Enabled read-only providers:** MuckRock and FragDenStaat through the
  provider-neutral `ReadOnlyFoiProvider` contract.
- **Separately governed discovery only:** FOIA.gov agency/component catalog
  reads, and only after operator-supplied API-key approval.
- **Explicitly disabled:** USCIS request creation, status polling, or any
  identity-sensitive workflow.

### Rollout

- Ship new providers behind explicit instance metadata and fail-closed
  capability flags.
- Keep live smoke opt-in, bounded, read-only, and publicly attributable.
- Preserve `provider_id`, `source_url`, retrieval time where available, and
  `raw_source_hash` so downstream archive and provenance tooling can prove
  origin without replaying live calls.

### Rollback

- Remove or disable the affected instance entry rather than silently falling
  back to scraping or alternate write paths.
- Treat schema drift, auth changes, or legal-policy regressions as a reason to
  demote the provider back to discovery-only or disabled status.
- Do not retain a "best effort" write mode for non-Alaveteli providers; writes
  stay unavailable unless the provider contract explicitly supports them.

### Rate limits and operator posture

- Use fixed HTTPS endpoints, bounded request counts, page sizes, bytes, and
  timeouts for all public smoke sensors.
- Identify traffic with the repository User-Agent and never use authenticated
  or credit-consuming routes in default validation.
- Keep FOIA.gov and USCIS out of unattended polling until their separate
  operator, consent, and approval gates are satisfied.

### Source attribution and deprecation

- Attribute every normalized record back to its source platform and public URL.
- Keep legal and operational differences visible in documentation instead of
  flattening providers into an "FOI is FOI" abstraction.
- When an upstream provider changes ownership, terms, auth, or response shape,
  update the documented risk gate first and only restore live status after
  fixtures, fingerprints, and operator review are refreshed.
