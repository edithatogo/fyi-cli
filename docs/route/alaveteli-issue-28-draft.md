# Draft: paired Alaveteli capability contract

This is a fork-local server proposal draft for Alaveteli issue #28. It is not
an upstream submission.

The server side should publish a versioned capability document compatible with
`fyi-endorsed-client/v1`, enforce allowlisted scoped credentials, apply request
and byte quotas, expose revocation/expiry and an emergency kill switch, and
emit operator audit/metrics. Bulk export must be separately authorized and
bounded; ordinary API retrieval must never be used as a fallback for an
unauthorized export.

The server proposal must add its own conformance fixtures, authorization and
disablement tests, privacy checks, and rollback evidence before the fyi-cli
maintainer package can pass its evidence gate.
