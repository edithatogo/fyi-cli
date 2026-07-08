# Specification: i18n-localization-framework

## Overview
This track implements comprehensive internationalization (i18n) and localization (l10n) infrastructure using `fluent-rs`. It enables locale-aware request-letter templates, terminology mapping, and working-day/deadline calculation engines with holiday calendars per jurisdiction.

## Functional Requirements
1. **Fluent-rs Integration:**
   - Integrate `fluent-rs` for i18n message handling
   - Create locale-aware message bundles
   - Support fallback locale chains (e.g., en-NZ → en → base)
2. **Request Letter Templates:**
   - Locale-aware salutations (Dear Sir/Madam, Kia ora, etc.)
   - Legal citation templates per jurisdiction
   - Professional sign-offs appropriate to locale
   - Template variable substitution (names, authorities, dates)
3. **Terminology Mapping:**
   - Map FOI terms across jurisdictions: OIA (NZ) ↔ FOI (AU/UK) ↔ RTI (AU states) ↔ IFG (DE)
   - Support jurisdiction-specific legal language
   - Maintain term glossary with translations
4. **Working-Day/Deadline Engine:**
   - Per-country/state working-day calculator
   - Holiday calendar support (public holidays vary by jurisdiction)
   - Statutory deadline computation (e.g., 20 working days from receipt)
   - Weekend/holiday skipping logic
   - Support for jurisdiction-specific business day rules
5. **Locale Data Management:**
   - Embed locale data files in binary
   - Support external locale overrides
   - Version locale data separately from code

## Non-Functional Requirements
- **Coverage:** Support for all target locales (en-NZ, en-AU, en-GB, de-DE, fr-FR, es-ES minimum)
- **Performance:** Template rendering < 10ms, deadline calculation < 1ms
- **Extensibility:** Easy addition of new locales without code changes
- **Accuracy:** Holiday calendars must be accurate and updatable

## Acceptance Criteria
- fluent-rs integrated with message bundles for all target locales
- Request letter templates render correctly in all locales
- Terminology mapping covers all known FOI term variations
- Working-day engine correctly handles holidays for NZ, AU, UK, DE, FR, ES
- Deadline calculations verified against statutory requirements
- All templates tested with realistic data
- Documentation for adding new locales

## Out of Scope
- Right-to-left (RTL) language support (future enhancement)
- Machine translation (all translations must be human-reviewed)
- Non-Latin scripts (Phase 1 focuses on Latin-script languages)

## Dependencies
- Depends on: `jurisdiction-abstraction-core` (track 2)

## Success Metrics
- **Locale Coverage:** 100% of target jurisdictions supported
- **Template Quality:** Human-verified translations for all templates
- **Deadline Accuracy:** 100% accuracy vs. statutory requirements
- **Test Coverage:** 95%+ on i18n module
