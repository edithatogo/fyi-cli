# Non-Alaveteli FOI platform landscape

Reviewed: **2026-11-14**. Scope: a research and prioritisation snapshot, **not
provider enablement, legal advice, or evidence of a successful live API call**.
The machine-readable record is
[`foi-platform-landscape-v1.json`](../conformance/providers/foi-platform-landscape-v1.json).

## Ranked decisions

Ranks are engineering judgements, not measured performance scores. Public-data
utility and documented contracts take priority over feature breadth; identity,
submission, payment and maintenance risks reduce priority. Every candidate has
its own jurisdiction, confidence, evidence and next experiment in the inventory.

| Rank | Candidate | Decision and rationale | Bounded next experiment | Confidence / refresh |
|---|---|---|---|---|
| 1 | MuckRock, United States | **Go: read-only pilot.** Request, agency, correspondence and file surfaces are a useful match; no filing or account enumeration. | Offline fixtures for one public request, agency, communication and file; separately authorise any live smoke. | Documented; 30 days |
| 2 | FragDenStaat / Froide, Germany | **Conditional go: read-only pilot.** Reconcile the deployed instance with framework documentation; retain German/EU distinctions. | Freeze public request/public-body fixtures and check deployed schema/auth before live work. | Provisional; 30 days |
| 3 | FOIA.gov, US federal | **Go: discovery only.** Agency/component catalogue is distinct from agency-side request delivery. | Offline component/form fixture; an operator supplies any API key for later live catalogue checks. | Documented; 30 days |
| 4 | USCIS FOIA/PA, United States | **No-go for the default fleet; defer.** Case-specific identity/consent workflow is not public corpus discovery. | Review sandbox contract and required interface text; no real subjects, case creation or polling. | Provisional; 30 days |
| 5 | Canada ATIP, federal | **No-go for automated transactions; manual only.** A suitable public transaction API was not identified in this review. | Document institution/IRCC routing and manually supplied exports; assess released summaries separately. | Limited search; 60 days |
| 6 | India RTI Online, central authorities | **No-go for automated transactions; manual only.** Do not infer an API from public forms or applicant status pages. | Manual routing and consent-aware export fixtures; no payment, OTP, CAPTCHA or filing automation. | Limited search; 60 days |

## Evidence and boundaries

**MuckRock.** The official API guide describes some unauthenticated operations,
access-token authentication for others, and the `api_v2` resource families.
Its stated general limit is 15 requests/minute with a burst allowance of 100;
users/organisations have a separate 5/minute limit without bursts. These are
upstream limits, not a recommendation to use the burst allowance. The pilot
excludes user/organisation enumeration. Public records can still contain personal
data. Sources: [API guide][muckrock], [maintainer client reference][muckrock-client].

**FragDenStaat / Froide.** Official developer material describes an API and OAuth
application registration; Froide documents the API-v1 request/public-body model.
This review could inspect indexed primary documentation, but direct page retrieval
was blocked/unavailable. It does **not** verify a deployed schema, a numeric
throttle limit, or endpoint availability. Do not carry old framework-auth advice
into a deployment without checking it. Sources: [operator developer guidance][fds],
[framework API documentation][froide].

**FOIA.gov — United States federal FOIA portal.** The public developer
documentation describes API-key-protected agency components and annual-report XML. The separate agency specification describes
portal-to-agency delivery with agency-specific security controls; it is not an
unrestricted third-party filing endpoint. The reviewed draft leaves rate limits
unspecified. Sources: [developer resources][foia], [agency specification][agency].

**USCIS.** Indexed official material describes case creation, case status,
developer/sandbox onboarding and production requirements. Direct retrieval of the
full current documentation was incomplete. Treat version, OAuth scopes, interface
wording, consent and quotas as sandbox-verification gates rather than established
runtime configuration. No identity documents belong in fixtures. Sources:
[API catalogue entry][uscis], [official detailed entry][uscis-detail].

