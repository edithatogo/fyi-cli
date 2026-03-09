# Plan: Webapp Coverage to 95%

## Phase 1: Route Tests

### 1.1 Basic Route Tests
- [ ] Task: Test GET / (dashboard)
- [ ] Task: Test GET /requests (request list)
- [ ] Task: Test GET /requests/new (new request form)
- [ ] Task: Test GET /authorities (authorities list)
- [ ] Task: Test GET /api/dashboard (JSON API)

### 1.2 Request Detail Routes
- [ ] Task: Test GET /requests/{id} (request detail)
- [ ] Task: Test GET /requests/{id}/timeline (timeline view)
- [ ] Task: Test GET /requests/{id}/correspondence (correspondence pack)
- [ ] Task: Test GET /requests/{id}/export-bundle (export)
- [ ] Task: Test 404 for non-existent requests

### 1.3 Error Handling
- [ ] Task: Test 404 handling for invalid routes
- [ ] Task: Test error page rendering
- [ ] Task: Test error logging

## Phase 2: Form Handling Tests

### 2.1 Request Creation
- [ ] Task: Test POST /requests (create request)
- [ ] Task: Test form validation errors
- [ ] Task: Test success redirect
- [ ] Task: Test with various input combinations

### 2.2 Status Updates
- [ ] Task: Test POST /requests/{id}/status (update status)
- [ ] Task: Test status transition validation
- [ ] Task: Test error handling

### 2.3 Authority Import
- [ ] Task: Test POST /authorities/import (CSV import)
- [ ] Task: Test multipart form data parsing
- [ ] Task: Test CSV validation
- [ ] Task: Test import success/failure

### 2.4 Search and Filter
- [ ] Task: Test GET /requests?q=search (search)
- [ ] Task: Test GET /requests?priority=high (filter)
- [ ] Task: Test GET /authorities?q=search (authority search)

## Phase 3: HTML Rendering Tests

### 3.1 Dashboard Rendering
- [ ] Task: Test dashboard with requests
- [ ] Task: Test dashboard with empty state
- [ ] Task: Test dashboard statistics
- [ ] Task: Test priority indicators

### 3.2 Request List Rendering
- [ ] Task: Test request list with data
- [ ] Task: Test request list pagination
- [ ] Task: Test status badges
- [ ] Task: Test priority indicators

### 3.3 Request Detail Rendering
- [ ] Task: Test request detail page
- [ ] Task: Test timeline display
- [ ] Task: Test correspondence pack display
- [ ] Task: Test action buttons

### 3.4 Security Rendering
- [ ] Task: Test privacy redaction in HTML
- [ ] Task: Test security headers present
- [ ] Task: Test no PII in page source

## Phase 4: Security & Integration Tests

### 4.1 Security Headers
- [ ] Task: Test Cache-Control header
- [ ] Task: Test Content-Security-Policy header
- [ ] Task: Test X-Content-Type-Options header
- [ ] Task: Test Referrer-Policy header

### 4.2 Integration Tests
- [ ] Task: Test full request lifecycle via web
- [ ] Task: Test create → update → export flow
- [ ] Task: Test search → view → update flow

## Phase 5: Verification & Documentation

### 5.1 Coverage Verification
- [ ] Task: Run coverage report
- [ ] Task: Verify 95% coverage achieved
- [ ] Task: Document any excluded lines

### 5.2 Documentation
- [ ] Task: Document webapp testing approach
- [ ] Task: Add testing examples
- [ ] Task: Update TESTING_STRATEGY.md

---

## Completion Criteria
- [ ] All phases complete
- [ ] webapp.py coverage ≥95%
- [ ] All tests passing
- [ ] Test execution time <2 minutes
- [ ] Documentation updated

## Track History
- **2026-03-09**: Track created from research-grade-quality split
