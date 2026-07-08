# Specification: jurisdiction-uk-whatdotheyknow

## Overview
This track onboards **whatdotheyknow.com** (UK) as the second non-NZ jurisdiction, proving the jurisdiction abstraction pattern scales. It includes FOIA 2000 metadata, UK authority taxonomy, scale/rate-limit hardening for the large UK corpus, and comprehensive integration testing.

## Functional Requirements
1. **Instance Configuration:**
   - Add `uk-wdtk` instance to `instances.toml`
   - Configure base URL: `https://www.whatdotheyknow.com`
   - Set country: `GB`, locale: `en-GB`
   - Document Freedom of Information Act 2000 + devolved variations
2. **FOIA 2000 Metadata:**
   - Request type term: "FOI request" (FOIA 2000)
   - Statutory deadline: 20 working days from receipt
   - Appeal body: Information Commissioner's Office (ICO)
   - Citation templates for FOIA 2000
   - Devolved legislation: Scotland (FOISA 2002), Wales, Northern Ireland
3. **UK Authority Taxonomy:**
   - Import extensive UK authority list (5,000+ bodies)
   - Map central government departments and agencies
   - Include local councils, NHS trusts, police forces, schools
   - Handle devolved administrations (Scottish Government, Welsh Government, NI Executive)
4. **UK Request Templates:**
   - Create en-GB request letter templates
   - British English spelling and phrasing
   - FOIA 2000 legal citations
   - Devolved legislation template variants
5. **Scale & Rate Limit Hardening:**
   - Handle large corpus (100,000+ requests on WhatDoTheyKnow)
   - Implement rate limiting for API calls
   - Add pagination optimization
   - Test performance under load
   - Implement caching strategies

## Non-Functional Requirements
- **Scalability:** Handle 5,000+ authorities and 100,000+ requests
- **Performance:** API operations complete within 5s even for large datasets
- **Rate Limiting:** Respect WhatDoTheyKnow rate limits (no more than 1 req/sec)
- **Reliability:** Graceful degradation under API throttling
- **Legal Accuracy:** FOIA 2000 metadata verified against official sources

## Acceptance Criteria
- uk-wdtk instance configured and functional
- All API operations work with whatdotheyknow.com
- 5,000+ UK authorities imported successfully
- UK templates with FOIA 2000 citations verified
- Rate limiting prevents API throttling
- Performance tests pass with large datasets
- Live-safe integration tests passing for uk-wdtk
- Multi-instance tests verify NZ/AU/UK isolation

## Out of Scope
- Historical data migration (new instance starts empty)
- Devolved legislation deep-dive (Phase 1 focuses on FOIA 2000)
- Subject access requests (GDPR) vs FOI distinction

## Dependencies
- Depends on: `jurisdiction-au-righttoknow` (track 4)

## Success Metrics
- **Instance Functional:** All core operations work on uk-wdtk
- **Authority Coverage:** 5,000+ UK authorities imported
- **Performance:** API calls within 5s even with large result sets
- **Rate Limit Compliance:** Zero API throttling incidents
