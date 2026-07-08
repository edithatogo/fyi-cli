# Plan: i18n-localization-framework

## Phase 1: Fluent-rs Integration

### 1.1 Core Integration
- [ ] Task: Add `fluent-rs` and `unic-langid` dependencies
- [ ] Task: Write failing tests for message bundle loading
- [ ] Task: Create fluent message loader with fallback chains
- [ ] Task: Implement locale detection and selection
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: Fluent Integration' (Protocol in workflow.md)

### 1.2 Message Bundles
- [ ] Task: Create directory structure for .ftl files (locales/en-NZ/, locales/en-AU/, etc.)
- [ ] Task: Create base English messages (en/common.ftl)
- [ ] Task: Write tests for message interpolation
- [ ] Task: Implement message formatting with variables
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Message Bundles' (Protocol in workflow.md)

## Phase 2: Request Letter Templates

### 2.1 Template Structure
- [ ] Task: Design template schema (salutation, body, citation, sign-off)
- [ ] Task: Write failing tests for template rendering
- [ ] Task: Create English templates for NZ, AU, UK
- [ ] Task: Implement template variable substitution
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Templates' (Protocol in workflow.md)

### 2.2 Jurisdiction-Specific Templates
- [ ] Task: Create NZ OIA request template with legal citation
- [ ] Task: Create AU FOI request template
- [ ] Task: Create UK FOIA request template
- [ ] Task: Test templates with realistic data
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Jurisdiction Templates' (Protocol in workflow.md)

### 2.3 Template Variants
- [ ] Task: Create formal vs. informal template variants
- [ ] Task: Add follow-up letter templates
- [ ] Task: Add internal review request templates
- [ ] Task: Add appeal letter templates
- [ ] Task: Conductor - User Manual Verification 'Phase 2.3: Variants' (Protocol in workflow.md)

## Phase 3: Terminology Mapping

### 3.1 Term Registry
- [ ] Task: Create terminology registry data structure
- [ ] Task: Map FOI term equivalents (OIA/FOI/RTI/IFG/FOIA)
- [ ] Task: Map request-type terms across jurisdictions
- [ ] Task: Map authority terms (department/ministry/agency)
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Registry' (Protocol in workflow.md)

### 3.2 Glossary Integration
- [ ] Task: Create jurisdiction-specific glossaries
- [ ] Task: Implement term lookup by jurisdiction and locale
- [ ] Task: Write tests for term mapping accuracy
- [ ] Task: Create glossary documentation
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: Glossary' (Protocol in workflow.md)

## Phase 4: Working-Day/Deadline Engine

### 4.1 Holiday Calendar System
- [ ] Task: Design holiday calendar data format
- [ ] Task: Create NZ public holiday calendar (including regional)
- [ ] Task: Create AU holiday calendars (federal + states)
- [ ] Task: Create UK holiday calendars (England/Scotland/Wales/NI)
- [ ] Task: Write tests for holiday lookup
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Holidays' (Protocol in workflow.md)

### 4.2 Working-Day Calculator
- [ ] Task: Write failing tests for working-day calculation
- [ ] Task: Implement business day counter (skip weekends)
- [ ] Task: Integrate holiday calendar skipping
- [ ] Task: Handle jurisdiction-specific rules (e.g., observed holidays)
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: Calculator' (Protocol in workflow.md)

### 4.3 Statutory Deadline Engine
- [ ] Task: Implement deadline calculation from receipt date
- [ ] Task: Add jurisdiction-specific deadline rules (20/30/45 days)
- [ ] Task: Test against known statutory examples
- [ ] Task: Add reminder/notification date calculation
- [ ] Task: Conductor - User Manual Verification 'Phase 4.3: Deadlines' (Protocol in workflow.md)

## Phase 5: European Locales (Non-English)

### 5.1 German Localization (de-DE)
- [ ] Task: Create de-DE message bundles
- [ ] Task: Create German IFG request template
- [ ] Task: Add German terminology (Informationsfreiheitsgesetz)
- [ ] Task: Add German holiday calendar
- [ ] Task: Human review and verification
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: German' (Protocol in workflow.md)

### 5.2 French Localization (fr-FR)
- [ ] Task: Create fr-FR message bundles
- [ ] Task: Create French CADA request template
- [ ] Task: Add French terminology
- [ ] Task: Add French holiday calendar
- [ ] Task: Human review and verification
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: French' (Protocol in workflow.md)

### 5.3 Spanish Localization (es-ES)
- [ ] Task: Create es-ES message bundles
- [ ] Task: Create Spanish transparency request template
- [ ] Task: Add Spanish terminology
- [ ] Task: Add Spanish holiday calendar
- [ ] Task: Human review and verification
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: Spanish' (Protocol in workflow.md)

## Phase 6: Integration & Testing

### 6.1 Instance Integration
- [ ] Task: Connect i18n system to Instance model
- [ ] Task: Auto-select locale from instance configuration
- [ ] Task: Test locale switching with instance changes
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Integration' (Protocol in workflow.md)

### 6.2 End-to-End Testing
- [ ] Task: Create comprehensive i18n test suite
- [ ] Task: Test all templates in all locales
- [ ] Task: Verify deadline calculations for all jurisdictions
- [ ] Task: Test fallback locale chains
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: E2E Tests' (Protocol in workflow.md)

### 6.3 Documentation
- [ ] Task: Document i18n architecture
- [ ] Task: Create locale addition guide
- [ ] Task: Document template customization
- [ ] Task: Create translation guidelines for contributors
- [ ] Task: Conductor - User Manual Verification 'Phase 6.3: Documentation' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] fluent-rs integrated with 6+ locales
- [ ] All templates human-verified
- [ ] Deadline engine accurate for all jurisdictions
- [ ] 95%+ test coverage on i18n module
- [ ] Documentation complete

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
