# FYI Request System (v14)

A local-first assisted workflow for planning, drafting, tracking, and monitoring public OIA requests that may be lodged through FYI.org.nz.

This repo is structured around Conductor's context-driven development workflow and includes prompt packs, handover templates, operator skills, a CLI, a local SQLite store, and a small local web UI.

## Why this architecture

FYI.org.nz documents a partial API surface rather than a full submission-management API. It supports prefilled request links, feeds, JSON on many pages, and authority exports. This repo therefore treats FYI as a public-facing request/publication layer and keeps your planning, drafting, monitoring, and analytics in your own local system.

## Features

- Conductor-style context and track scaffolding under `.conductor/`
- SQLite-backed authority and tracked-request registry
- CSV authority import
- FYI prefilled request URL builder
- feed ingestion and reconciliation
- request-page JSON snapshotting
- attention report and markdown handover generation
- static HTML dashboard
- local web UI for:
  - browsing and searching tracked requests
  - creating and editing tracked requests
  - viewing a prefilled FYI draft link
  - browsing/searching authorities
  - importing authority CSVs through the browser
- local scheduler runner

## Install

```bash
pip install -e .[dev]
```

## Quick start

```bash
fyi-system init-db
fyi-system import-authorities data/sample_authorities.csv
fyi-system register-request auckland_council "Service metrics request" "Please provide..." --status draft
fyi-system dashboard --output outputs/dashboard.html --json-output outputs/dashboard.json
fyi-system serve --host 127.0.0.1 --port 8000
```

Then open `http://127.0.0.1:8000`.

## Useful commands

```bash
fyi-system build-prefilled-url auckland_council "Service metrics request" "Please provide..." --tags "topic:metrics"
fyi-system ingest-feed "https://fyi.org.nz/search/service/feed"
fyi-system fetch-request-page 1
fyi-system reconcile-events
fyi-system attention-report --output outputs/attention-report.json
fyi-system handover --output outputs/handover.md
fyi-system export-request 1 --output outputs/request-1.json
```

## Suggested operator flow

1. Import or refresh authorities.
2. Create a tracked request locally.
3. Use the prefilled FYI link to open a draft in the browser.
4. When an FYI request exists, save its request ID / URL back into the tracked record.
5. Ingest relevant feeds and fetch request-page JSON snapshots.
6. Reconcile events, generate attention reports, and produce handovers.

## Notes

- This repo does **not** automate hidden identity routing, proxy chaining, or Tor-based masking.
- It is designed for transparent, local operation with a human in the loop.


## Phase 8 additions

- richer per-request detail page in the local UI
- latest snapshot summary with detected attachments and event-like objects
- `request-detail` CLI command for combined tracked-request + snapshot output
- extra prompt and skill for snapshot review


## Phase 9 additions

- `fyi-system follow-up-draft <request_id>` generates a suggested follow-up payload
- `fyi-system attachment-manifest <request_id>` exports detected attachments from the latest snapshot


## Phase 10 additions

- `fyi-system follow-up-variants <id>` for multiple follow-up strategies
- `fyi-system attachment-manifest-csv <id>` for CSV attachment export
- `fyi-system response-analysis <id>` for snapshot-based response assessment


## Phase 11 additions

- `fyi-system follow-up-pack <id>` for strategy-and-tone follow-up bundles
- `fyi-system triage-report` for action-now and action-soon queues
- dashboard and UI priority filtering

## Phase 12 additions

- `fyi-system next-best-action <id>` for a concrete per-request next-step recommendation
- `fyi-system correspondence-pack <id>` for grouped correspondence exports by strategy and tone
- local UI next-best-action card and dedicated correspondence page
- request detail route fixed so detail, edit, and correspondence views are separate


## v13 highlights

- `fyi-system export-bundle <request_id>` builds a portable per-request bundle.
- The local UI now has direct “Open recommended draft” actions.
- Snapshot parsing is more tolerant of varied FYI/Alaveteli JSON shapes.


## v14 privacy and security additions

- private file/directory permissions are applied to the database and generated outputs where the OS supports it
- `fyi-system privacy-audit` checks localhost binding, file modes, and export posture
- `fyi-system show-settings` reads privacy defaults from a JSON settings file and/or environment variables
- bundled exports are sanitized by default, with a stricter profile available for moving material off-device
- the local web UI now emits `Cache-Control: no-store`, `Referrer-Policy: no-referrer`, `X-Content-Type-Options: nosniff`, and a restrictive Content Security Policy
- `.env.example` documents privacy-related environment variables

### Example

```bash
fyi-system privacy-audit --db fyi_system.db --output outputs/privacy-audit.json
fyi-system export-bundle 1 --profile strict
fyi-system show-settings
```
