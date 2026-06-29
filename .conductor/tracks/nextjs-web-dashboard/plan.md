# Plan: nextjs-web-dashboard

## Phase 1: Next.js Scaffold & TailwindCSS Foundation

### Task 1.1: Initialize Next.js project
- [x] Create Next.js 14+ project with TypeScript, App Router in `dashboard/` [11bf304]
- [x] Configure TailwindCSS with custom design tokens [f77df17]
- [x] Add base layout (sidebar, header, content area) [f77df17]
- [x] Add dark/light mode with system preference detection [f77df17]
- [x] Commit: `feat(dashboard): scaffold Next.js project with TailwindCSS and dark mode` [f77df17]

### Task 1.2: Design system components
- [x] Build reusable UI components (Button, Card, Badge, Input, Select, Table)
- [x] Build navigation sidebar with icons (done as part of 1.1)
- [x] Build KPI stat card component (done as part of 1.1)
- [x] Commit: `feat(dashboard): add design system components and layout` [105bf3a]

## Phase 2: MCP Client Layer & API Integration

### Task 2.1: MCP client library
- [x] Create TypeScript MCP client class that spawns `fyi-mcp` process
- [x] Implement JSON-RPC 2.0 transport over stdin/stdout
- [x] Add request/response types matching Rust API contracts
- [x] Write unit tests for MCP client
- [x] Commit: `feat(dashboard): create MCP client for Rust backend communication` [b1c629b]

### Task 2.2: Request CRUD pages
- [x] Build requests list page with server-side data fetching via MCP [d9f7dca]
- [x] Build request creation form with authority selector
- [x] Build request detail page with inline editing
- [x] Build request deletion with confirmation
- [x] Commit: `feat(dashboard): implement request CRUD operations via MCP`

### Task 2.3: Authority management
- [x] Build authorities browse page
- [x] Build authority import page (CSV upload)
- [x] Commit: `feat(dashboard): add authority management pages`

## Phase 3: Interactive Dashboard & Charting

### Task 3.1: Summary dashboard with KPIs
- [x] Build summary dashboard page with KPI cards [f7f7a8c]
- [x] Fetch dashboard data from MCP server [c37ac4d]
- [x] Add auto-refresh polling (configurable interval) [5742b77]
- [x] Commit: `feat(dashboard): add summary dashboard with KPI cards`

### Task 3.2: Interactive charts
- [x] Add charting library (Chart.js or Recharts) [94f771e]
- [x] Build status distribution pie/donut chart [0f105ee]
- [x] Build request timeline bar/line chart [a22be28]
- [x] Build attention heatmap or trends chart [f7870ae]
- [x] Commit: `feat(dashboard): add interactive charts for request analytics`

### Task 3.3: Export actions
- [x] Add JSON export button to dashboard [29e076c]
- [x] Add CSV export button to dashboard [af6cd93]
- [x] Commit: `feat(dashboard): add one-click export actions` [af6cd93]

## Phase 4: Request Detail & Timeline

### Task 4.1: Timeline visualization
- [x] Build correspondence timeline component [4d6e304]
- [x] Fetch and display request lifecycle events [5a1d4fb]
- [x] Add visual status indicators on timeline [87180a8]
- [x] Commit: `feat(dashboard): add request timeline visualization` [87180a8]

### Task 4.2: Inline editing & attachments
- [x] Implement inline field editing with auto-save [5657128]
- [x] Add attachment link previews [8b9731b]
- [x] Commit: `feat(dashboard): add inline editing and attachment previews` [8b9731b]

## Phase 5: Advanced Features & Polish

### Task 5.1: Search and filtering
- [x] Add full-text search across requests [a41a4f0]
- [x] Add status/authority filters [b7e72fb]
- [x] Add date range picker for filtering [650ad63]
- [x] Commit: `feat(dashboard): add search and filtering capabilities` [650ad63]

### Task 5.2: Bulk actions and responsive design
- [x] Implement bulk status update (multi-select) [c5936b6]
- [x] Implement bulk export [0b71841]
- [x] Make all views mobile-responsive [0b71841]
- [x] Add accessibility labels and keyboard navigation [0b71841]
- [x] Commit: `feat(dashboard): add bulk actions, responsive design, and accessibility` [0b71841]

### Task 5.3: Conductor review
- [x] Run conductor-review for nextjs-web-dashboard track
- [x] Apply any fix recommendations [a19142a]
- [x] Push to GitHub [58f3f59]
- [x] Commit: `conductor(track): complete nextjs-web-dashboard after review`

## Archive
- [ ] Archive track: move to archive/ directory
