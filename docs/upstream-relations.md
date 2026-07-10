# Upstream relations (Alaveteli / instance operators)

This document records how **fyi-cli** relates to the [Alaveteli](https://alaveteli.org/)
project (mySociety) and to individual FOI/OIA instance operators. It is documentation and
outreach preparation only: it does not itself generate live bulk traffic.

## Relationship overview

| Role | Entity | Notes |
|------|--------|--------|
| Upstream platform | [mysociety/alaveteli](https://github.com/mysociety/alaveteli) | Shared FOI CMS / API surface fyi-cli speaks to |
| Primary NZ instance | [fyi.org.nz](https://fyi.org.nz) | First-class, `status = supported` in catalog |
| Additional instances | RightToKnow (AU), WhatDoTheyKnow (UK), MyRightToKnow (IE), Ma Da Da (FR), Tu Derecho a Saber (ES), FragDenStaat (DE) | Embedded in `crates/fyi-core/instances.toml` |

fyi-cli is an **independent, open-source multi-instance client and archival tool**. It is not
affiliated with mySociety or any single instance operator. We aim to be good citizens of
shared public infrastructure.

## Capability summary (catalog snapshot)

Derived from the embedded instance catalog (`crates/fyi-core/instances.toml`). Values are
configuration claims used for client behaviour; live endpoint parity may still vary by deploy.

| Instance ID | Base URL | Locale | Catalog status | Read | Write | Feeds | Search | Batch | Prefilled URL | Health |
|-------------|----------|--------|----------------|------|-------|-------|--------|-------|---------------|--------|
| `nz-fyi` | https://fyi.org.nz | en-NZ | supported | yes | yes | yes | yes | yes | yes | yes |
| `au-rtk` | https://www.righttoknow.org.au | en-AU | experimental | yes | yes | yes | yes | no | yes | yes |
| `uk-wdtk` | https://www.whatdotheyknow.com | en-GB | experimental | yes | yes | yes | yes | no | yes | yes |
| `ie-myrighttoknow` | https://www.myrighttoknow.org | en-IE | experimental | yes | yes | yes | yes | no | yes | yes |
| `fr-cada` | https://www.madada.fr | fr-FR | community | yes | yes | yes | yes | no | yes | yes |
| `es-tdas` | https://www.tuderechoasaber.es | es-ES | community | yes | yes | yes | yes | no | yes | yes |
| `de-fds` | https://fragdenstaat.de | de-DE | community | yes | yes | yes | yes | no | yes | yes |

### API documentation gaps (not yet filed upstream)

Captured during multi-jurisdiction client work; not yet filed as upstream issues (timing left to
the maintainer):

1. **Per-instance capability drift** — Alaveteli docs describe a general JSON API surface, but
   deployed instances differ in which endpoints and feed formats are enabled. A single
   capability matrix in upstream docs would reduce client guesswork.
2. **Rate-limit signalling** — Behaviour on `429` / Retry-After is not consistently documented
   across instances; clients currently treat any `429` as a signal to back off.
3. **Health / version endpoints** — Availability of health or API-version probes varies; a
   recommended probe path would help multi-instance clients.

## Etiquette norms enforced by fyi-cli

These norms are expected of bulk discovery, capture, and sync workloads:

1. **Identify yourself** — Send a cryptographic-aligned `User-Agent` that names the tool,
   version, and build fingerprint, plus an **opt-in** administrative contact when provided
   (`FYI_ADMIN_CONTACT` / Rust `ClientIdentity`). See
   [`docs/agent-network-middleware.md`](agent-network-middleware.md).
2. **Respect robots.txt** — Discovery paths check robots rules before walking public feeds.
3. **Back off on pressure** — Parse `RateLimit-*` and `Retry-After` when present; treat `429`
   and transient `5xx` as backoff signals with exponential backoff; prefer checkpoints and
   small date windows over long uninterruptible crawls.
4. **Adaptive pacing & guardrails** — Throttle concurrency/velocity under low remaining quota
   or latency spikes; enforce max requests / bytes / runtime per run to prevent runaway loops.
5. **Shared rate limiting** — When multiple workers share a DB, use the cross-worker limiter
   (`fyi rate-limit-status`) so aggregate load stays polite.
6. **Local cache first** — Filesystem response cache avoids redundant remote GETs for unchanged
   resources.
7. **Opt-in live tests only** — Live smoke tests require explicit environment flags
   (e.g. `FYI_LIVE_SMOKE=1`); CI and default developer workflows stay offline-safe.
8. **Tor transparency** — If traffic is routed via Tor/proxy, operators should still be able
   to identify the tool via User-Agent; do not use Tor to hide abusive load.
9. **No silent bulk archiving at scale** — Large historical seeds should be coordinated with
   instance operators and the ethics guidance in sibling archive work.
10. **Auditable agent decisions** — Bulk runs can emit local JSONL traces (Langfuse/Braintrust-
    compatible schema) without shipping secrets.

## Official third-party client listing

As of 2026-07-09:

- Alaveteli’s public project materials focus on deploying and running instances, not on a
  curated third-party client registry.
- The practical path for visibility remains: open a polite issue/discussion on
  `mysociety/alaveteli` (or Discourse if preferred by maintainers) introducing the client and
  asking whether an official “related tools” section would be welcome.

A ready-to-post draft is in
[`docs/outreach/alaveteli-project-intro.md`](outreach/alaveteli-project-intro.md).

## Contribution / outreach log

| Date | Action | Link / location |
|------|--------|-----------------|
| 2026-07-08 | Multi-jurisdiction FOI foundation merged | PR [#126](https://github.com/edithatogo/fyi-cli/pull/126) |
| 2026-07-09 | Upstream relations doc + outreach draft created | this file; `docs/outreach/alaveteli-project-intro.md` |
| (pending) | Maintainer posts outreach to Alaveteli | draft ready; posting is intentional human step |

## Related Conductor track

- `.conductor/tracks/upstream-alaveteli-engagement/`
