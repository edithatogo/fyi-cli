# Government API evaluation

Reviewed 2026-07-13 against the official developer documentation.

## FOIA.gov

Decision: keep this as a separately governed, read-only discovery candidate;
do not add it to the general provider fleet yet.

The public Agency Component API is documented at
`https://api.foia.gov/api/agency_components`. It follows JSON:API conventions,
supports agency/component metadata and request-form discovery, and requires an
API key. The official Swagger page states that API versioning is not supported.
The separate Agency API specification is a decentralized, agency-operated
request-ingestion interface whose documented method is `POST`; it is outside
the read-only pilot boundary.

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

The official API documentation describes OAuth 2.0 client credentials, a
registered developer app, a sandbox endpoint, and a case-creation `POST`
operation. The required application experience includes identity, consent,
attestation, signature/notarization, account ownership, and sensitive Alien
File/Privacy Act material. Production access additionally requires sandbox
implementation, at least five consecutive calendar days of traffic, and
support approval.

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
this repository or its CI.

## Canada ATIP Online Request portal

Decision: keep this as a discovery-only or manual-route target; do not treat it
as a machine-readable capture source.

The official portal is available at `https://atip-aiprp.apps.gc.ca/atip/` and
is an online request form rather than a stable public API for listing requests,
responses, attachments, or timelines. That means:

- the portal may be recorded as a stable official target identifier;
- unattended scraping, browser automation, or form submission is out of scope;
- no public request-response-attachment feed is available for archive handoff;
- any future automation would require a separately justified lawful
  machine-readable source, not opportunistic form driving.

Canada should therefore remain blocked for public capture completion in this
repository until a documented machine-readable public surface exists.

## South Africa PAIA guidance

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
