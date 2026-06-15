# Plan: Webapp Coverage to 95%

## Phase 1: Route Tests

### 1.1 Basic Route Tests
- [x] Task: Test GET / (dashboard)
- [x] Task: Test GET /requests (request list)
- [x] Task: Test GET /requests/new (new request form)
- [x] Task: Test GET /authorities (authorities list)
- [x] Task: Test GET /api/dashboard (JSON API)

### 1.2 Request Detail Routes
- [x] Task: Test GET /requests/{id} (request detail)
- [x] Task: Test GET /requests/{id}/timeline (timeline view)
- [x] Task: Test GET /requests/{id}/correspondence (correspondence pack)
- [x] Task: Test GET /requests/{id}/export-bundle (export)
- [x] Task: Test 404 for non-existent requests

### 1.3 Error Handling
- [x] Task: Test 404 handling for invalid routes
- [x] Task: Test error page rendering
- [x] Task: Test error logging

## Phase 2: Form Handling Tests

### 2.1 Request Creation
- [x] Task: Test POST /requests (create request)
- [x] Task: Test form validation errors
- [x] Task: Test success redirect
- [x] Task: Test with various input combinations

### 2.2 Status Updates
- [x] Task: Test POST /requests/{id}/status (update status)
- [x] Task: Test status transition validation
- [x] Task: Test error handling

### 2.3 Authority Import
- [x] Task: Test POST /authorities/import (CSV import)
- [x] Task: Test multipart form data parsing
- [x] Task: Test CSV validation
- [x] Task: Test import success/failure

### 2.4 Search and Filter
- [x] Task: Test GET /requests?q=search (search)
- [x] Task: Test GET /requests?priority=high (filter)
- [x] Task: Test GET /authorities?q=search (authority search)

## Phase 3: HTML Rendering Tests

### 3.1 Dashboard Rendering
- [x] Task: Test dashboard with requests
- [x] Task: Test dashboard with empty state
- [x] Task: Test dashboard statistics
- [x] Task: Test priority indicators

### 3.2 Request List Rendering
- [x] Task: Test request list with data
- [x] Task: Test request list pagination
- [x] Task: Test status badges
- [x] Task: Test priority indicators

### 3.3 Request Detail Rendering
- [x] Task: Test request detail page
- [x] Task: Test timeline display
- [x] Task: Test correspondence pack display
- [x] Task: Test action buttons

### 3.4 Security Rendering
- [x] Task: Test privacy redaction in HTML
- [x] Task: Test security headers present
- [x] Task: Test no PII in page source

## Phase 4: Security & Integration Tests

### 4.1 Security Headers
- [x] Task: Test Cache-Control header
- [x] Task: Test Content-Security-Policy header
- [x] Task: Test X-Content-Type-Options header
- [x] Task: Test Referrer-Policy header

### 4.2 Integration Tests
- [x] Task: Test full request lifecycle via web
- [x] Task: Test create → update → export flow
- [x] Task: Test search → view → update flow

## Phase 5: Verification & Documentation

### 5.1 Coverage Verification
- [x] Task: Run coverage report
- [x] Task: Verify 95% coverage achieved
- [x] Task: Document any excluded lines

### 5.2 Documentation
- [x] Task: Document webapp testing approach
- [x] Task: Add testing examples
- [x] Task: Update TESTING_STRATEGY.md

---

## Completion Criteria
- [x] All phases complete
- [x] webapp.py coverage ≥95%
- [x] All tests passing
- [x] Test execution time <2 minutes
- [x] Documentation updated

## Track History
- **2026-03-09**: Track created from research-grade-quality split
