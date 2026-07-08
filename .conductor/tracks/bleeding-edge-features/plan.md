# Plan: bleeding-edge-features

## Phase 1: AI-Assisted Request Drafting

### 1.1 LLM Integration Framework
- [ ] Task: Design provider-agnostic LLM interface
- [ ] Task: Integrate with OpenAI API (reference impl)
- [ ] Task: Support local models (ollama, llama.cpp)
- [ ] Task: Implement token budget management
- [ ] Task: Write tests for LLM integration
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: LLM Framework' (Protocol in workflow.md)

### 1.2 Jurisdiction-Aware Drafting
- [ ] Task: Create jurisdiction-aware prompt templates
- [ ] Task: Include legal citations in prompts
- [ ] Task: Inject authority context
- [ ] Task: Test with multiple jurisdictions
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Drafting' (Protocol in workflow.md)

### 1.3 Request Refinement
- [ ] Task: Implement multi-turn conversation for request improvement
- [ ] Task: Add legal compliance checks
- [ ] Task: Quality scoring for generated requests
- [ ] Task: User feedback collection
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: Refinement' (Protocol in workflow.md)

## Phase 2: Semantic Search over Corpora

### 2.1 Embedding Generation
- [ ] Task: Choose embedding model (sentence-transformers, OpenAI)
- [ ] Task: Generate embeddings for archived requests
- [ ] Task: Store embeddings in vector database (qdrant, weaviate, or sqlite-vec)
- [ ] Task: Batch processing for large corpora
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Embeddings' (Protocol in workflow.md)

### 2.2 Vector Search
- [ ] Task: Implement vector similarity search
- [ ] Task: Hybrid search (vector + full-text)
- [ ] Task: Cross-jurisdiction search
- [ ] Task: Result ranking and scoring
- [ ] Task: Test search relevance
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Vector Search' (Protocol in workflow.md)

### 2.3 Attachment OCR
- [ ] Task: Integrate OCR engine (tesseract, doctr)
- [ ] Task: Extract text from PDF attachments
- [ ] Task: Index OCR text for search
- [ ] Task: Test with real-world attachments
- [ ] Task: Conductor - User Manual Verification 'Phase 2.3: OCR' (Protocol in workflow.md)

### 2.4 Full-Text Indexing
- [ ] Task: Create full-text index for requests/correspondence
- [ ] Task: Support advanced query syntax
- [ ] Task: Faceted search (by jurisdiction, status, date)
- [ ] Task: Test indexing performance
- [ ] Task: Conductor - User Manual Verification 'Phase 2.4: Full-Text' (Protocol in workflow.md)

## Phase 3: Statutory Deadline Engine

### 3.1 Deadline Calculation
- [ ] Task: Implement working-day calculator (extend from i18n track)
- [ ] Task: Integrate holiday calendars
- [ ] Task: Support jurisdiction-specific rules (calendar days vs working days)
- [ ] Task: Test deadline accuracy
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Calculation' (Protocol in workflow.md)

### 3.2 Deadline Tracking
- [ ] Task: Store calculated deadlines in database
- [ ] Task: Track request status vs deadline
- [ ] Task: Identify overdue requests
- [ ] Task: Generate deadline reports
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: Tracking' (Protocol in workflow.md)

### 3.3 Notification System
- [ ] Task: Implement email notifications for approaching deadlines
- [ ] Task: Add webhook support for integrations
- [ ] Task: Support push notifications (optional)
- [ ] Task: Configurable reminder schedules (7 days, 3 days, 1 day before)
- [ ] Task: Test notification delivery
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: Notifications' (Protocol in workflow.md)

## Phase 4: Federation & Cross-Jurisdiction Analytics

### 4.1 Unified Data View
- [ ] Task: Create federated query engine across instances
- [ ] Task: Aggregate statistics (total requests, response rates)
- [ ] Task: Cross-jurisdiction data model
- [ ] Task: Test with multiple instances
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Federation' (Protocol in workflow.md)

### 4.2 Comparative Analytics
- [ ] Task: Implement jurisdiction comparison dashboard
- [ ] Task: Response time benchmarking
- [ ] Task: Transparency scoring per jurisdiction
- [ ] Task: Visualize comparative data
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: Analytics' (Protocol in workflow.md)

### 4.3 Multi-Instance Dashboard
- [ ] Task: Extend Next.js dashboard for federation
- [ ] Task: Instance switcher UI
- [ ] Task: Aggregate views across instances
- [ ] Task: Per-instance drill-down
- [ ] Task: Conductor - User Manual Verification 'Phase 4.3: Dashboard' (Protocol in workflow.md)

## Phase 5: Adapter SDK & Plugin Architecture

### 5.1 Plugin API Design
- [ ] Task: Define plugin interface (FoiProvider extension)
- [ ] Task: Create plugin loader/registry
- [ ] Task: Support dynamic plugin loading
- [ ] Task: Write plugin API documentation
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: API Design' (Protocol in workflow.md)

### 5.2 Example Plugins
- [ ] Task: Create example plugin for non-Alaveteli FOI system
- [ ] Task: Document plugin development workflow
- [ ] Task: Create plugin template/scaffold
- [ ] Task: Test plugin isolation
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: Examples' (Protocol in workflow.md)

