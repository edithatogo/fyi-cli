# Specification: upstream-alaveteli-engagement

## Overview
fyi-cli is evolving from an NZ-only client into a multi-jurisdiction Alaveteli/FOI client
covering instances such as FYI.org.nz, RightToKnow.org.au, WhatDoTheyKnow.com,
MyRightToKnow.org (Ireland), Ma Da Da / CADA (France), Tu Derecho a Saber (Spain), and
FragDenStaat (Germany). This track establishes a respectful, sustainable relationship with the
upstream Alaveteli project (github.com/mysociety/alaveteli) and the individual instance
operators, rather than treating them purely as silent API endpoints.

## Functional Requirements
1. **Project introduction:** Draft an outreach issue/discussion for the Alaveteli project (and,
   where appropriate, direct contact with instance operators) introducing fyi-cli as a
   multi-instance, open-source client and archival tool.
2. **Upstream documentation contributions:** Capture any API documentation gaps, ambiguities, or
   inaccuracies discovered during the `fyi-api-coverage-audit` track, and file/propose fixes
   upstream (Alaveteli docs, or per-instance API docs where publicly maintained).
3. **Findings sharing:** Summarize capability-detection/health-check results (which instances
   support which endpoints/features) in a shareable form, useful both to fyi-cli users and to
   Alaveteli maintainers auditing deployment consistency.
4. **Etiquette & rate-limit norms:** Document expected etiquette for bulk archival/sync use
   (request rate limits, polling intervals, User-Agent identification, caching behavior) so
   fyi-cli's defaults are good citizens of shared public infrastructure.
5. **Official listing exploration:** Investigate whether Alaveteli maintains (or would welcome) a
   list of known third-party clients/tools, and whether fyi-cli should request inclusion.
6. **Deliverable docs:** Produce a `docs/upstream-relations.md` (or docs-site page) documenting:
   the relationship with Alaveteli/instance operators, etiquette expectations enforced by
   fyi-cli's defaults (rate limiting, User-Agent, Tor usage disclosure), and links to any
   upstream issues/PRs filed.

## Non-Functional Requirements
- **No unsolicited bulk traffic:** This track is documentation/outreach only; it must not itself
  generate high-volume requests against any live instance.
- **Transparency:** Any outreach should clearly identify fyi-cli, its purpose, and a contact
  point (GitHub repo).

## Acceptance Criteria
- A drafted outreach issue (or equivalent written communication) exists for the Alaveteli
  project, ready to be posted by the maintainer.
- `docs/upstream-relations.md` exists, documenting etiquette norms, rate-limiting defaults, and
  the outreach/contribution log.
- Any concrete upstream documentation gaps found are captured as either a filed issue/PR link or
  a documented "not yet filed" note with rationale.
- CONTRIBUTING.md or the docs site links to the upstream-relations document.

## Out of Scope
- Actually sending/posting the outreach (requires the maintainer's own account/judgment on
  timing) — this track prepares the drafts and documentation only.
- Building any new upstream-facing tooling (e.g. an Alaveteli PR) beyond documentation.

## Dependencies
- Benefits from `fyi-api-coverage-audit` findings (health-check/capability results) but does not
  block on its full completion — can proceed with currently known findings.

## Success Metrics
- Upstream relations doc published and linked from README/docs site.
- At least one concrete, ready-to-send outreach draft produced.
