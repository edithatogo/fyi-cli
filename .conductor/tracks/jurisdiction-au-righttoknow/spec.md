# Specification: jurisdiction-au-righttoknow

## Overview
This track onboards **righttoknow.org.au** as the first non-NZ jurisdiction. It creates the instance entry, FOI Act metadata, authority taxonomy, Australian request templates, and ensures discovery/archive parity with the existing NZ implementation.

## Functional Requirements
1. **Instance Configuration:**
   - Add `au-rtk` instance to `instances.toml`
   - Configure base URL: `https://www.righttoknow.org.au`
   - Set country: `AU`, locale: `en-AU`
   - Document FOI Act 1982 (Commonwealth) + state variations
2. **FOI Metadata:**
   - Request type term: "FOI request" (Commonwealth), "RTI" (Queensland, etc.)
   - Statutory deadlines: 30 calendar days (Commonwealth FOI Act), variations by state
   - Appeal body: Office of the Australian Information Commissioner (OAIC)
   - Citation templates for Commonwealth FOI Act 1982
3. **Authority Taxonomy:**
   - Import authority list from righttoknow.org.au
   - Map authority types (federal departments, agencies, statutory bodies, state/territory bodies)
   - Handle multi-level government structure (federal, state, local)
4. **Australian Templates:**
   - Create en-AU request letter templates
   - Appropriate salutations and sign-offs for Australian context
   - Legal citations for Commonwealth FOI Act
   - State-specific template variations where needed
5. **Discovery & Archive Parity:**
   - Verify all API endpoints work with au-rtk instance
   - Test feed discovery and parsing
   - Validate archive capture for Australian requests
   - Ensure search functionality covers au-rtk

## Non-Functional Requirements
- **Compatibility:** Zero impact on existing nz-fyi functionality
- **Performance:** API response times comparable to nz-fyi
- **Legal Accuracy:** FOI Act metadata verified by Australian FOI expert or official sources
- **Test Coverage:** 90%+ coverage on AU-specific code paths

## Acceptance Criteria
- au-rtk instance configured and functional
- All API operations work with righttoknow.org.au
- Authority import successful with proper taxonomy
- Australian templates render correctly with proper legal citations
- Live-safe integration tests passing for au-rtk
- Documentation includes Australian FOI Act specifics
- Multi-instance tests verify NZ/AU isolation

## Out of Scope
- State/territory-specific FOI/RTI legislation (Phase 1 focuses on Commonwealth)
- Non-English content (au-rtk is English-only)
- Historical data migration (new instance starts empty)

## Dependencies
- Depends on: `jurisdiction-abstraction-core` (track 2)

## Success Metrics
- **Instance Functional:** All core operations work on au-rtk
- **Authority Coverage:** 500+ Australian authorities imported
- **Template Quality:** Legal citations verified accurate
- **Zero Regressions:** NZ functionality unchanged