### 5.3 Community Plugin Support
- [ ] Task: Create plugin registry/catalog
- [ ] Task: Plugin submission process
- [ ] Task: Plugin verification and security review
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: Community' (Protocol in workflow.md)

## Phase 6: Signed Provenance for Archives

### 6.1 WARC/WACZ Extension
- [ ] Task: Design signature extension for WARC records
- [ ] Task: Integrate sigstore for signature generation
- [ ] Task: Sign WACZ archives
- [ ] Task: Test signature generation
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Signing' (Protocol in workflow.md)

### 6.2 C2PA Integration
- [ ] Task: Research C2PA (Content Authenticity Initiative)
- [ ] Task: Integrate C2PA manifest generation
- [ ] Task: Embed provenance metadata
- [ ] Task: Test C2PA validation
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: C2PA' (Protocol in workflow.md)

### 6.3 Verification Tooling
- [ ] Task: Implement signature verification CLI command
- [ ] Task: Verify archive integrity
- [ ] Task: Detect tampering
- [ ] Task: Generate verification reports
- [ ] Task: Conductor - User Manual Verification 'Phase 6.3: Verification' (Protocol in workflow.md)

## Phase 7: MCP Resources (Corpus Exposure)

### 7.1 MCP Resource Protocol
- [ ] Task: Implement MCP resources (not just tools)
- [ ] Task: Expose archived requests as resources
- [ ] Task: Support resource listing and browsing
- [ ] Task: Test with MCP clients
- [ ] Task: Conductor - User Manual Verification 'Phase 7.1: Resources' (Protocol in workflow.md)

### 7.2 Structured Data Access
- [ ] Task: Create resource schemas for requests/authorities
- [ ] Task: Support filtering and search via resources
- [ ] Task: Citation and attribution metadata
- [ ] Task: Test with AI assistants (Claude, GPT)
- [ ] Task: Conductor - User Manual Verification 'Phase 7.2: Data Access' (Protocol in workflow.md)

### 7.3 Resource Documentation
- [ ] Task: Document MCP resource endpoints
- [ ] Task: Create examples for AI assistant integration
- [ ] Task: Write integration guide
- [ ] Task: Conductor - User Manual Verification 'Phase 7.3: Docs' (Protocol in workflow.md)

## Phase 8: Offline-First PWA Dashboard

### 8.1 Service Worker
- [ ] Task: Add service worker to Next.js dashboard
- [ ] Task: Implement offline caching strategy
- [ ] Task: Cache static assets
- [ ] Task: Test offline functionality
- [ ] Task: Conductor - User Manual Verification 'Phase 8.1: Service Worker' (Protocol in workflow.md)

### 8.2 Local-First Architecture
- [ ] Task: Implement client-side data storage (IndexedDB)
- [ ] Task: Sync mechanism with server
- [ ] Task: Conflict resolution for offline changes
- [ ] Task: Test offline→online sync
- [ ] Task: Conductor - User Manual Verification 'Phase 8.2: Local-First' (Protocol in workflow.md)

### 8.3 Progressive Enhancement
- [ ] Task: Ensure core functionality works without JS
- [ ] Task: Add progressive enhancement layers
- [ ] Task: Test on various devices/browsers
- [ ] Task: Mobile-responsive refinement
- [ ] Task: Conductor - User Manual Verification 'Phase 8.3: Progressive' (Protocol in workflow.md)

### 8.4 PWA Manifest
- [ ] Task: Create PWA manifest
- [ ] Task: Add install prompts
- [ ] Task: Test "add to home screen" on mobile
- [ ] Task: Configure app icons and branding
- [ ] Task: Conductor - User Manual Verification 'Phase 8.4: PWA' (Protocol in workflow.md)

## Phase 9: Integration & Documentation

### 9.1 Feature Flag System
- [ ] Task: Implement feature flags for experimental features
- [ ] Task: Allow per-user feature enablement
- [ ] Task: Create feature graduation process (experimental→stable)
- [ ] Task: Conductor - User Manual Verification 'Phase 9.1: Flags' (Protocol in workflow.md)

### 9.2 Experimental Feature Documentation
- [ ] Task: Document all bleeding-edge features
- [ ] Task: Add experimental disclaimers
- [ ] Task: Create feature comparison matrix (stable vs experimental)
- [ ] Task: Usage examples for each feature
- [ ] Task: Conductor - User Manual Verification 'Phase 9.2: Docs' (Protocol in workflow.md)

### 9.3 User Feedback Collection
- [ ] Task: Create feedback mechanism for experimental features
- [ ] Task: Track feature usage metrics
- [ ] Task: Collect satisfaction ratings
- [ ] Task: Process feedback for feature refinement
- [ ] Task: Conductor - User Manual Verification 'Phase 9.3: Feedback' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] AI request drafting functional
- [ ] Semantic search operational
- [ ] Deadline engine tracking and notifying
- [ ] Federation dashboard live
- [ ] Adapter SDK documented
- [ ] Signed provenance working
- [ ] MCP resources exposed
- [ ] PWA offline-capable
- [ ] All features documented

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
