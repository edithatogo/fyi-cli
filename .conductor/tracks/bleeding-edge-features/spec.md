# Specification: bleeding-edge-features

## Overview
This track implements a prioritized backlog of advanced, experimental, and innovative features that push the boundaries of FOI tooling. These features are jurisdiction-aware where relevant and represent R&D investments in cutting-edge functionality.

## Functional Requirements
1. **AI-Assisted Request Drafting:**
   - Jurisdiction-aware request templates
   - LLM-backed request generation and refinement
   - Context-aware suggestions (authority, request type, legal citations)
   - Multi-turn conversation for request improvement
   - Quality checks and legal compliance verification
2. **Semantic Search over Corpora:**
   - Embedding generation for archived requests/responses
   - Vector similarity search across jurisdictions
   - Attachment OCR for searchable text extraction
   - Full-text indexing for structured and unstructured data
   - Cross-jurisdiction comparative search
3. **Statutory Deadline Engine:**
   - Working-day calculation with jurisdiction-specific rules
   - Holiday calendar integration
   - Automated deadline tracking
   - Reminder/notification system (email, webhook, push)
   - Escalation alerts for approaching deadlines
4. **Federation:**
   - Unified cross-jurisdiction view
   - Aggregate statistics across instances
   - Comparative transparency analytics
   - Multi-instance dashboard
   - Jurisdiction benchmarking
5. **Adapter SDK:**
   - Plugin architecture for community-contributed jurisdictions
   - Extension API for custom FOI providers
   - Hooks for instance-specific logic
   - Developer documentation and examples
6. **Signed Provenance for Archives:**
   - Extend WARC/WACZ with cryptographic signatures
   - Integrate sigstore/C2PA for content authenticity
   - Tamper-evident archive records
   - Verification tooling
7. **MCP Resources:**
   - Expose corpora as MCP resources (not just tools)
   - Allow AI assistants to browse archived requests
   - Structured data access for LLMs
   - Citation and attribution support
8. **Offline-First PWA Dashboard:**
   - Build on existing Next.js dashboard
   - Service worker for offline functionality
   - Local-first architecture with sync
   - Progressive enhancement
   - Mobile-responsive design

## Non-Functional Requirements
- **Innovation:** Experimental features with clear experimental disclaimers
- **Quality:** Features graduate from experimental to stable based on user feedback
- **Interoperability:** Features work across all supported jurisdictions
- **Documentation:** Clear docs for experimental features

## Acceptance Criteria
- AI request drafting functional with LLM integration
- Semantic search returns relevant results across instances
- Deadline engine tracks and notifies accurately
- Federation dashboard shows cross-jurisdiction data
- Adapter SDK allows community extensions
- Signed provenance verifies archive authenticity
- MCP resources expose corpora to AI assistants
- PWA dashboard works offline
- All features documented as experimental/stable

## Out of Scope
- Production-ready deployment of all features (some remain experimental)
- Integration with specific LLM providers (provider-agnostic)
- Real-time collaboration features

## Dependencies
- Depends on: `jurisdiction-abstraction-core` (track 2)

## Success Metrics
- **AI Drafting Quality:** 80%+ user satisfaction with generated requests
- **Search Relevance:** 90%+ precision@10 for semantic search
- **Deadline Tracking:** 100% accuracy in deadline calculations
- **Federation Adoption:** 50%+ of users use cross-jurisdiction features
