# Plan: jurisdiction-au-righttoknow

## Phase 1: Instance Configuration & Metadata

### 1.1 Instance Entry
- [ ] Task: Add au-rtk entry to `instances.toml`
- [ ] Task: Configure base_url, country, locale fields
- [ ] Task: Set capabilities (read/write/attachments/feeds)
- [ ] Task: Write tests for au-rtk instance loading
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: Instance' (Protocol in workflow.md)

### 1.2 FOI Act Metadata
- [ ] Task: Research Commonwealth FOI Act 1982 requirements
- [ ] Task: Document statutory deadlines (30 calendar days)
- [ ] Task: Identify appeal body (OAIC)
- [ ] Task: Create FOI Act metadata structure
- [ ] Task: Write tests for metadata validation
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Metadata' (Protocol in workflow.md)

### 1.3 State Variations
- [ ] Task: Document state/territory FOI/RTI legislation differences
- [ ] Task: Map Queensland RTI Act, NSW GIPA, etc.
- [ ] Task: Create state-specific metadata where needed
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: State Variations' (Protocol in workflow.md)

## Phase 2: Authority Taxonomy

### 2.1 Authority Discovery
- [ ] Task: Fetch authority list from righttoknow.org.au API
- [ ] Task: Parse and validate authority data
- [ ] Task: Write tests for authority import
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Discovery' (Protocol in workflow.md)

### 2.2 Taxonomy Mapping
- [ ] Task: Map federal departments (e.g., Department of Finance)
- [ ] Task: Map agencies and statutory bodies
- [ ] Task: Map state/territory authorities
- [ ] Task: Create authority classification system
- [ ] Task: Write tests for taxonomy lookup
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Taxonomy' (Protocol in workflow.md)

### 2.3 Authority Import
- [ ] Task: Implement bulk authority import for au-rtk
- [ ] Task: Validate imported data quality
- [ ] Task: Test with 500+ authorities
- [ ] Task: Create authority update mechanism
- [ ] Task: Conductor - User Manual Verification 'Phase 2.3: Import' (Protocol in workflow.md)

## Phase 3: Australian Request Templates

### 3.1 Commonwealth FOI Template
- [ ] Task: Create en-AU request letter template
- [ ] Task: Add Australian salutations (Dear Sir/Madam, Dear FOI Officer)
- [ ] Task: Include Commonwealth FOI Act 1982 citation
- [ ] Task: Add appropriate sign-off for Australian context
- [ ] Task: Write tests for template rendering
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Template' (Protocol in workflow.md)

### 3.2 State-Specific Templates
- [ ] Task: Create Queensland RTI template variant
- [ ] Task: Create NSW GIPA template variant
- [ ] Task: Test templates with realistic data
- [ ] Task: Verify legal citations with official sources
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: State Templates' (Protocol in workflow.md)

### 3.3 Follow-up Templates
- [ ] Task: Create follow-up letter template for AU
- [ ] Task: Create internal review request template
- [ ] Task: Create IC review/complaint template
- [ ] Task: Test all template variants
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: Follow-ups' (Protocol in workflow.md)

## Phase 4: API Integration & Testing

### 4.1 API Endpoint Verification
- [ ] Task: Test all read operations on au-rtk
- [ ] Task: Test search API with Australian requests
- [ ] Task: Test feed discovery for au-rtk
- [ ] Task: Verify pagination and filtering
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: API' (Protocol in workflow.md)

### 4.2 Live-Safe Integration Tests
- [ ] Task: Create wiremock tests for au-rtk endpoints
- [ ] Task: Test request creation (mock mode)
- [ ] Task: Test correspondence addition
- [ ] Task: Test state updates
- [ ] Task: Verify error handling
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: Integration' (Protocol in workflow.md)

### 4.3 Archive Parity
- [ ] Task: Test archive capture for Australian requests
- [ ] Task: Verify WARC generation for au-rtk
- [ ] Task: Test attachment handling
- [ ] Task: Validate content-addressed deduplication
- [ ] Task: Conductor - User Manual Verification 'Phase 4.3: Archive' (Protocol in workflow.md)

## Phase 5: Multi-Instance Isolation Testing

### 5.1 Data Isolation
- [ ] Task: Create tests verifying NZ/AU data separation
- [ ] Task: Test concurrent operations on both instances
- [ ] Task: Verify no cross-instance credential leakage
- [ ] Task: Test instance switching
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: Isolation' (Protocol in workflow.md)

### 5.2 Regression Testing
- [ ] Task: Run full NZ test suite with AU instance present
- [ ] Task: Verify zero NZ functionality impact
- [ ] Task: Test default instance behavior
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: Regression' (Protocol in workflow.md)

## Phase 6: Documentation & Rollout

### 6.1 Australian FOI Documentation
- [ ] Task: Document Commonwealth FOI Act 1982 specifics
- [ ] Task: Create guide to Australian FOI system
- [ ] Task: Document state/territory variations
- [ ] Task: Add examples of Australian requests
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Documentation' (Protocol in workflow.md)

### 6.2 User Guide
- [ ] Task: Update user guide with au-rtk examples
- [ ] Task: Document instance selection for Australian users
- [ ] Task: Create Australian quickstart guide
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: User Guide' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] au-rtk instance fully functional
- [ ] 500+ Australian authorities imported
- [ ] Templates verified accurate
- [ ] All tests passing (NZ + AU)
- [ ] Documentation complete

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
