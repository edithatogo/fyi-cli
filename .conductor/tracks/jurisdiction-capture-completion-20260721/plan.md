# Plan

- [ ] Foundation capture audit and archive handoff.
- [~] Australian pilot, contrast and remaining-jurisdiction capture increments.
  - [x] Audit generic `au-rtk` capture support for AU-CTH and AU-NSW,
    including registry selection, JSON/HTML/attachment capture, bounded retries
    and caps, WARC/WACZ output, resource hashes, and the non-executing archive
    handoff boundary. (`3a87899`; audit SHA-256
    `d1af66697c173cfafd8de2dfd00a8d83f3be5a5401ece5fae2bac1e67c723d71`)
  - [ ] Exercise live capture only under a separately authorized non-empty
    source run, then classify jurisdiction from recorded authority evidence.
- [ ] UK and European English increments.
- [~] UK + Ireland bounded capture increment (next in sequence).
  - [x] Confirm bounded read-only discovery + request JSON/HTML/attachment capture for
    `uk-wdtk` and `ie-myrighttoknow` using the existing profile adapters, with explicit
    no-fallback profile assertions. (`uk_ie_capture_capability_audit.json`)
  - [x] Record per-profile evidence bundle hashes and checkpoint resumability outcomes for
    handoff to `fyi-archive`. (SHA-256:
    `425865653fa64a7c352b59f8272b0deabed1089be5f9a60f4768e82fc5ba7ade`)
  - [x] Capture authorization/policy blockers as explicit gates in `metadata.json`
    (do not proceed to unrestricted live capture).
- [~] Rescan and implement official Alaveteli deployments one jurisdiction at a time.
  - [x] Confirm bounded read-only discovery + request JSON/HTML/attachment capture for
    `fr-cada` using the explicit profile selector with no cross-profile fallback.
    (`fr_capture_capability_audit.json`)
  - [x] Preserve an explicit external authorization gate for non-empty France live capture prior
    to archive handoff.
- [x] Verify/add Germany, Spain and Ireland platform support.
  - [x] Confirm bounded read-only discovery + request JSON/HTML/attachment capture for
    `de-fds` and `es-tdas` using explicit profile selectors with no cross-profile fallback.
    (`de_es_capture_capability_audit.json`)
  - [x] Keep Ireland (`ie-myrighttoknow`) coverage under the UK+IE bounded capture increment.
  - [x] Preserve explicit external authorization gates for non-empty live capture runs prior to
    archive handoff.
- [x] Implement explicitly bounded non-Alaveteli adapters for Canada federal, US federal and South Africa.
  - [x] Record US federal FOIA.gov as a bounded discovery-only target with explicit blocked
    request/attachment/timeline capture status and no POST fallback assumptions.
    (`us_federal_capture_capability_audit.json`)
  - [x] Record Canada federal as a discovery-only or manual-route target with stable blockers.
    (`canada_federal_capture_capability_audit.json`)
  - [x] Record South Africa as a bounded unsupported/blocked target with stable blockers.
    (`south_africa_capture_capability_audit.json`)
- [ ] At each increment, stop before live capture without operator authorization and record archive/NLP/process dependencies.
