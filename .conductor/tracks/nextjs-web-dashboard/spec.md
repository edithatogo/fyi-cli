# Specification: nextjs-web-dashboard

## Overview
Build a modern, highly aesthetic Next.js web interface featuring interactive charting and TailwindCSS to replace the basic Python HTTP UI (`webapp.py`). The dashboard communicates with the Rust core service through the MCP server (JSON-RPC 2.0 over stdin/stdout), providing a professional-grade interface for managing OIA requests.

## Functional Requirements

### Phase 1: Next.js Scaffold & TailwindCSS Foundation
1. **Project Scaffold:** Initialize Next.js 14+ project with TypeScript, App Router, and TailwindCSS
2. **Layout System:** Responsive layout with sidebar navigation, top header, and content area
3. **Design System:** Custom TailwindCSS design tokens matching FYI brand (privacy-focused, NZ-themed)
4. **Dark/Light Mode:** Theme toggle with system preference detection

### Phase 2: MCP Client Layer & API Integration
1. **MCP Client:** TypeScript client that spawns and communicates with `fyi-mcp` over JSON-RPC 2.0
2. **Request CRUD:** List, create, view, update, and delete tracked requests
3. **Authority Management:** Browse and import authorities
4. **Status Management:** Update request statuses with visual indicators

### Phase 3: Interactive Dashboard & Charting
1. **Summary Dashboard:** KPI cards (total requests, attention-needed, overdue, authorities count)
2. **Interactive Charts:** Request status distribution (pie/donut), request timeline (bar/line), attention heatmap
3. **Real-time Updates:** Polling-based updates from MCP server
4. **Export Actions:** One-click JSON/CSV export from dashboard

### Phase 4: Request Detail & Timeline Views
1. **Request Detail Page:** Full request view with correspondence timeline
2. **Inline Editing:** Edit request fields inline with auto-save
3. **Timeline Visualization:** Visual timeline of request lifecycle events
4. **Attachment Previews:** Link to correspondence attachments

### Phase 5: Advanced Features & Polish
1. **Search & Filtering:** Full-text search, status/authority filters, date range picker
2. **Bulk Actions:** Batch status updates, bulk export
3. **Responsive Design:** Mobile-friendly views
4. **Accessibility:** ARIA labels, keyboard navigation, screen reader support

## Non-Functional Requirements
- **Performance:** Initial load < 2s, subsequent navigation < 500ms
- **Accessibility:** WCAG 2.1 AA compliant
- **Browser Support:** Modern browsers (Chrome, Firefox, Safari, Edge)
- **Security:** No API keys/tokens in client-side code; all API calls proxied through MCP server
- **Offline Graceful Degradation:** Show cached data when MCP server is unreachable

## Acceptance Criteria
- Next.js app builds and runs with `npm run dev`
- Dashboard displays real request data from SQLite via MCP
- Charts render with interactive tooltips
- All CRUD operations work end-to-end
- Dark/light mode functions correctly
- Search and filtering return accurate results
- Mobile layout is usable and responsive