**Canada.** Official guidance routes requests to individual institutions and
separately points to summaries of released records. The ATIP landing guidance also
distinguishes IRCC application-record requests from other routes. Neither is proof
of a general transaction API. "Not identified" is a bounded search result, not a
claim that no government API exists. Sources: [request guidance][canada],
[ATIP landing guidance][atip].

**India.** The official FAQ describes central-authority scope, forms, payments,
applicant-specific status access and OTP controls. It is not a public API contract.
No general-purpose public request API was identified. Retain normal TLS validation:
legacy help text is not permission to disable certificate checks. Source:
[RTI Online FAQ][india].

## Explicit exclusions and rollout policy

Alaveteli deployments, including FYI, WhatDoTheyKnow and RightToKnow, remain in the
existing provider/jurisdiction work and are explicitly excluded here. Froide is
not Alaveteli. South Africa PAIA guidance and other regional portals may be future
research entries; they must not be silently folded into one of these six candidates.

These decisions do not claim that an adapter is implemented or operationally
approved. Before adoption, preserve provider identity, jurisdiction, source URLs,
retrieval metadata, redactions and raw-response digests; prove fixtures and bounded
read behaviour under the source-adapter track. Authentication, private case data,
submissions and payments require separate explicit scope. Never fall back from a
documented API to protected-page scraping. Demote affected integrations when
contracts, availability, identity requirements or terms change.

## Reproducible refresh and drift checks

The inventory records source URLs, review dates, confidence, access method and
per-platform next-review dates: **2026-12-14** for the first four candidates and
**2027-01-13** for Canada and India. `retrieved_page` means documentation was
read directly; `indexed_page` means indexed primary material was available but a
complete direct retrieval was not. Neither means live API verification.

Run the network-free completeness/integrity gate:

```sh
python scripts/verify_foi_platform_landscape.py
python -m pytest -q tests/test_foi_platform_landscape.py
```

By default the checker evaluates the historical snapshot at its own review date,
so old reproducible builds do not fail merely because time passes. To check current
freshness, supply an explicit date (and optionally fail the command when overdue):

```sh
python scripts/verify_foi_platform_landscape.py --as-of 2026-12-14 --fail-on-overdue
```

A later authorised source review can provide `--observations observations.json`:

```json
{
  "schema_version": "1.0.0",
  "observed_on": "2026-12-14",
  "platforms": [
    {"id": "muckrock", "status": "available", "contract": {"...": "full reviewed contract"}}
  ]
}
```

This abbreviated example is **not valid input**: a real observation must include
all six unique platforms and every contract field from the inventory. Availability
here concerns the supplied documentation/contract observation, not a live service
health attestation. Changed projections, deprecated sources or unavailable sources
produce drift and a nonzero exit. Missing, duplicate or malformed evidence fails
closed. The checker never fetches a URL, edits its baseline or authorises a provider.

`contract_sha256` is a deterministic fingerprint of the **reviewed semantic
projection**, not a downloaded OpenAPI/HTML byte digest, signature or proof that
an upstream source is authentic. Raw schema acquisition and actual remote
observations remain outstanding #195 phase-3 work and must use the authorised
source-adapter boundary. Re-review primary sources,
record uncertainty, update the projection and dates together, and run the tests
before changing a baseline. Do not advance review dates after failed retrieval.

[muckrock]: https://help.muckrock.com/19ef8892696381e88627c50e4ee90ed4
[muckrock-client]: https://python-muckrock.readthedocs.io/en/latest/requests.html
[fds]: https://fragdenstaat.de/hilfe/funktionen-der-plattform/entwicklerinnen/
[froide]: https://froide.readthedocs.io/en/latest/api/
[foia]: https://www.foia.gov/developer/
[agency]: https://www.foia.gov/developer/agency-api/
[uscis]: https://developer.uscis.gov/api/foia-request-and-status
[uscis-detail]: https://developer.uscis.gov/node/157
[canada]: https://www.canada.ca/en/treasury-board-secretariat/services/access-information-privacy/access-information/request-information/make-request.html
[atip]: https://atip-aiprp.apps.gc.ca/atip/
[india]: https://rtionline.gov.in/faq.php
