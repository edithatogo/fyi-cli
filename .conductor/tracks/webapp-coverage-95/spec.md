# Track Specification: Webapp Coverage to 95%

## Overview
This track focuses exclusively on improving webapp.py test coverage from 63% to 95% by adding comprehensive route tests, form handling tests, and HTML rendering tests.

## Current State
- **webapp.py Coverage:** 63% (283 statements, 104 missing)
- **Missing Coverage:** Routes, form handling, HTML rendering, error handling
- **Priority:** HIGH - webapp is the primary user interface

## Functional Requirements

### 1. Route Tests
- [ ] Test all GET routes (/, /requests, /request/<id>, /authorities)
- [ ] Test all POST routes (import, status update, new request)
- [ ] Test JSON API endpoints (/api/dashboard)
- [ ] Test 404 error handling
- [ ] Test redirect behavior

### 2. Form Handling Tests
- [ ] Test request creation form
- [ ] Test status update form
- [ ] Test authority import (CSV upload)
- [ ] Test form validation errors
- [ ] Test multipart form data parsing

### 3. HTML Rendering Tests
- [ ] Test dashboard rendering with data
- [ ] Test request list rendering
- [ ] Test request detail rendering
- [ ] Test correspondence pack rendering
- [ ] Test timeline rendering
- [ ] Test authorities list rendering

### 4. Security Tests
- [ ] Test security headers (Cache-Control, CSP, etc.)
- [ ] Test privacy redaction in web output
- [ ] Test PII protection in logs
- [ ] Test input sanitization

## Non-Functional Requirements
- **Test Execution Time:** <2 minutes for webapp tests
- **Coverage Target:** 95% (from current 63%)
- **No Regressions:** All existing tests must continue passing

## Acceptance Criteria
1. ✅ webapp.py coverage: 63% → 95%
2. ✅ All webapp routes tested
3. ✅ All form handling tested
4. ✅ Security headers verified
5. ✅ All tests passing
6. ✅ Test execution time <2 minutes

## Out of Scope
- Other modules (already covered by other tracks)
- Performance optimization
- UI/UX changes
- Backend API changes

## Success Metrics
- **Coverage:** 95% (from 63%)
- **Test Count:** +40-50 new webapp tests
- **Statements Covered:** +89 statements (from 179 to 268)

## Dependencies
- pytest (installed)
- pytest-cov (installed)
- Existing test infrastructure

## Risks
- **Risk:** HTML rendering tests are brittle
- **Mitigation:** Test structure, not exact HTML
- **Risk:** Form parsing is complex
- **Mitigation:** Use existing test utilities

## Timeline Estimate
- **Phase 1 (Route Tests):** 1-2 days
- **Phase 2 (Form Tests):** 1-2 days
- **Phase 3 (Rendering Tests):** 1-2 days
- **Total:** 3-6 days

## Priority
**HIGH** - webapp is the primary user interface and has the lowest coverage of all critical modules.
