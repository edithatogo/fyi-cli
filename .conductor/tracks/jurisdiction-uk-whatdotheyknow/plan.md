# Plan: jurisdiction-uk-whatdotheyknow

## Phase 1: Instance Configuration & FOIA 2000 Metadata

### 1.1 Instance Entry
- [ ] Task: Add uk-wdtk entry to `instances.toml`
- [ ] Task: Configure base_url, country, locale fields
- [ ] Task: Set capabilities flags
- [ ] Task: Write tests for uk-wdtk instance loading
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: Instance' (Protocol in workflow.md)

### 1.2 FOIA 2000 Metadata
- [ ] Task: Research Freedom of Information Act 2000
- [ ] Task: Document statutory deadlines (20 working days)
- [ ] Task: Identify appeal body (ICO)
- [ ] Task: Create FOIA 2000 metadata structure
- [ ] Task: Write tests for metadata validation
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: FOIA Metadata' (Protocol in workflow.md)

### 1.3 Devolved Legislation
- [ ] Task: Document Scotland FOISA 2002
- [ ] Task: Document Wales FOI provisions
- [ ] Task: Document Northern Ireland FOI Act 2000
- [ ] Task: Map devolved variations
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: Devolved' (Protocol in workflow.md)

## Phase 2: UK Authority Taxonomy (Large Scale)

### 2.1 Authority Discovery
- [ ] Task: Fetch authority list from whatdotheyknow.com API
- [ ] Task: Implement pagination for large authority sets
- [ ] Task: Parse and validate 5,000+ authorities
- [ ] Task: Write tests for large-scale import
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Discovery' (Protocol in workflow.md)

### 2.2 Taxonomy Mapping
- [ ] Task: Map central government departments (e.g., Home Office)
- [ ] Task: Map local councils (district, county, unitary)
- [ ] Task: Map NHS bodies (trusts, CCGs, ICBs)
- [ ] Task: Map police forces and emergency services
- [ ] Task: Map schools, universities, and education authorities
- [ ] Task: Map devolved administrations
- [ ] Task: Write tests for taxonomy lookup at scale
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Taxonomy' (Protocol in workflow.md)

### 2.3 Bulk Authority Import
- [ ] Task: Implement chunked bulk import (batch of 100)
- [ ] Task: Add progress tracking for large imports
- [ ] Task: Validate imported data quality
- [ ] Task: Test with 5,000+ authorities
- [ ] Task: Implement incremental update mechanism
- [ ] Task: Conductor - User Manual Verification 'Phase 2.3: Import' (Protocol in workflow.md)

## Phase 3: UK Request Templates

### 3.1 FOIA 2000 Template
- [ ] Task: Create en-GB request letter template
- [ ] Task: Use British English spelling (colour, organisation, etc.)
- [ ] Task: Include FOIA 2000 citation
- [ ] Task: Add British salutations and sign-offs
- [ ] Task: Write tests for template rendering
- [ ] Task: Verify with UK FOI practitioners
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Template' (Protocol in workflow.md)

### 3.2 Devolved Templates
- [ ] Task: Create Scotland FOISA template variant
- [ ] Task: Test templates with realistic data
- [ ] Task: Verify legal citations
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: Devolved Templates' (Protocol in workflow.md)

### 3.3 Follow-up Templates
- [ ] Task: Create follow-up letter template
- [ ] Task: Create internal review request template
- [ ] Task: Create ICO complaint template
- [ ] Task: Test all template variants
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: Follow-ups' (Protocol in workflow.md)

## Phase 4: Rate Limiting & Performance Hardening

### 4.1 Rate Limiting Implementation
- [ ] Task: Implement request rate limiter (1 req/sec for WhatDoTheyKnow)
- [ ] Task: Add configurable rate limits per instance
- [ ] Task: Implement exponential backoff on rate limit errors
- [ ] Task: Write tests for rate limiting behavior
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Rate Limiting' (Protocol in workflow.md)

### 4.2 Pagination Optimization
- [ ] Task: Implement efficient pagination for large result sets
- [ ] Task: Add result streaming for large queries
- [ ] Task: Test with 10,000+ result sets
- [ ] Task: Optimize memory usage during large operations
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: Pagination' (Protocol in workflow.md)

### 4.3 Caching Strategy
- [ ] Task: Implement authority list caching
- [ ] Task: Add request metadata caching with TTL
- [ ] Task: Test cache invalidation
- [ ] Task: Measure cache hit rates
- [ ] Task: Conductor - User Manual Verification 'Phase 4.3: Caching' (Protocol in workflow.md)

## Phase 5: Scale Testing & API Integration

### 5.1 Load Testing
- [ ] Task: Create load test suite for uk-wdtk
- [ ] Task: Test with 5,000+ authorities
- [ ] Task: Test with 100,000+ requests
- [ ] Task: Measure API response times under load
- [ ] Task: Identify and fix performance bottlenecks
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: Load Testing' (Protocol in workflow.md)

### 5.2 API Endpoint Verification
- [ ] Task: Test all read operations on uk-wdtk
- [ ] Task: Test search API with UK requests
- [ ] Task: Test feed discovery for uk-wdtk
- [ ] Task: Verify rate limiting doesn't break functionality
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: API' (Protocol in workflow.md)

### 5.3 Live-Safe Integration Tests
- [ ] Task: Create wiremock tests for uk-wdtk endpoints
- [ ] Task: Test large result set handling
- [ ] Task: Test error recovery and retry logic
- [ ] Task: Verify graceful degradation
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: Integration' (Protocol in workflow.md)

## Phase 6: Multi-Instance Testing & Documentation

### 6.1 Three-Instance Isolation
- [ ] Task: Create tests verifying NZ/AU/UK data separation
- [ ] Task: Test concurrent operations on all three instances
- [ ] Task: Verify no cross-instance interference
- [ ] Task: Test instance switching across all three
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Isolation' (Protocol in workflow.md)

### 6.2 UK FOI Documentation
- [ ] Task: Document FOIA 2000 specifics
- [ ] Task: Create guide to UK FOI system
- [ ] Task: Document devolved variations
- [ ] Task: Add examples of UK requests
- [ ] Task: Document rate limiting considerations
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: Documentation' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] uk-wdtk instance fully functional
- [ ] 5,000+ UK authorities imported
- [ ] Rate limiting prevents API throttling
- [ ] Performance benchmarks met
- [ ] All tests passing (NZ + AU + UK)
- [ ] Documentation complete

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
