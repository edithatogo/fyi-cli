# Plan: nextjs-web-dashboard

## Phase 1: Next.js Scaffold & TailwindCSS Foundation

### Task 1.1: Initialize Next.js project
- [x] Create Next.js 14+ project with TypeScript, App Router in `dashboard/` [11bf304]
- [x] Configure TailwindCSS with custom design tokens [COMMIT_SHA]
- [x] Add base layout (sidebar, header, content area) [COMMIT_SHA]
- [x] Add dark/light mode with system preference detection [COMMIT_SHA]
- [x] Commit: `feat(dashboard): scaffold Next.js project with TailwindCSS and dark mode`

### Task 1.2: Design system components
- [ ] Build reusable UI components (Button, Card, Badge, Input, Select, Table)
- [ ] Build navigation sidebar with icons (done as part of 1.1)
- [ ] Build KPI stat card component (done as part of 1.1)
- [ ] Commit: `feat(dashboard): add design system components and layout`

## Phase 2: MCP Client Layer & API Integration

### Task 2.1: MCP client library
- [ ] Create TypeScript MCP client class that spawns `fyi-mcp` process
- [ ] Implement JSON-RPC 2.0 transport over stdin/stdout
- [ ] Add request/response types matching Rust API contracts
- [ ] Write unit tests for MCP client
- [ ] Commit: `feat(dashboard): create MCP client for Rust backend communication`

### Task 2.2: Request CRUD pages
- [ ] Build requests list page with server-side data fetching via MCP
- [ ] Build request creation form with authority selector
- [ ] Build request detail page with inline editing
- [ ] Build request deletion with confirmation
- [ ] Commit: `feat(dashboard): implement request CRUD operations via MCP`

### Task 2.3: Authority management
- [ ] Build authorities browse page
- [ ] Build authority import page (CSV upload)
- [ ] Commit: `feat(dashboard): add authority management pages`

## Phase 3: Interactive Dashboard & Charting

### Task 3.1: Summary dashboard with KPIs
- [ ] Build summary dashboard page with KPI cards
- [ ] Fetch dashboard data from MCP server
- [ ] Add auto-refresh polling (configurable interval)
- [ ] Commit: `feat(dashboard): add summary dashboard with KPI cards`

### Task 3.2: Interactive charts
- [ ] Add charting library (Chart.js or Recharts)
- [ ] Build status distribution pie/donut chart
- [ ] Build request timeline bar/line chart
- [ ] Build attention heatmap or trends chart
- [ ] Commit: `feat(dashboard): add interactive charts for request analytics`

### Task 3.3: Export actions
- [ ] Add JSON export button to dashboard
- [ ] Add CSV export button to dashboard
- [ ] Commit: `feat(dashboard): add one-click export actions`

## Phase 4: Request Detail & Timeline

### Task 4.1: Timeline visualization
- [ ] Build correspondence timeline component
- [ ] Fetch and display request lifecycle events
- [ ] Add visual status indicators on timeline
- [ ] Commit: `feat(dashboard): add request timeline visualization`

### Task 4.2: Inline editing & attachments
- [ ] Implement inline field editing with auto-save
- [ ] Add attachment link previews
- [ ] Commit: `feat(dashboard): add inline editing and attachment previews`

## Phase 5: Advanced Features & Polish

### Task 5.1: Search and filtering
- [ ] Add full-text search across requests
- [ ] Add status/authority filters
- [ ] Add date range picker for filtering
- [ ] Commit: `feat(dashboard): add search and filtering capabilities`

### Task 5.2: Bulk actions and responsive design
- [ ] Implement bulk status update (multi-select)
- [ ] Implement bulk export
- [ ] Make all views mobile-responsive
- [ ] Add accessibility labels and keyboard navigation
- [ ] Commit: `feat(dashboard): add bulk actions, responsive design, and accessibility`

### Task 5.3: Conductor review
- [ ] Run conductor-review for nextjs-web-dashboard track
- [ ] Apply any fix recommendations
- [ ] Push to GitHub
- [ ] Commit: `conductor(track): complete nextjs-web-dashboard after review`

## Archive
- [ ] Archive track: move to archive/ directory
