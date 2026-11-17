# Non-Alaveteli FOI platform landscape

Reviewed: **2026-11-17**. Scope: a research and prioritisation snapshot, **not
provider enablement, legal advice, or evidence of a successful live API call**.
The machine-readable semantic record is
[`foi-platform-landscape-v1.json`](../conformance/providers/foi-platform-landscape-v1.json).
The separate [source-byte baseline](../conformance/providers/foi-source-baseline-v1.json)
records six public documentation captures without publishing their raw bodies.

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
| 4 | USCIS FOIA/PA, United States | **No-go for the default fleet; defer.** Case-specific identity/consent workflow is not public corpus discovery. | Review the captured 1.3.0 schema and required interface text offline; no real subjects, case creation or polling. | Documented; 30 days |
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
The raw capture is the [official client documentation source][muckrock-source],
not deployed OpenAPI or a successful endpoint observation. Its bytes were checked
against the pinned Git blob. Direct deployed schema retrieval remained
unavailable. The client repository's [LICENSE][muckrock-license] says GPL-3.0 while
its [package metadata][muckrock-package] says MIT; preserve that discrepancy before
redistributing source or adding the client as a dependency.

**FragDenStaat / Froide.** Official developer material describes an API and OAuth
application registration; Froide documents the API-v1 request/public-body model.
The [framework documentation source][froide-source] was retrieved and checked
against its pinned Git blob. It describes Basic Authentication and `/api/v1/docs/`,
while the same revision's [settings][froide-settings] select OAuth2/session
authentication and its [routes][froide-routes] expose
`/api/v1/schema/swagger-ui/`. This known disagreement keeps confidence provisional.
The [operator developer page][fds] remained unavailable during this refresh;
framework source does not verify a deployed schema, a numeric throttle limit or
endpoint availability. Do not copy older auth prose into a deployment. The
[rendered framework documentation][froide] is a separate source surface.

**FOIA.gov — United States federal FOIA portal.** The public developer
documentation describes API-key-protected agency components and annual-report XML. The separate agency specification describes
portal-to-agency delivery with agency-specific security controls; it is not an
unrestricted third-party filing endpoint. The reviewed draft leaves rate limits
unspecified. Sources: [developer resources][foia], [agency specification][agency].
The monitor captures the public developer page. An upstream schema contains a
published testing credential, which is neither used nor included in this
repository; no access permission is inferred from its publication.

**USCIS.** The directly retrieved [OpenAPI document][uscis-schema] identifies API
**1.3.0** in an OpenAPI 3.0.1 schema, with `POST /case`, `GET /case-status` and
OAuth client credentials. The [catalogue entry][uscis] announces birth-country
changes in this version. The [sandbox guide][uscis-sandbox] states **3,500 requests
per day and 5 transactions per second** for the FOIA sandbox; production quotas
are supplied after approval. These are documentation facts, not an approved traffic
budget. Account onboarding, granted scopes, required interface wording and consent
remain separate gates. The raw schema includes identity-shaped examples that are
not independently verified synthetic; no such examples or identity documents
belong in public fixtures. Confidence is documented; the decision remains defer.

**Canada.** Official guidance routes requests to individual institutions and
separately points to summaries of released records. The refreshed
[request guidance][canada] links to `atip-aiprp.tbs-sct.gc.ca` and invites account
creation for future access. The earlier [ATIP landing guidance][atip] is retained
as routing context, including the distinct IRCC route, but was not requalified by
this refresh. None of these routes establishes a general transaction API.
"Not identified" is a bounded search result, not a claim that no government API
exists. The raw capture covers the public guidance page only.

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
per-platform next-review dates: **2026-12-17** for the first four candidates and
**2027-01-16** for Canada and India. `retrieved_page` means the cited primary page
or documentation source was read directly; `indexed_page` means indexed primary
material was available but a complete direct retrieval was not. Neither means
live API verification. An unsuccessful deployment fetch does not gain a new
retrieval date from a successful repository-source review.

Run the network-free completeness/integrity gate:

```sh
python scripts/verify_foi_platform_landscape.py
python -m pytest -q tests/test_foi_platform_landscape.py
```

By default the checker evaluates the historical snapshot at its own review date,
so old reproducible builds do not fail merely because time passes. To check current
freshness, supply an explicit date (and optionally fail the command when overdue):

```sh
python scripts/verify_foi_platform_landscape.py --as-of 2026-12-17 --fail-on-overdue
```

A later authorised source review can provide `--observations observations.json`:

