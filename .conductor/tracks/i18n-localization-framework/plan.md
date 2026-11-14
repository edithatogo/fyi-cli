# Plan: i18n-localization-framework

## Reconciliation — 2026-11-14

The historical completed status is unsupported by the original acceptance criteria.
The baseline has six smoke tests and hardcoded greetings/terms, but no Fluent
resources, typed templates, holiday calendars or reviewed translations. Work resumes
within this track; shared registries remain untouched during parallel work.

- [x] Bounded increment 1: versioned message bundles, explicit locale selection,
  deterministic fallback and strict interpolation contracts.
- [x] Bounded increment 2: typed request/letter templates, golden fixtures and
  delegation to the existing deadline module.

The historical checklist below is reconciled only against inspectable evidence.
Human language/legal review is distinct from automated rendering verification.

Current reconciliation (2026-11-14): foundation PR #436 merged as `a0fa71a`,
and typed-template PR #447 merged as `4322d47`. Both are present in master
`9050a66`. Their earlier verification checkpoints below remain historical
evidence. Issue #39 remains open; its closed historical children are not proof
of the remaining terminology, calendar or legal acceptance.

- [x] Bounded increment 3: explicit jurisdiction terminology profiles, localized
  glossary lookup, source references and private catalog validation (PR #450).
  Uses the existing Fluent/locale implementation; deadline and source owners'
  components remain unchanged. Implementation checkpoint `9f34fda` passed 12
  terminology contracts and 670 instrumented workspace tests; final-head checks
  and coordinated integration remain required.

## Phase 1: Fluent-rs Integration

### 1.1 Core Integration
- [x] Task: Add `fluent-rs` and `unic-langid` dependencies
- [x] Task: Write failing tests for message bundle loading
- [x] Task: Create fluent message loader with fallback chains
- [x] Task: Implement locale detection and selection
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: Fluent Integration' (Protocol in workflow.md)

### 1.2 Message Bundles
- [x] Task: Create directory structure for .ftl files (locales/en-NZ/, locales/en-AU/, etc.)
- [x] Task: Create base English messages (en/common.ftl)
- [x] Task: Write tests for message interpolation
- [x] Task: Implement message formatting with variables
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: Message Bundles' (Protocol in workflow.md)

Automated Phase 1 evidence: [acceptance-evidence.md](acceptance-evidence.md),
red checkpoint `bc2c804`, 16 localization integration tests and 389 workspace tests
passing with Rust 1.99.1. Manual/human review remains explicitly unchecked.

## Phase 2: Request Letter Templates

### 2.1 Template Structure
- [x] Task: Design template schema (salutation, body, citation, sign-off)
- [x] Task: Write failing tests for template rendering
- [x] Task: Create English templates for NZ, AU, UK
- [x] Task: Implement template variable substitution
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: Templates' (Protocol in workflow.md)

### 2.2 Jurisdiction-Specific Templates
- [x] Task: Create NZ OIA request template with legal citation
- [x] Task: Create AU FOI request template
- [x] Task: Create UK FOIA request template
- [x] Task: Test templates with realistic data
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: Jurisdiction Templates' (Protocol in workflow.md)

### 2.3 Template Variants
- [x] Task: Create formal vs. informal template variants
- [x] Task: Add follow-up letter templates
- [x] Task: Add internal review request templates
- [x] Task: Add appeal letter templates
- [ ] Task: Conductor - User Manual Verification 'Phase 2.3: Variants' (Protocol in workflow.md)

Automated Phase 2 evidence: [acceptance-evidence.md](acceptance-evidence.md),
21 template/deadline integration tests plus the existing 16 message tests, all
48 locale/variant/register combinations, six full-letter goldens, and 99.635%
combined localization line coverage. Human review remains unchecked.

## Phase 3: Terminology Mapping

### 3.1 Term Registry
- [x] Task: Create terminology registry data structure — `9f34fda`
- [x] Task: Map FOI term equivalents (OIA/FOI/RTI/IFG/FOIA) — nine explicit profiles;
  additional regimes and legal applicability remain outside this bounded glossary
- [x] Task: Map request-type terms across jurisdictions — typed display concepts
- [x] Task: Map authority terms (department/ministry/agency)
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: Registry' (Protocol in workflow.md)

### 3.2 Glossary Integration
- [x] Task: Create jurisdiction-specific glossaries — nine source-qualified profiles
- [x] Task: Implement term lookup by jurisdiction and locale — no jurisdiction fallback
- [x] Task: Write tests for term mapping accuracy — exact rendering/source contracts;
  this does not certify human translation or legal accuracy
- [x] Task: Create glossary documentation — API, extension contract and sources
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
- [x] Task: Implement business day counter (skip weekends) — existing canonical
  `deadlines.rs` implementation retained through delegation; holiday/legal rules
  remain unverified and open below.
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
- [x] Task: Create de-DE message bundles
- [x] Task: Create German IFG request template
- [x] Task: Add German terminology (Informationsfreiheitsgesetz) — bounded display glossary
- [ ] Task: Add German holiday calendar
- [ ] Task: Human review and verification
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: German' (Protocol in workflow.md)

### 5.2 French Localization (fr-FR)
- [x] Task: Create fr-FR message bundles
- [x] Task: Create French CADA request template
- [x] Task: Add French terminology — CRPA profile; CADA is not treated as a law
- [ ] Task: Add French holiday calendar
- [ ] Task: Human review and verification
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: French' (Protocol in workflow.md)

### 5.3 Spanish Localization (es-ES)
- [x] Task: Create es-ES message bundles
- [x] Task: Create Spanish transparency request template
- [x] Task: Add Spanish terminology — sourced LTAIBG profile
- [ ] Task: Add Spanish holiday calendar
- [ ] Task: Human review and verification
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: Spanish' (Protocol in workflow.md)

## Phase 6: Integration & Testing

### 6.1 Instance Integration
- [x] Task: Connect i18n system to Instance model
- [ ] Task: Auto-select locale from instance configuration
- [ ] Task: Test locale switching with instance changes
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Integration' (Protocol in workflow.md)

### 6.2 End-to-End Testing
- [x] Task: Create comprehensive i18n test suite
- [x] Task: Test all templates in all locales
- [ ] Task: Verify deadline calculations for all jurisdictions
- [x] Task: Test fallback locale chains
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: E2E Tests' (Protocol in workflow.md)

### 6.3 Documentation
- [x] Task: Document i18n architecture
- [x] Task: Create locale addition guide
- [x] Task: Document template customization
- [x] Task: Create translation guidelines for contributors
- [ ] Task: Conductor - User Manual Verification 'Phase 6.3: Documentation' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [x] fluent-rs integrated with 6+ locales
- [ ] All templates human-verified
- [ ] Deadline engine accurate for all jurisdictions
- [x] 95%+ test coverage on i18n module
- [ ] Documentation complete

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion

- **2026-11-14**: Reconciled historical completion; Phases 1 and 2 implementation
  and automated verification complete. Manual translation/legal review and later
  acceptance criteria remain open. Canonical deadline arithmetic is reused.
- **2026-11-14**: Reconciled merged #436/#447 and reopened incomplete children
  #64–#67 under #39. PR #450 implements the bounded terminology/glossary contracts.
  Human verification, calendars/statutory accuracy and broader acceptance remain
  unchecked. Shared-registry reconciliation stays with the integration coordinator.
