# Plan: Alaveteli MCP coverage and requester journeys

Keep this track open until all phases and the existing archive gates have
evidence. Execute each code task test-first; use the named existing tracks for
source, MCP harness and governed-write ownership. Never replace a failing
profile with a fixture that merely mirrors its implementation.

## Phase 1 — Source-backed MCP fixture

- [ ] Task: Pin the stock controller/routes and retain the existing synthetic
  `tests/fixtures/remote_mcp/upstream-v2.json` as its own provider profile.
  - [ ] Add a stock fixture with five explicit request/read/correspondence/
    attachment/state cases and synthetic, non-secret input placeholders.
  - [ ] Add negative controls for route, `k`, form JSON names, status and result
    fields; fail if a case disappears or the pinned reference drifts.
- [ ] Task: Add an explicit source-profile choice to the existing MCP policy.
  - [ ] First demonstrate the five stock fixture cases fail through the current
    selected handlers; then implement the matching source operations.
  - [ ] Preserve the old profile and its SDK tests, guarded confirmation,
    idempotency journal, authorization, size bounds and failure privacy.
  - [ ] Re-run the real MCP handler/SDK contracts with a socket-free injected
    source transport and prove that unrelated provider cases do not change.
- [ ] Task: Phase verification and checkpoint; record exact focused commands,
  fixture revision, executed binary/transport and untested deployment scope.

## Phase 2 — Authority read, consumer and receipt

- [~] Task: Integrate the existing pinned authority-read behavior into the
  existing `fetch-request-page` consumer and request-evidence path.
  - [ ] Write socket-free failing cases for the shared read profile, explicit
    credential selection, raw/result/receipt digest linkage and key redaction.
  - [ ] Reuse the existing bounded transport, request parser, private immutable
    evidence writer, limiter and v1 receipt verifier; do not issue live reads.
  - [ ] Test 200, 403, redirects, malformed/mismatched responses, incomplete
    exchange, unsafe paths, key exposure, tampering and no-clobber behavior.
  - [ ] Update the inventory and operator guide with the command, sensitive
    evidence boundary, tested scope and selected source profile.
- [ ] Task: Phase verification and checkpoint; run the focused Rust tests,
  formatting, Clippy and the Conductor/CLI-surface contract checks.

## Phase 3 — Requester account and session

- [ ] Task: Prepare a pinned, isolated upstream test environment using the
  project's existing upstream setup where possible; seed only fictional users,
  authority and request content, and intercept outbound mail.
  - [ ] Record exact upstream revision, runtime, commands, reset method and
    zero-external-network proof; no shared service or real mailbox is used.
- [ ] Task: Cover signup and confirmation.
  - [ ] Capture the pinned signup form, CSRF/session and confirmation-token
    behavior; write failing valid, duplicate, invalid, expired and replay cases.
  - [ ] Implement the minimum supported requester client flow and assert the
    fictional account is unusable until the intercepted confirmation succeeds.
- [ ] Task: Cover sign-in, challenge and sign-out.
  - [ ] Test valid and invalid passwords, unconfirmed/disabled accounts,
    expiry, session isolation, CSRF, optional MFA and logout invalidation.
  - [ ] Implement user-mediated challenges and safe session storage/redaction;
    stop on unsupported provider challenge rather than bypassing it.
- [ ] Task: Phase verification and checkpoint using focused offline contracts
  plus the isolated upstream environment; record what actually ran.

## Phase 4 — Request and follow-up journey

- [ ] Task: Cover an authenticated new request.
  - [ ] Test authority selection, form/CSRF, review, explicit send authorization,
    validation failure and duplicate/retry prevention with fictional data.
  - [ ] Assert exactly one intercepted outgoing message and the created request
    and event state; an accepted HTTP response alone is insufficient.
- [ ] Task: Cover response, follow-up and classification.
  - [ ] Inject a synthetic authority reply, including an attachment, via the
    isolated mail path; assert request association and readable response bytes.
  - [ ] Test and implement preview and explicitly authorized follow-up; assert
    one intercepted message, correct thread, state and retained attachment.
  - [ ] Cover rejected sessions, changed CSRF, denied/unsupported actions,
    private errors, failed delivery and retry/recovery without duplicate sends.
- [ ] Task: Execute the complete fictional journey from registration through
  logout and subsequent access denial, with independent state and mail oracles.
- [ ] Task: Phase verification and checkpoint; update the coverage matrix and
  retain limits for each source version and any unexecuted provider variants.

## Phase 5 — Archive integration and cutover

- [ ] Task: Complete source-adapter #310 receipt coverage, #311 CDX discovery,
  #312 replay and WARC/WACZ evidence including response and attachment bytes.
- [ ] Task: Pin a qualified release in `fyi-archive`, replace remaining direct
  source transports, and run the existing static and behavioral boundary checks.
- [ ] Task: Compare old/new offline outputs and NZ shadow queues, captures,
  events, attachments, revisions, receipts and checkpoints; prove rollback.
- [ ] Task: Obtain session authorization for each exact hosted read-only smoke;
  run the bounded Alaveteli and Internet Archive checks and retain their evidence.
- [ ] Task: Re-read the source-adapter gates and downstream results. Once every
  prerequisite passes, record the exact commits/runs and change
  `archive_cutover_allowed` to `true` in its existing metadata. If any gate
  remains incomplete, retain `false` and identify the failing gate.
- [ ] Task: Phase verification and checkpoint; reconcile the registry and keep
  source, MCP, requester and downstream evidence linked.
