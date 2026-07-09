# Specification: bleeding-edge-features

## Overview
This track implements a prioritized backlog of advanced, experimental, and innovative features that push the boundaries of FOI tooling. These features are jurisdiction-aware where relevant and represent R&D investments in cutting-edge functionality.

**Archive scope (2026-07-09):** Repo-side foundations are complete and accepted for archive. Production integrations (live LLM providers, OCR, sigstore/C2PA, full offline IndexedDB PWA, live federation UI, email delivery) are explicitly deferred and are **not** required for this track to close.

## Functional Requirements
1. **AI-Assisted Request Drafting:**
   - Jurisdiction-aware request templates
   - LLM-backed request generation and refinement (provider-agnostic trait + mock)
   - Context-aware suggestions (authority, request type, legal citations)
   - Multi-turn conversation for request improvement *(production deferred)*
   - Quality checks and legal compliance verification *(production deferred)*
2. **Semantic Search over Corpora:**
   - Embedding generation for archived requests/responses *(production deferred)*
   - In-memory hybrid search (token overlap + title boost) as foundation
   - Vector similarity / remote vector DB *(production deferred)*
   - Attachment OCR for searchable text extraction *(production deferred)*
   - Full-text indexing trait (SQLite FTS-friendly) for structured and unstructured data
   - Cross-jurisdiction comparative search *(production deferred)*
3. **Statutory Deadline Engine:**
   - Working-day calculation with jurisdiction-specific rules
   - Holiday / calendar-day vs working-day rules
   - Automated deadline evaluation helpers (overdue / remaining)
   - Reminder/notification system (email, webhook, push) *(production deferred)*
   - Escalation alerts for approaching deadlines *(production deferred)*
4. **Federation:**
   - Unified cross-jurisdiction view over the embedded catalog
   - Aggregate statistics across instances
   - Comparative transparency analytics *(production deferred UI)*
   - Multi-instance dashboard *(production deferred live Next.js UI)*
   - Jurisdiction benchmarking *(production deferred)*
5. **Adapter SDK:**
   - Plugin architecture for community-contributed jurisdictions
   - Extension API for custom FOI providers (`CommunityJurisdictionAdapter`)
   - Hooks for instance-specific logic (scaffold + validation)
   - Developer documentation and examples (stub adapter)
6. **Signed Provenance for Archives:**
   - SHA-256 hash-chain provenance for tamper-evident local archives
   - Full WARC/WACZ + sigstore/C2PA content authenticity *(production deferred)*
   - Verification helpers for chain integrity / tampering detection
7. **MCP Resources:**
   - Expose corpora as MCP resources (not just tools)
   - Allow AI assistants to browse archived requests
   - Structured data access for LLMs
   - Citation and attribution support (foundation listing/read)
8. **Offline-First PWA Dashboard:**
   - Design document for offline-first operator workflows
   - Service worker stub under `dashboard/public/sw-stub.js`
   - Full IndexedDB local-first + conflict sync *(production deferred)*
   - Progressive enhancement / installable manifest polish *(production deferred)*

## Non-Functional Requirements
- **Innovation:** Experimental features with clear experimental disclaimers
- **Quality:** Features graduate from experimental to stable based on user feedback
- **Interoperability:** Features work across all supported jurisdictions
- **Documentation:** Clear docs for experimental features
- **Archive readiness:** Repo-side modules ship with unit tests; production providers may remain stubs

## Acceptance Criteria

### Repo-side (required for archive) — met
- [x] Provider-agnostic LLM drafting trait (`LlmClient`) with `MockLlmClient` and jurisdiction-aware prompt path (`crates/fyi-core/src/drafting.rs`)
- [x] In-memory hybrid search index with ranking (`crates/fyi-core/src/search.rs`)
- [x] Statutory deadline engine: working-day rules, instance-aware calculation, overdue helpers (`crates/fyi-core/src/deadlines.rs`)
- [x] Federation view aggregating catalog instances (`crates/fyi-core/src/federation.rs`)
- [x] Community adapter trait + stub + registration helper (`crates/fyi-core/src/adapter.rs`)
- [x] SHA-256 provenance hash chain with verification (`crates/fyi-core/src/provenance.rs`)
- [x] MCP `resources/list` and `resources/read` exposing corpora (`crates/fyi-mcp`)
- [x] Offline PWA design doc + service-worker stub (`docs/bleeding-edge/offline-pwa-design.md`, `dashboard/public/sw-stub.js`)
- [x] Features documented as experimental/foundation where implemented

### Production (deferred — not required for archive)
- [ ] Real remote LLM provider integration (OpenAI, ollama, llama.cpp)
- [ ] Embedding generation + vector DB + true semantic/vector search
- [ ] Attachment OCR pipeline (tesseract/doctr) and OCR indexing
- [ ] Email / webhook / push deadline notifications and live delivery
- [ ] Live Next.js federation dashboard UI (instance switcher, aggregate views, drill-down)
- [ ] Full sigstore/cosign + C2PA WARC/WACZ signing and external verification
- [ ] Full PWA offline: IndexedDB store, conflict resolution, installable manifest, production SW caching
- [ ] Multi-turn drafting refinement, compliance scoring, and feedback collection productization

## Out of Scope
- Production-ready deployment of all features (some remain experimental)
- Integration with specific LLM providers as hard dependencies (provider-agnostic foundation only)
- Real-time collaboration features
- Production items listed under **Production (deferred)** above for this archive

## Dependencies
- Depends on: `jurisdiction-abstraction-core` (track 2)

## Success Metrics
- **AI Drafting Quality:** 80%+ user satisfaction with generated requests *(production metric; foundation ships mock path)*
- **Search Relevance:** 90%+ precision@10 for semantic search *(production metric; foundation ships hybrid token search)*
- **Deadline Tracking:** 100% accuracy in deadline calculations *(repo-side unit-tested)*
- **Federation Adoption:** 50%+ of users use cross-jurisdiction features *(production metric; foundation ships catalog federation view)*
