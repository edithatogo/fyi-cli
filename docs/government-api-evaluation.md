# Government API evaluation

FOIA.gov, USCIS, Canada and India documentation reviewed **2026-11-17**.
The [landscape](foi-platform-landscape.md) records rankings, source confidence and
refresh cadence; its [source-byte baseline](../conformance/providers/foi-source-baseline-v1.json)
records exact documentation captures. This is documentation evidence, not
production API qualification. The separate South Africa assessment below retains
its 2026-07-13 review boundary.

## FOIA.gov

Decision: keep this as a separately governed, read-only discovery candidate;
do not add it to the general provider fleet yet.

The official [developer guide](https://www.foia.gov/developer/) documents the
public Agency Component API at
`https://api.foia.gov/api/agency_components`. It follows JSON:API conventions,
supports agency/component metadata and request-form discovery, and requires an
API key. The separate [Agency API specification](https://www.foia.gov/developer/agency-api/)
is a decentralized, agency-operated
request-ingestion interface whose documented method is `POST`; it is outside
the read-only pilot boundary.

The monitor captures public developer documentation without an API key. An
upstream schema contains a published testing credential; its publication is not
permission to use or redistribute it, and it is excluded from this repository.
The reviewed agency draft does not set rate limits; a future catalogue adapter
must obtain the applicable key quota separately. No general public request,
response, attachment or timeline capture service is established by this review.

Before implementation, an operator must provide an approved API key and confirm
that the key may be used for bounded catalog reads. The future adapter must:

- expose only agency/component metadata and request-form reads;
- use a fixed HTTPS allowlist for `api.foia.gov`;
- cap pages, bytes, and request count;
- fingerprint the JSON:API shape and fail closed on drift;
- never call the Agency API `POST` surface or submit requests;
- preserve source URLs and retrieval timestamps.

## USCIS FOIA Request and Status API

Decision: do not implement in the current read-only provider pilot.

The directly retrieved [official OpenAPI document](https://developer.uscis.gov/sites/default/files/2026-10/swagger.yaml)
is OpenAPI 3.0.1 with API version **1.3.0**. It describes OAuth 2.0 client
credentials, `POST /case` and `GET /case-status`; the latter is case-specific,
not public-corpus discovery. The [developer entry](https://developer.uscis.gov/api/foia-request-and-status)
announces changes to the birth-country field. A registered developer app and
sandbox onboarding remain prerequisites; documented scope names are not evidence
that any particular application has received them.

The required application experience includes identity, consent,
attestation, signature/notarization, account ownership, and sensitive Alien
File/Privacy Act material. Production access additionally requires sandbox
implementation, at least five consecutive calendar days of traffic, and
support approval.

The [sandbox guide](https://developer.uscis.gov/get-started/sandbox) states a FOIA
sandbox quota of **3,500 requests/day and 5 transactions/second**. Production
quotas and throughput are provided when access is granted. Keep those separate
from the unrelated Case Status API's limits and from the guidance about how often
to check an individual FOIA case. No traffic allowance is activated here.

The following checklist is required before any future, separately approved
USCIS work:

1. Register the developer account and sandbox app with the operator's legal
   entity and named data owner.
2. Obtain written approval for the exact OAuth scopes and data retention policy.
3. Validate sandbox-only traffic for at least five consecutive calendar days.
4. Implement the mandatory identity, consent, attestation, signature, account,
   and duplicate-request disclosures in the user experience.
5. Threat-model PII, document uploads, access tokens, audit records, and
   deletion/retention before code review.
6. Keep case creation and all identity-sensitive operations disabled until a
   separate provider contract and explicit operator approval exist.

No USCIS credentials, tokens, request payloads, or production calls belong in
this repository or its CI. The public schema contains identity-shaped examples;
their placement under `example` keys does not independently establish that every
literal is synthetic. Retain raw bytes only in the private operator evidence
bundle and use reviewed metadata/digests in the repository.

## Canada ATIP Online Request portal

Decision: keep this as a discovery-only or manual-route target; do not treat it
as a machine-readable capture source.

The refreshed [official request guidance](https://www.canada.ca/en/treasury-board-secretariat/services/access-information-privacy/access-information/request-information/make-request.html)
routes users to individual institutions and links to `atip-aiprp.tbs-sct.gc.ca`.
It separately recommends checking summaries of released requests. The earlier
`https://atip-aiprp.apps.gc.ca/atip/` route remains recorded as historical routing
context, not requalified by this refresh. The guidance does not establish a
stable public API for listing requests, responses, attachments, or timelines.
That means:

- the portal may be recorded as a stable official target identifier;
- unattended scraping, browser automation, or form submission is out of scope;
- no public request-response-attachment feed was established for archive handoff
  by the reviewed sources;
- any future automation would require a separately justified lawful
  machine-readable source, not opportunistic form driving.

Canada should therefore remain blocked for public capture completion in this
repository until a documented machine-readable public surface exists.
This bounded review does not prove that no government API exists. The monitor
captures the public guidance document only; it does not automate either portal.
[Canada.ca terms](https://www.canada.ca/en/transparency/terms.html) distinguish
non-commercial reproduction from commercial redistribution, so the repository
stores public metadata and digests rather than republishing the page.

## India RTI Online

Decision: manual routing only; do not implement portal transactions or private
status capture.

The [official FAQ](https://rtionline.gov.in/faq.php) describes requests to central
ministries and other listed central authorities, forms and payment handling,
and applicant-specific status access protected by OTP. It does not establish a
general-purpose public transaction API. The absence of a suitable API in this
review is not proof of nonexistence. No submission, payment, CAPTCHA, OTP or
account workflow is enabled by the documentation monitor.

Retain standard TLS certificate verification despite the FAQ's legacy advice
about certificate errors. The [site policy](https://rtionline.gov.in/Policies.php)
permits accurate reproduction with source attribution and excludes third-party
material; the monitoring record still publishes only reviewed metadata/digests.

## Documentation refresh boundary

The [six-source monitor](foi-platform-landscape.md#six-source-byte-monitor)
observes allowlisted public documents through the guarded source-adapter
transport. The offline verifier requires complete raw bodies, receipts and
observations to compare a new capture. Its default metadata-only check does not
verify those raw bytes. A source hash change requires human review and cannot
by itself establish deprecation, a semantic API break or safe provider rollout.

## South Africa PAIA guidance

Retained assessment from 2026-07-13; this target is outside the six-candidate
refresh and source-byte monitor.

Decision: keep this as a guidance-only or manual-route target; do not treat it
as a machine-readable capture source.

The Information Regulator publishes PAIA guidance at
`https://inforegulator.org.za/paia/`. The public surface is a guidance/manual
page rather than a stable public API for listing requests, responses,
attachments, or timelines. That means:

- the guidance page may be recorded as a stable official target identifier;
- unattended scraping, browser automation, or submission driving is out of
  scope;
- no public request-response-attachment feed is available for archive handoff;
- any future automation would require a separately justified lawful
  machine-readable public source.

South Africa should therefore remain blocked for public capture completion in
this repository until a documented machine-readable public surface exists.
