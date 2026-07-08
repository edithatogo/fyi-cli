# Plan: jurisdiction-english-alaveteli-fleet

## Phase 1: Instance Discovery & Cataloging

### 1.1 Alaveteli Instance List
- [ ] Task: Fetch official Alaveteli instance list from mySociety
- [ ] Task: Identify English-language instances
- [ ] Task: Filter by API availability
- [ ] Task: Create instance catalog spreadsheet
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: Discovery' (Protocol in workflow.md)

### 1.2 Automated Instance Validation
- [ ] Task: Create instance health check script
- [ ] Task: Test API availability for each instance
- [ ] Task: Detect API version and capabilities
- [ ] Task: Generate validation report
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Validation' (Protocol in workflow.md)

### 1.3 Prioritization
- [ ] Task: Prioritize instances by corpus size
- [ ] Task: Identify instances with active communities
- [ ] Task: Create onboarding priority list
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: Prioritization' (Protocol in workflow.md)

## Phase 2: Community-Tier Support Model

### 2.1 Support Tiers Definition
- [ ] Task: Document `supported` vs `community` tier differences
- [ ] Task: Define community-tier testing requirements
- [ ] Task: Create tier assignment criteria
- [ ] Task: Write tests for tier-specific behavior
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Tiers' (Protocol in workflow.md)

### 2.2 Community Instance Template
- [ ] Task: Create community instance configuration template
- [ ] Task: Add disclaimer for community-tier instances
- [ ] Task: Implement reduced testing for community tier
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Template' (Protocol in workflow.md)

### 2.3 Contribution Process
- [ ] Task: Create CONTRIBUTING.md for new jurisdictions
- [ ] Task: Document instance configuration requirements
- [ ] Task: Create PR template for new instances
- [ ] Task: Set up automated validation for contributions
- [ ] Task: Conductor - User Manual Verification 'Phase 2.3: Contribution' (Protocol in workflow.md)

## Phase 3: Catalog Automation

### 3.1 Instance Discovery Script
- [ ] Task: Create script to fetch Alaveteli instance list
- [ ] Task: Parse instance data (base_url, country, name)
- [ ] Task: Generate instance configuration TOML
- [ ] Task: Write tests for discovery automation
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Discovery Script' (Protocol in workflow.md)

### 3.2 Capability Detection
- [ ] Task: Create API capability scanner
- [ ] Task: Test for search, feeds, create, attachments support
- [ ] Task: Auto-generate capability flags
- [ ] Task: Test with diverse instances
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: Capabilities' (Protocol in workflow.md)

### 3.3 Bulk Configuration Generation
- [ ] Task: Create bulk config generator from instance list
- [ ] Task: Generate FOI metadata templates
- [ ] Task: Create batch validation process
- [ ] Task: Test with 10+ instances
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: Bulk Generation' (Protocol in workflow.md)

## Phase 4: Generic English Templates

### 4.1 Fallback Template
- [ ] Task: Create generic English FOI request template
- [ ] Task: Use neutral terminology (FOI/information request)
- [ ] Task: Avoid jurisdiction-specific legal citations
- [ ] Task: Test with multiple jurisdictions
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Generic Template' (Protocol in workflow.md)

### 4.2 Customization Hooks
- [ ] Task: Design template customization system
- [ ] Task: Allow jurisdiction-specific overrides
- [ ] Task: Support term substitution (FOI/RTI/OIA)
- [ ] Task: Write tests for customization
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: Customization' (Protocol in workflow.md)

### 4.3 Template Testing
- [ ] Task: Test generic template with all English instances
- [ ] Task: Verify rendering quality
- [ ] Task: Collect feedback from test users
- [ ] Task: Conductor - User Manual Verification 'Phase 4.3: Template Testing' (Protocol in workflow.md)

## Phase 5: Bulk Onboarding & Testing

### 5.1 Automated Onboarding
- [ ] Task: Create end-to-end onboarding script
- [ ] Task: Generate instance config + add to catalog
- [ ] Task: Run smoke tests on new instance
- [ ] Task: Test onboarding with 5+ instances
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: Onboarding' (Protocol in workflow.md)

### 5.2 Smoke Testing Suite
- [ ] Task: Create minimal smoke tests for community instances
- [ ] Task: Test basic read operations
- [ ] Task: Test search functionality
- [ ] Task: Verify feed parsing
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: Smoke Tests' (Protocol in workflow.md)

### 5.3 Batch Testing
- [ ] Task: Run smoke tests across all community instances
- [ ] Task: Generate batch test report
- [ ] Task: Identify and quarantine failing instances
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: Batch Testing' (Protocol in workflow.md)

## Phase 6: Health Monitoring & Maintenance

### 6.1 Health Check System
- [ ] Task: Create periodic health check scheduler
- [ ] Task: Test instance availability (HTTP 200)
- [ ] Task: Test API endpoint responsiveness
- [ ] Task: Detect API version changes
- [ ] Task: Write tests for health monitoring
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Health Checks' (Protocol in workflow.md)

### 6.2 Monitoring Dashboard
- [ ] Task: Create instance health dashboard
- [ ] Task: Show status for all community instances
- [ ] Task: Alert on instance failures
- [ ] Task: Track uptime statistics
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: Dashboard' (Protocol in workflow.md)

### 6.3 Automated Maintenance
- [ ] Task: Auto-disable failing instances after grace period
- [ ] Task: Send notifications on instance status changes
- [ ] Task: Create maintenance runbook
- [ ] Task: Conductor - User Manual Verification 'Phase 6.3: Maintenance' (Protocol in workflow.md)

## Phase 7: Documentation & Community Enablement

### 7.1 Onboarding Documentation
- [ ] Task: Document automated onboarding process
- [ ] Task: Create troubleshooting guide
- [ ] Task: Document community tier limitations
- [ ] Task: Conductor - User Manual Verification 'Phase 7.1: Documentation' (Protocol in workflow.md)

### 7.2 Contribution Guide
- [ ] Task: Publish CONTRIBUTING.md for jurisdictions
- [ ] Task: Create example PRs for new instances
- [ ] Task: Document review process
- [ ] Task: Set up issue templates for instance requests
- [ ] Task: Conductor - User Manual Verification 'Phase 7.2: Contribution Guide' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] 10+ English Alaveteli instances onboarded
- [ ] Automated onboarding script functional
- [ ] Health monitoring operational
- [ ] Community contribution process documented
- [ ] All smoke tests passing

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
