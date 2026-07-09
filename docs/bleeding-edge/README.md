# Bleeding-edge foundations

This directory documents **experimental** and **foundation** work for multi-jurisdiction
FOI operations (adapters, MCP resources, offline PWA, drafting refinement, notifications).

## Disclaimer

> **Not production guarantees.** Code and docs here may change without a stable
> semver commitment. Defaults keep experimental **feature flags off**
> (`fyi_core::features::FeatureSet::default_experimental()` is empty).
> Do not treat scaffolds, stubs, or heuristic scores as legal advice or as a
> substitute for jurisdiction-specific practice.

| Severity | Meaning |
|----------|---------|
| **Foundation** | Library types + unit tests land; API may still evolve |
| **Experimental** | Design, stub, or opt-in surface; not release-blocking |
| **Design only** | Docs / stub assets; full product behaviour not implemented |

## Status matrix

| Area | Status | Primary code / docs | Feature flag |
|------|--------|---------------------|--------------|
| Community jurisdiction adapter SDK | Foundation | `fyi_core::adapter`, [adapter-sdk.md](./adapter-sdk.md) | `community_adapters` |
| MCP resources (`fyi://…`) | Foundation | `fyi-mcp` list/read handlers, [mcp-resources.md](./mcp-resources.md) | `mcp_resources` |
| Feature flags | Foundation | `fyi_core::features` | (meta) |
| Drafting + multi-turn refinement | Foundation | `fyi_core::drafting` (`score_draft`, `refine_request_multi_turn`) | `ai_drafting_refinement` |
| Deadline notifications (types only) | Foundation | `fyi_core::notifications` | `deadline_notifications` |
| Statutory deadlines engine | Foundation | `fyi_core::deadlines` | — |
| Federation view | Foundation | `fyi_core::federation` | `federation_view` |
| Provenance hash chain | Foundation | `fyi_core::provenance` | `provenance_chain` |
| Hybrid search index | Foundation | `fyi_core::search` | `advanced_search` |
| Offline PWA | Design only + stub | [offline-pwa-design.md](./offline-pwa-design.md), `dashboard/public/` | `offline_pwa` |

## Feature flags (library)

```rust
use fyi_core::features::{FeatureFlag, FeatureSet};

let mut flags = FeatureSet::default_experimental(); // all off
flags.enable(FeatureFlag::AiDraftingRefinement);
assert!(flags.is_enabled(FeatureFlag::AiDraftingRefinement));

// Opt into everything documented here:
let edge = FeatureSet::all_experimental();
```

String ids (snake_case): `ai_drafting_refinement`, `community_adapters`,
`deadline_notifications`, `offline_pwa`, `mcp_resources`, `federation_view`,
`provenance_chain`, `advanced_search`.

## Doc index

| Document | Topic |
|----------|--------|
| [adapter-sdk.md](./adapter-sdk.md) | `CommunityJurisdictionAdapter` contributor guide |
| [mcp-resources.md](./mcp-resources.md) | MCP `fyi://` resource URIs |
| [offline-pwa-design.md](./offline-pwa-design.md) | Dashboard offline / installability design |

## Testing

```bash
cargo +stable-x86_64-pc-windows-gnu test -p fyi-core --lib
```

## Non-goals for this track

- Shipping experimental flags enabled by default in release builds
- Real email/SMS delivery of deadline reminders
- Full Workbox PWA production wiring
- Unreviewed community adapters in the embedded catalog
