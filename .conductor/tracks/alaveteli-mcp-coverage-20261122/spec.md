# Alaveteli MCP coverage and requester journeys

## Outcome

Replace the assumption that a synthetic MCP server represents Alaveteli with
source-versioned, executable contracts. Provide the ordinary requester journey
and the separately scoped authority disclosure journey, and integrate the
bounded authority read with a consumer and verifiable acquisition receipt.
Complete the existing source-adapter acceptance path to archive cutover.

## Requirements

1. Retain the existing synthetic MCP profile and its regression evidence. Add
   an explicitly selected stock-upstream profile, pinned to Alaveteli commit
   `3966415f0b2a9f445fa3bb25c06eaeb75ded6609`, with correct methods, routes,
   key handling, form fields, attachment rules and response/error shapes.
   Provider-specific behavior must be identified, not silently substituted.
2. Exercise the actual Rust MCP handlers and the selected source profile using
   socket-free, deterministic transports. Cover success and failures, missing or
   wrong authority key, malformed results, authorization, multipart response
   attachments, update preconditions, budgets and secret redaction. Preserve
   existing confirmation, journal, replay and attachment safeguards.
3. Implement a requester session journey for a selected pinned Alaveteli source:
   registration, email confirmation, login, logout, new request, follow-up and
   response/status readback. Include invalid, expired, challenged and denied
   cases. Model MFA and site restrictions as explicit user-mediated outcomes;
   do not bypass CAPTCHA, MFA or provider controls.
4. Verify end to end in an isolated local upstream environment with synthetic
   users, public bodies and intercepted mail. Assert state and delivered mail,
   not just HTTP success. Automated tests use in-process Rack or process IPC
   and mock transports without TCP/UDP network access, including loopback.
   A separate hosted smoke is operator-authorized and is never a CI test.
5. Expose the existing pinned authority read through an explicit consumer and
   the existing immutable raw/result/receipt path. Never accept an authority
   key in an argument, trace, receipt, fixture, log or error. Mark raw authority
   responses as sensitive; retain their exact bytes only in private outputs.
6. Preserve `archive_cutover_allowed=false` until the existing #310/#311/#312
   source evidence, release pin, downstream parity, bounded authorized hosted
   smokes, NZ shadow parity and rollback checks are satisfied. Then record their
   exact evidence and update the flag in the existing source-adapter track.

## Acceptance

- Five stock reference cases pass the source-backed contract comparison and
  the actual selected MCP path; historical synthetic cases still pass.
- A fictional requester registers, confirms, signs in, submits a request,
  receives an intercepted authority response, follows up, reads the final
  conversation and signs out; negative and challenge paths fail safely.
- The authority-read consumer produces a verified receipt binding the original
  response bytes and normalized result; rejected, incomplete and tampered
  cases cannot produce a false success or expose credentials.
- The workflow map distinguishes requester submission, authority email/web
  responses and the experimental authority import API, with citations and
  provider-specific unknowns.
- Cutover is enabled only after the existing archive integration gates have
  independently passed and their checkpoints are recorded.

## Boundaries

No real account, request, follow-up, response or form submission is authorized
by this track. Any hosted smoke requires the exact action to be authorized in
the session. The legacy Python package remains a reference, not the target
implementation. Public FAQ behavior does not prove a deployed API version.
