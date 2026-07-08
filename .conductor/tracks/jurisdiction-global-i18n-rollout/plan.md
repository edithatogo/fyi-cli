# Plan: jurisdiction-global-i18n-rollout

## Phase 1: Germany - FragDenStaat.de (de-DE)

### 1.1 Instance Configuration
- [ ] Task: Add de-fds instance to `instances.toml`
- [ ] Task: Configure base_url for FragDenStaat.de
- [ ] Task: Set country: DE, locale: de-DE
- [ ] Task: Document IFG (Informationsfreiheitsgesetz) metadata
- [ ] Task: Conductor - User Manual Verification 'Phase 1.1: DE Instance' (Protocol in workflow.md)

### 1.2 German IFG Templates
- [ ] Task: Create de-DE request letter template
- [ ] Task: Include IFG legal citations
- [ ] Task: Use appropriate German salutations (Sehr geehrte Damen und Herren)
- [ ] Task: Professional German sign-offs
- [ ] Task: Native speaker review and approval
- [ ] Task: Conductor - User Manual Verification 'Phase 1.2: DE Templates' (Protocol in workflow.md)

### 1.3 German Terminology
- [ ] Task: Map German FOI terms (Informationsfreiheitsgesetz, Antrag, Behörde)
- [ ] Task: Create German glossary
- [ ] Task: Test terminology accuracy
- [ ] Task: Conductor - User Manual Verification 'Phase 1.3: DE Terminology' (Protocol in workflow.md)

### 1.4 German Holiday Calendar
- [ ] Task: Add German public holidays (federal + state)
- [ ] Task: Implement working-day calculation for Germany
- [ ] Task: Test deadline calculations
- [ ] Task: Conductor - User Manual Verification 'Phase 1.4: DE Calendar' (Protocol in workflow.md)

## Phase 2: France - MaDada.fr / CADA (fr-FR)

