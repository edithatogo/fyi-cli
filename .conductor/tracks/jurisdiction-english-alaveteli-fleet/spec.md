# Specification: jurisdiction-english-alaveteli-fleet

## Overview
This track onboards the remaining English-language Alaveteli instances (e.g., Ireland, other English deployments). It establishes community-tier onboarding processes and catalog automation to scale beyond manually-configured jurisdictions.

## Functional Requirements
1. **Remaining English Instances:**
   - Ireland (if applicable)
   - Other English-speaking countries with Alaveteli deployments
   - English-language instances in non-English-majority countries
2. **Community-Tier Support Model:**
   - Status: `community` (vs `supported` for NZ/AU/UK)
   - Reduced testing requirements (basic smoke tests vs full integration)
   - Community-contributed configurations
   - Best-effort support with clear disclaimers
3. **Catalog Automation:**
   - Automated discovery of Alaveteli instances from public lists
   - Validation of instance availability and API compatibility
   - Automatic instance configuration generation
   - Periodic instance health checks
4. **Instance Templates:**
   - Generic English-language request templates
   - Jurisdiction-specific customization hooks
   - Fallback to generic FOI terminology when specific terms unavailable
5. **Bulk Onboarding Process:**
   - Script to generate instance configurations from instance list
   - Automated capability detection (API endpoints available)
   - Batch testing across all community instances
   - Health monitoring dashboard

## Non-Functional Requirements
- **Automation:** 80% of onboarding automated (manual review for legal accuracy)
- **Scalability:** Support for 20+ community instances without performance impact
- **Maintenance:** Automated health checks detect unavailable instances
- **Documentation:** Clear contribution guide for community additions

## Acceptance Criteria
- All identified English Alaveteli instances cataloged
- Automated instance discovery working from public lists
- Community-tier instances functional with basic tests
- Generic English templates work across all instances
- Bulk onboarding script documented and tested
- Health monitoring detects instance failures
- Contribution guide published

## Out of Scope
- Non-English instances (covered by track 7)
- Legally-verified templates for community instances (generic only)
- Full integration testing (community tier uses smoke tests)

## Dependencies
- Depends on: `jurisdiction-uk-whatdotheyknow` (track 5)

## Success Metrics
- **Instance Count:** 10+ English Alaveteli instances onboarded
- **Automation Rate:** 80%+ of onboarding automated
- **Health Detection:** 95%+ uptime detection accuracy
- **Community Contributions:** Process documented and ready