```json
{
  "schema_version": "1.0.0",
  "observed_on": "2026-12-17",
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
projection**. It is separate from the raw documentation hashes below. Neither
kind of plaintext hash is an authentic signature or independent proof of origin.

## Six-source byte monitor

The [source-byte baseline](../conformance/providers/foi-source-baseline-v1.json)
records exact source URLs, capture timestamps, body sizes and SHA-256 fingerprints.
It also preserves source type and retention limitations. The selected sources are:

| Source ID | Candidate | Observed surface | Qualification limit |
|---|---|---|---|
| `muckrock_client` | MuckRock | Official `python-muckrock` documentation source | Client source, not deployed schema; licensing discrepancy remains. |
| `froide_api` | FragDenStaat / Froide | Official Froide documentation source | Framework prose disagrees with current source; deployment remains unverified. |
| `foia_developer` | FOIA.gov | Public developer page | Documents catalogue access; no API key or production call. |
| `uscis_openapi` | USCIS | Published FOIA OpenAPI YAML | Schema version 1.3.0; no case or identity query. |
| `canada_guidance` | Canada ATIP | Official request guidance | Public guidance, not a transaction API contract. |
| `india_faq` | India RTI | Official portal FAQ | Public guidance, not a transaction API contract. |

The two repository sources are monitored at their mutable `main` documentation
paths; the evidence register separately records the immutable revisions reviewed.
Changes to other source files, new documentation URLs or deployed behaviour are
outside this six-document sensor and still require a source review.
In particular, USCIS may publish a replacement schema at another dated URL;
the scheduled catalogue review must discover that change before the allowlist
and reviewed baseline can be updated.

Validate only the committed metadata, without networking or raw captures:

```sh
python scripts/verify_foi_source_snapshots.py
python -m pytest -q tests/test_foi_source_snapshots.py
```

The default result explicitly reports metadata-only validation. It cannot verify
raw bytes or claim a new remote observation. To collect an operator-authorised
documentation observation, choose a private directory outside the checkout and
replace `<ID>` with one of the six source IDs:

```sh
cargo run --locked -p fyi-core --example observe_platform_docs -- --source <ID> --output /private/captures/<ID> --live
```

Create the private parent directory first; each source output directory must be
new. A failed capture can leave an incomplete directory without
`observation.json`; keep it separate from successful captures and choose a fresh
destination for an authorised retry. The existing MSRV workflow also retains a
compiled Linux observer with its source revision and checksum for seven days;
building that artifact makes no live documentation request.

The example uses the guarded source-adapter transport and requires `--live` for
the public-documentation GET. It does not register accounts, accept arbitrary
URLs, acquire credentials, invoke a case API or enable provider writes. Tests
use injected transports; live observations never run in CI.

After capturing all six sources into separate folders, compare the raw bodies,
receipts and observations offline:

```sh
python scripts/verify_foi_source_snapshots.py --observations /private/captures
```

Missing or inconsistent evidence fails closed. A changed byte digest requires
review; it does not establish an API break, deprecation or a changed legal regime.
Unchanged bytes prove only consistency with the selected captured document, not
that its claims are correct or current. Adjacent Canada and India captures changed
despite equal byte lengths, demonstrating that page chrome can affect hashes.
Neither verifier rewrites its baseline or advances the semantic review dates.
Review the affected primary material, record uncertainty, update the projection
and dates together, and run the offline checks before committing a new baseline.
Byte verification is not a cadence check: also run the landscape checker with
the current `--as-of` date and `--fail-on-overdue` when checking whether a new
review is due.

### Raw-body retention and attribution

Raw captures remain in the operator's private evidence bundle; public repository
records contain reviewed metadata and digests. Do not commit full pages, response
headers, credentials, cookies, identity-shaped schema examples or local evidence
locations. USCIS pages contain generated form identifiers, and Canada's guidance
includes page infrastructure values. The [Canada.ca terms][canada-terms] distinguish
non-commercial reproduction from commercial redistribution; the [India policy][india-policy]
requires accurate reproduction and source attribution and excludes third-party
material. [Froide's licence][froide-license] and [FOIA.gov's licence][foia-license]
also need to accompany any separately permitted source redistribution. MuckRock's
licence discrepancy and the absence of an explicit USCIS schema licence remain
recorded; this monitoring workflow grants no redistribution rights.

[muckrock]: https://help.muckrock.com/19ef8892696381e88627c50e4ee90ed4
[muckrock-client]: https://python-muckrock.readthedocs.io/en/latest/requests.html
[muckrock-source]: https://github.com/MuckRock/python-muckrock/blob/ba065fb10240b0293c08d961e04406d6860c29f7/docs/requests.rst
[muckrock-license]: https://github.com/MuckRock/python-muckrock/blob/ba065fb10240b0293c08d961e04406d6860c29f7/LICENSE
[muckrock-package]: https://github.com/MuckRock/python-muckrock/blob/ba065fb10240b0293c08d961e04406d6860c29f7/pyproject.toml
[fds]: https://fragdenstaat.de/hilfe/funktionen-der-plattform/entwicklerinnen/
[froide]: https://froide.readthedocs.io/en/latest/api/
[froide-source]: https://github.com/okfde/froide/blob/b80fbe9269d3744b38a123363bbdd7d830d1bcf9/docs/api.rst
[froide-settings]: https://github.com/okfde/froide/blob/b80fbe9269d3744b38a123363bbdd7d830d1bcf9/froide/settings.py
[froide-routes]: https://github.com/okfde/froide/blob/b80fbe9269d3744b38a123363bbdd7d830d1bcf9/froide/urls.py
[froide-license]: https://github.com/okfde/froide/blob/b80fbe9269d3744b38a123363bbdd7d830d1bcf9/LICENSE.txt
[foia]: https://www.foia.gov/developer/
[agency]: https://www.foia.gov/developer/agency-api/
[foia-license]: https://github.com/usdoj/foia.gov/blob/dc020ffc0b6eb7380f430f62c00557ff37a667cc/LICENSE.md
[uscis]: https://developer.uscis.gov/api/foia-request-and-status
[uscis-schema]: https://developer.uscis.gov/sites/default/files/2026-10/swagger.yaml
[uscis-sandbox]: https://developer.uscis.gov/get-started/sandbox
[canada]: https://www.canada.ca/en/treasury-board-secretariat/services/access-information-privacy/access-information/request-information/make-request.html
[atip]: https://atip-aiprp.apps.gc.ca/atip/
[canada-terms]: https://www.canada.ca/en/transparency/terms.html
[india]: https://rtionline.gov.in/faq.php
[india-policy]: https://rtionline.gov.in/Policies.php