### 2.1 Instance Configuration
- [ ] Task: Add fr-cada instance to `instances.toml`
- [ ] Task: Document CADA (Commission d'accès aux documents administratifs) process
- [ ] Task: Set country: FR, locale: fr-FR
- [ ] Task: Conductor - User Manual Verification 'Phase 2.1: FR Instance' (Protocol in workflow.md)

### 2.2 French CADA Templates
- [ ] Task: Create fr-FR request letter template
- [ ] Task: Include French administrative law references
- [ ] Task: Use appropriate French salutations (Madame, Monsieur)
- [ ] Task: Professional French sign-offs
- [ ] Task: Native speaker review and approval
- [ ] Task: Conductor - User Manual Verification 'Phase 2.2: FR Templates' (Protocol in workflow.md)

### 2.3 French Terminology
- [ ] Task: Map French transparency terms
- [ ] Task: Create French glossary
- [ ] Task: Test terminology accuracy
- [ ] Task: Conductor - User Manual Verification 'Phase 2.3: FR Terminology' (Protocol in workflow.md)

### 2.4 French Holiday Calendar
- [ ] Task: Add French public holidays (including regional)
- [ ] Task: Implement working-day calculation for France
- [ ] Task: Test deadline calculations
- [ ] Task: Conductor - User Manual Verification 'Phase 2.4: FR Calendar' (Protocol in workflow.md)

## Phase 3: Spain - tuderechoasaber.es (es-ES)

### 3.1 Instance Configuration
- [ ] Task: Add es-tdas instance to `instances.toml`
- [ ] Task: Document Spanish transparency law
- [ ] Task: Set country: ES, locale: es-ES
- [ ] Task: Conductor - User Manual Verification 'Phase 3.1: ES Instance' (Protocol in workflow.md)

### 3.2 Spanish Templates
- [ ] Task: Create es-ES request letter template
- [ ] Task: Include Spanish transparency law citations
- [ ] Task: Use appropriate Spanish salutations (Estimado/a Señor/a)
- [ ] Task: Professional Spanish sign-offs
- [ ] Task: Native speaker review and approval
- [ ] Task: Conductor - User Manual Verification 'Phase 3.2: ES Templates' (Protocol in workflow.md)

### 3.3 Spanish Terminology
- [ ] Task: Map Spanish transparency terms
- [ ] Task: Create Spanish glossary
- [ ] Task: Test terminology accuracy
- [ ] Task: Conductor - User Manual Verification 'Phase 3.3: ES Terminology' (Protocol in workflow.md)

### 3.4 Spanish Holiday Calendar
- [ ] Task: Add Spanish public holidays (national + autonomous communities)
- [ ] Task: Implement working-day calculation for Spain
- [ ] Task: Test deadline calculations
- [ ] Task: Conductor - User Manual Verification 'Phase 3.4: ES Calendar' (Protocol in workflow.md)

## Phase 4: GDPR/PII Handling

### 4.1 Data Minimization
- [ ] Task: Audit data collection for EU instances
- [ ] Task: Implement minimal data retention
- [ ] Task: Remove unnecessary PII storage
- [ ] Task: Write tests for data minimization
- [ ] Task: Conductor - User Manual Verification 'Phase 4.1: Minimization' (Protocol in workflow.md)

### 4.2 Right to Erasure
- [ ] Task: Implement data deletion for EU users
- [ ] Task: Add "forget me" functionality
- [ ] Task: Test complete data removal
- [ ] Task: Document erasure process
- [ ] Task: Conductor - User Manual Verification 'Phase 4.2: Erasure' (Protocol in workflow.md)

### 4.3 Consent Management
- [ ] Task: Implement consent tracking for EU instances
- [ ] Task: Add explicit consent prompts where required
- [ ] Task: Store consent records
- [ ] Task: Allow consent withdrawal
- [ ] Task: Conductor - User Manual Verification 'Phase 4.3: Consent' (Protocol in workflow.md)

### 4.4 PII Redaction
- [ ] Task: Enhance PII detection for European names/data
- [ ] Task: Implement redaction for EU privacy rules
- [ ] Task: Test redaction accuracy
- [ ] Task: Conductor - User Manual Verification 'Phase 4.4: Redaction' (Protocol in workflow.md)

### 4.5 Privacy Impact Assessment
- [ ] Task: Conduct GDPR privacy impact assessment
- [ ] Task: Document data flows for EU instances
- [ ] Task: Identify and mitigate privacy risks
- [ ] Task: Create GDPR compliance checklist
- [ ] Task: Conductor - User Manual Verification 'Phase 4.5: PIA' (Protocol in workflow.md)

## Phase 5: Multi-Language Integration Testing

### 5.1 Locale Switching
- [ ] Task: Test switching between English and non-English instances
- [ ] Task: Verify correct locale selection
- [ ] Task: Test fallback behavior
- [ ] Task: Conductor - User Manual Verification 'Phase 5.1: Switching' (Protocol in workflow.md)

### 5.2 Cross-Locale Testing
- [ ] Task: Run test suite across all locales
- [ ] Task: Verify data isolation per locale
- [ ] Task: Test concurrent multi-locale operations
- [ ] Task: Conductor - User Manual Verification 'Phase 5.2: Cross-Locale' (Protocol in workflow.md)

### 5.3 Character Encoding
- [ ] Task: Test Unicode handling across all locales
- [ ] Task: Verify diacritics and special characters
- [ ] Task: Test German umlauts, French accents, Spanish tildes
- [ ] Task: Conductor - User Manual Verification 'Phase 5.3: Encoding' (Protocol in workflow.md)

## Phase 6: Documentation & Legal Review

### 6.1 Legal Verification
- [ ] Task: Legal expert review of German templates
- [ ] Task: Legal expert review of French templates
- [ ] Task: Legal expert review of Spanish templates
- [ ] Task: Document legal review process
- [ ] Task: Conductor - User Manual Verification 'Phase 6.1: Legal Review' (Protocol in workflow.md)

### 6.2 Translation Documentation
- [ ] Task: Document translation guidelines
- [ ] Task: Create glossary for translators
- [ ] Task: Document cultural considerations per locale
- [ ] Task: Conductor - User Manual Verification 'Phase 6.2: Translation Docs' (Protocol in workflow.md)

### 6.3 User Guides
- [ ] Task: Create German user guide
- [ ] Task: Create French user guide
- [ ] Task: Create Spanish user guide
- [ ] Task: Document locale selection process
- [ ] Task: Conductor - User Manual Verification 'Phase 6.3: User Guides' (Protocol in workflow.md)

## Completion Criteria
- [ ] All phases complete
- [ ] 3 non-English instances functional (DE, FR, ES)
- [ ] All templates native-speaker verified
- [ ] GDPR compliance implemented
- [ ] Privacy impact assessment complete
- [ ] All tests passing across all locales
- [ ] Documentation complete in all languages

## Track History
- **2026-07-08**: Track created for multi-jurisdictional expansion
