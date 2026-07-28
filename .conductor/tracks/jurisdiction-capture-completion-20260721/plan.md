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
  - [ ] Confirm bounded read-only discovery + request JSON/HTML/attachment capture for
    `uk-wdtk` and `ie-myrighttoknow` using the existing profile adapters, with explicit
    no-fallback profile assertions.
  - [ ] Record per-profile evidence bundle hashes and checkpoint resumability outcomes for
    handoff to `fyi-archive`.
  - [ ] Capture any authorization/policy blockers as explicit gates in `metadata.json`
    (do not proceed to unrestricted live capture).
- [ ] Rescan and implement official Alaveteli deployments one jurisdiction at a time.
- [ ] Verify/add Germany, Spain and Ireland platform support.
- [ ] Implement explicitly bounded non-Alaveteli adapters for Canada federal, US federal and South Africa.
- [ ] At each increment, stop before live capture without operator authorization and record archive/NLP/process dependencies.
