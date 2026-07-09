# Offline PWA design (dashboard)

Status: design only (issue #125). This document describes how the Next.js
`dashboard/` app can become a Progressive Web App with offline-first operator
workflows. No full Next.js PWA implementation is required for the bleeding-edge
foundation track; a minimal service-worker stub lives under
`dashboard/public/sw-stub.js`.

## Goals

- Operators can open the local dashboard, triage requests, and draft letters
  while offline or on an unreliable link to the MCP/SQLite backend.
- Cached shell + last-known summary data remain readable without network.
- Mutations queue locally and flush when connectivity returns (aligned with
  `fyi-core` offline sync / outgoing queue concepts).
- No remote Alaveteli submission from the SW; the CLI/MCP remain the authority
  for writes.

## Non-goals (this phase)

- Full Workbox / `next-pwa` production wiring.
- Background Sync API polyfills for every browser.
- Encrypted client-side stores (see credentials/security modules separately).
- App Store packaging.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Browser (dashboard PWA shell)                          │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │ App Router  │  │ IndexedDB    │  │ Service Worker │  │
│  │ pages/API   │◄─┤ cache +      │◄─┤ (stub → full)  │  │
│  │ client      │  │ outbox       │  │ cache shell    │  │
│  └──────┬──────┘  └──────▲───────┘  └────────┬───────┘  │
└─────────┼────────────────┼───────────────────┼──────────┘
          │ fetch          │                   │ precache
          ▼                │                   ▼
┌──────────────────┐       │            static assets
│ Next API routes  │       │            /_next/static/*
│ (dashboard BFF)  │       │
└────────┬─────────┘       │
         │ MCP / SQLite    │
         ▼                 │
┌──────────────────┐       │
│ fyi-mcp + fyi-core db    │
│ (source of truth)        │
└──────────────────────────┘
```

### Layers

1. **App shell** — static Next export or standalone server assets cached by the
   service worker (`/`, layout CSS/JS, icons, offline fallback page).
2. **Data cache** — IndexedDB (or Cache Storage JSON) holding:
   - last `dashboard/summary` payload
   - recent `list_requests` / per-request snapshots
   - authority catalog slice
3. **Outbox** — pending local mutations (`create_request`, `update_request`,
   tag edits) with client-generated UUIDs, replayed via MCP tools when online.
4. **MCP bridge** — existing `dashboard/src/lib/mcp-client.ts` remains the
   online path; offline mode short-circuits reads to IndexedDB and enqueues
   writes.

## Caching strategy

| Resource | Strategy | Notes |
|----------|----------|-------|
| App shell HTML/JS/CSS | Cache-first, versioned precache | Bust on deploy hash |
| `/api/dashboard/summary` | Network-first, fallback to IDB | Stale-while-revalidate optional |
| `/api/requests/*` | Network-first | On failure serve last snapshot |
| MCP JSON-RPC | Never cache blindly | Use explicit IDB snapshots |
| External CDNs | Avoid | Prefer self-hosted assets |

## Service worker phases

### Phase 0 — stub (current)

`dashboard/public/sw-stub.js` registers optionally from a future layout effect.
It only logs lifecycle events and does **not** intercept fetches. Safe for
local dev; no offline behaviour.

### Phase 1 — shell offline

- Precache build assets after `next build`.
- Offline fallback route showing “Dashboard offline — showing last sync”.
- Manual “Save for offline” on request detail pages.

### Phase 2 — outbox + sync

- Mirror MCP mutating tools into an outbox table.
- On `online` event, drain outbox with idempotent tool calls.
- Surface dirty/conflict counts using the same semantics as `sync_monitor`.

### Phase 3 — installability

- `manifest.webmanifest` (name, icons, `display: standalone`, `start_url`).
- HTTPS or localhost only.
- Optional periodic background sync where supported.

## Dashboard integration notes

Files of interest under `dashboard/`:

| Path | Role |
|------|------|
| `src/app/layout.tsx` | Future SW registration (feature-flagged) |
| `src/app/page.tsx` | Home summary — primary offline surface |
| `src/lib/mcp-client.ts` | Online MCP transport; wrap with offline facade |
| `src/lib/dashboard-summary.ts` | Shape of cached summary documents |
| `src/components/DashboardSummary*.tsx` | UI that should tolerate stale data banners |
| `public/sw-stub.js` | Phase 0 service worker stub |
| `public/` (future) | `manifest.webmanifest`, icons, offline.html |

Suggested registration (not wired by default):

```ts
// layout effect, only when NEXT_PUBLIC_ENABLE_SW=1
if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/sw-stub.js").catch(console.warn);
}
```

## Security & privacy

- Offline cache may hold request bodies and personal names — treat device disk
  as sensitive; document wipe on logout if multi-user machines are used.
- Do not cache API keys or master secrets in Cache Storage.
- Prefer same-origin MCP/BFF; avoid exposing SQLite over the public internet.

## Relation to other bleeding-edge tracks

- **#120 deadlines** — offline UI can show computed due dates from cached
  start dates without network.
- **#119 search** — in-memory / FTS search runs on cached request corpus.
- **#123 provenance** — archive downloads can verify hash chains after offline
  capture.
- **#121 federation** — multi-instance summaries are static enough to precache
  from the embedded catalog.

## Acceptance criteria (future implementation PR)

- [ ] Installable manifest + icons
- [ ] Shell loads with network disabled after first visit
- [ ] Summary page shows last-known metrics with stale indicator
- [ ] At least one mutation survives offline → online replay
- [ ] Feature flag keeps default dev experience unchanged
- [ ] No regression in Vitest suite for existing components
