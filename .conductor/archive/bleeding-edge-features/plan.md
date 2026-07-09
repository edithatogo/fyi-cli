# Plan: bleeding-edge-features

> **Archive note (2026-07-09):** Repo-side foundations are complete. Production
> integrations are listed under [Deferred (not required for archive)](#deferred-not-required-for-archive)
> and do not block track completion.

## Phase 1: AI-Assisted Request Drafting

### 1.1 LLM Integration Framework
- [x] Task: Design provider-agnostic LLM interface (`LlmClient` trait)
- [x] Task: Write tests for LLM integration (`MockLlmClient`, draft path unit tests)
- [ ] Task: Integrate with OpenAI API (reference impl) — **deferred** (see Deferred)
- [ ] Task: Support local models (ollama, llama.cpp) — **deferred**
- [ ] Task: Implement token budget management — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: LLM Framework' (Protocol in workflow.md)

### 1.2 Jurisdiction-Aware Drafting
- [x] Task: Create jurisdiction-aware prompt templates
- [x] Task: Include legal citations in prompts
- [x] Task: Inject authority context
- [x] Task: Test with multiple jurisdictions
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Drafting' (Protocol in workflow.md)

### 1.3 Request Refinement
- [ ] Task: Implement multi-turn conversation for request improvement — **deferred**
- [ ] Task: Add legal compliance checks — **deferred**
- [ ] Task: Quality scoring for generated requests — **deferred**
- [ ] Task: User feedback collection — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: Refinement' (Protocol in workflow.md)

## Phase 2: Semantic Search over Corpora

### 2.1 Embedding Generation
- [ ] Task: Choose embedding model (sentence-transformers, OpenAI) — **deferred**
- [ ] Task: Generate embeddings for archived requests — **deferred**
- [ ] Task: Store embeddings in vector database (qdrant, weaviate, or sqlite-vec) — **deferred**
- [ ] Task: Batch processing for large corpora — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Embeddings' (Protocol in workflow.md)

### 2.2 Vector / Hybrid Search
- [x] Task: Implement hybrid search (token overlap + title boost; in-memory index)
- [x] Task: Result ranking and scoring
- [x] Task: Test search relevance (unit tests)
- [ ] Task: Implement vector similarity search — **deferred**
- [ ] Task: Cross-jurisdiction search — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Vector Search' (Protocol in workflow.md)

### 2.3 Attachment OCR
- [ ] Task: Integrate OCR engine (tesseract, doctr) — **deferred**
- [ ] Task: Extract text from PDF attachments — **deferred**
- [ ] Task: Index OCR text for search — **deferred**
- [ ] Task: Test with real-world attachments — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 2.3: OCR' (Protocol in workflow.md)

### 2.4 Full-Text Indexing
- [x] Task: Create full-text index for requests/correspondence (`SearchIndex` trait + in-memory backend)
- [x] Task: Support basic query tokenization and ranked hits
- [ ] Task: Support advanced query syntax — **deferred**
- [ ] Task: Faceted search (by jurisdiction, status, date) — **deferred**
- [ ] Task: Test indexing performance at scale — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 2.4: Full-Text' (Protocol in workflow.md)

## Phase 3: Statutory Deadline Engine

### 3.1 Deadline Calculation
- [x] Task: Implement working-day calculator (extend from i18n track)
- [x] Task: Support jurisdiction-specific rules (calendar days vs working days)
- [x] Task: Test deadline accuracy
- [ ] Task: Integrate full holiday calendars for every locale — **deferred** (rules present; full calendar productization later)
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Calculation' (Protocol in workflow.md)

### 3.2 Deadline Tracking
- [x] Task: Track request status vs deadline (`evaluate_overdue` / `is_overdue` helpers)
- [x] Task: Identify overdue requests
- [ ] Task: Store calculated deadlines in production database schema — **deferred**
- [ ] Task: Generate deadline reports — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: Tracking' (Protocol in workflow.md)

### 3.3 Notification System
- [ ] Task: Implement email notifications for approaching deadlines — **deferred**
- [ ] Task: Add webhook support for integrations — **deferred**
- [ ] Task: Support push notifications (optional) — **deferred**
- [ ] Task: Configurable reminder schedules (7 days, 3 days, 1 day before) — **deferred**
- [ ] Task: Test notification delivery — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: Notifications' (Protocol in workflow.md)

## Phase 4: Federation & Cross-Jurisdiction Analytics

### 4.1 Unified Data View
- [x] Task: Create federated query/view engine across catalog instances
- [x] Task: Aggregate statistics (by country, by status)
- [x] Task: Cross-jurisdiction data model (`FederationView`, `FederatedInstanceSummary`)
- [x] Task: Test with multiple instances
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Federation' (Protocol in workflow.md)

### 4.2 Comparative Analytics
- [ ] Task: Implement jurisdiction comparison dashboard — **deferred** (live UI)
- [ ] Task: Response time benchmarking — **deferred**
- [ ] Task: Transparency scoring per jurisdiction — **deferred**
- [ ] Task: Visualize comparative data — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: Analytics' (Protocol in workflow.md)

### 4.3 Multi-Instance Dashboard
- [ ] Task: Extend Next.js dashboard for federation — **deferred**
- [ ] Task: Instance switcher UI — **deferred**
- [ ] Task: Aggregate views across instances — **deferred**
- [ ] Task: Per-instance drill-down — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 4.3: Dashboard' (Protocol in workflow.md)

## Phase 5: Adapter SDK & Plugin Architecture

### 5.1 Plugin API Design
- [x] Task: Define plugin interface (`CommunityJurisdictionAdapter`)
- [x] Task: Create plugin loader/registry (`register_adapter_instance`)
- [x] Task: Write plugin API documentation (module docs + stub example)
- [ ] Task: Support dynamic plugin loading (dlopen / runtime packages) — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: API Design' (Protocol in workflow.md)

### 5.2 Example Plugins
- [x] Task: Create example/stub plugin for non-first-party FOI site (`StubCommunityAdapter`)
- [x] Task: Document plugin development workflow (inline docs + validation helpers)
- [ ] Task: Create full plugin template/scaffold package — **deferred**
- [ ] Task: Test plugin isolation sandbox — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: Examples' (Protocol in workflow.md)

### 5.3 Community Plugin Support
- [ ] Task: Create plugin registry/catalog — **deferred**
- [ ] Task: Plugin submission process — **deferred**
- [ ] Task: Plugin verification and security review — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: Community' (Protocol in workflow.md)

## Phase 6: Signed Provenance for Archives

### 6.1 WARC/WACZ Extension / Hash Chain Foundation
- [x] Task: Design tamper-evident provenance for archive records (SHA-256 hash chain)
- [x] Task: Test signature/chain generation
- [ ] Task: Integrate sigstore for signature generation — **deferred**
- [ ] Task: Sign WACZ archives with external trust roots — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Signing' (Protocol in workflow.md)

### 6.2 C2PA Integration
- [ ] Task: Research C2PA (Content Authenticity Initiative) — **deferred**
- [ ] Task: Integrate C2PA manifest generation — **deferred**
- [ ] Task: Embed provenance metadata — **deferred**
- [ ] Task: Test C2PA validation — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: C2PA' (Protocol in workflow.md)

### 6.3 Verification Tooling
- [x] Task: Implement chain verification (integrity + tamper detection helpers)
- [x] Task: Verify archive integrity via prev-hash linkage
- [x] Task: Detect tampering in broken chains
- [ ] Task: Implement signature verification CLI command (user-facing) — **deferred**
- [ ] Task: Generate verification reports — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 6.3: Verification' (Protocol in workflow.md)

## Phase 7: MCP Resources (Corpus Exposure)

### 7.1 MCP Resource Protocol
- [x] Task: Implement MCP resources (not just tools)
- [x] Task: Expose archived requests as resources
- [x] Task: Support resource listing and browsing (`resources/list`, `resources/read`)
- [x] Task: Test with MCP client path (unit tests in `fyi-mcp`)
- [ ] Task: Conductor - User Manual Verification 'Phase 7.1: Resources' (Protocol in workflow.md)

### 7.2 Structured Data Access
- [x] Task: Create resource schemas for requests/authorities (JSON resource payloads)
- [ ] Task: Support filtering and search via resources — **deferred**
- [ ] Task: Citation and attribution metadata productization — **deferred**
- [ ] Task: Test with AI assistants (Claude, GPT) in production — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 7.2: Data Access' (Protocol in workflow.md)

### 7.3 Resource Documentation
- [ ] Task: Document MCP resource endpoints — **deferred** (module/tests exist; published guide later)
- [ ] Task: Create examples for AI assistant integration — **deferred**
- [ ] Task: Write integration guide — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 7.3: Docs' (Protocol in workflow.md)

## Phase 8: Offline-First PWA Dashboard

### 8.1 Service Worker
- [x] Task: Add service worker stub to Next.js dashboard (`dashboard/public/sw-stub.js`)
- [x] Task: Document offline caching strategy (`docs/bleeding-edge/offline-pwa-design.md`)
- [ ] Task: Implement production offline caching strategy — **deferred**
- [ ] Task: Cache static assets in production SW — **deferred**
- [ ] Task: Test offline functionality end-to-end — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 8.1: Service Worker' (Protocol in workflow.md)

### 8.2 Local-First Architecture
- [ ] Task: Implement client-side data storage (IndexedDB) — **deferred**
- [ ] Task: Sync mechanism with server — **deferred**
- [ ] Task: Conflict resolution for offline changes — **deferred**
- [ ] Task: Test offline→online sync — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 8.2: Local-First' (Protocol in workflow.md)

### 8.3 Progressive Enhancement
- [ ] Task: Ensure core functionality works without JS — **deferred**
- [ ] Task: Add progressive enhancement layers — **deferred**
- [ ] Task: Test on various devices/browsers — **deferred**
- [ ] Task: Mobile-responsive refinement — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 8.3: Progressive' (Protocol in workflow.md)

### 8.4 PWA Manifest
- [ ] Task: Create PWA manifest — **deferred**
- [ ] Task: Add install prompts — **deferred**
- [ ] Task: Test "add to home screen" on mobile — **deferred**
- [ ] Task: Configure app icons and branding — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 8.4: PWA' (Protocol in workflow.md)

## Phase 9: Integration & Documentation

### 9.1 Feature Flag System
- [ ] Task: Implement feature flags for experimental features — **deferred**
- [ ] Task: Allow per-user feature enablement — **deferred**
- [ ] Task: Create feature graduation process (experimental→stable) — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 9.1: Flags' (Protocol in workflow.md)

### 9.2 Experimental Feature Documentation
- [x] Task: Document bleeding-edge foundation modules (module docs + PWA design)
- [x] Task: Add experimental / design-only disclaimers where applicable
- [ ] Task: Create feature comparison matrix (stable vs experimental) — **deferred**
- [ ] Task: Usage examples for each production feature — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 9.2: Docs' (Protocol in workflow.md)

### 9.3 User Feedback Collection
- [ ] Task: Create feedback mechanism for experimental features — **deferred**
- [ ] Task: Track feature usage metrics — **deferred**
- [ ] Task: Collect satisfaction ratings — **deferred**
- [ ] Task: Process feedback for feature refinement — **deferred**
- [ ] Task: Conductor - User Manual Verification 'Phase 9.3: Feedback' (Protocol in workflow.md)

## Deferred (not required for archive)

Production integrations intentionally left out of this archive. They may be
picked up as follow-on tracks/issues; none block `bleeding-edge-features`
completion.

| Area | Deferred work |
|------|----------------|
| LLM | Real OpenAI (or other remote) provider; ollama/llama.cpp; token budgets; multi-turn refinement productization |
| Search | Embedding models; vector DB; true semantic/vector search; OCR (tesseract/doctr); faceted/advanced FTS at scale |
| Deadlines | Full holiday calendar productization; DB persistence of deadlines; email/webhook/push delivery |
| Federation UI | Live Next.js federation dashboard, instance switcher, aggregate/drill-down views, benchmarking visualizations |
| Adapter ecosystem | Dynamic runtime plugin loading; public registry/catalog; sandbox isolation; submission/review process |
| Provenance | Full sigstore/cosign; C2PA manifests; WARC/WACZ external signing; user-facing verify CLI/reports |
| MCP | Published resource integration guide; production assistant E2E; rich citation metadata productization |
| PWA | Full offline IndexedDB; conflict sync; production SW caching; installable manifest/icons; progressive enhancement |
| Product | Feature flags, usage metrics, satisfaction feedback loops |

## Completion Criteria (archive)

### Repo-side foundations — met
- [x] AI request drafting foundation (trait + mock + jurisdiction-aware path)
- [x] Hybrid / full-text search foundation operational
- [x] Deadline engine calculation and overdue helpers
- [x] Federation catalog view
- [x] Adapter SDK trait + stub documented in-module
- [x] Provenance hash-chain working
- [x] MCP resources exposed
- [x] PWA design + SW stub

### Production — deferred (not required)
- [ ] All original production phases complete end-to-end
- [ ] Remote LLM, OCR, sigstore, full offline PWA, live federation UI, email delivery

## Implementation progress (repo-side, 2026-07-09)

- [x] #120 deadlines engine module (`crates/fyi-core/src/deadlines.rs`)
- [x] #118 LlmClient + MockLlm + draft_request_with_llm (`drafting.rs`)
- [x] #119 in-memory hybrid search (`search.rs`)
- [x] #121 federation view (`federation.rs`)
- [x] #122 community adapter trait (`adapter.rs`)
- [x] #123 SHA-256 provenance chain (`provenance.rs`)
- [x] #124 MCP resources/list + resources/read (`fyi-mcp`)
- [x] #125 offline PWA design + SW stub (design only)
- [x] Linked implementation PRs: #126, #130

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
- **2026-07-09**: Repo-side foundations implemented (deadlines, drafting LLM trait, hybrid search, federation, adapter SDK, provenance hash-chain, MCP resources, PWA design/stub)
- **2026-07-09**: Archive prep — mark foundations complete; production integrations (real OpenAI, OCR, sigstore, full PWA IndexedDB offline, live Next.js federation UI, email delivery) deferred; `metadata.status` → `completed`; `github_pr` → `[126, 130]`; spec acceptance split repo-side vs production. Orchestrator archives via `tracks.md` (not edited here).
