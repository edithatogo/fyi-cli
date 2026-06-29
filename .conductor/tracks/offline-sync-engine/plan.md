# Plan: offline-sync-engine

## Phase 1: Sync State Tracking

### Task 1.1: Sync metadata table and dirty flagging
- [x] Create SQL migration for `sync_metadata` table (request_id, last_synced_at, remote_updated_at, local_updated_at, sync_status, conflict_version)
- [x] Implement dirty flagging wrapper in DbPool
- [x] Implement field-level change tracking
- [x] Add sync_status query API (per-request and global)
- [x] Commit: `feat(sync): add sync metadata tracking and dirty flagging`

### Task 1.2: Sync state API
- [x] Expose sync state via MCP tools
- [x] Add CLI command `fyi sync status`
- [x] Add sync status display in TUI
- [x] Commit: `feat(sync): add sync status API, CLI command, and TUI display`

## Phase 2: Pull Synchronization

### Task 2.1: Incremental pull from FYI API
- [x] Implement pull function that fetches updates since last_synced_at
- [x] Add RSS/Atom feed integration for watched requests
- [x] Apply remote updates to local database
- [x] Commit: `feat(sync): implement incremental pull synchronization from FYI API`

### Task 2.2: Pull scheduler and configuration
- [x] Add configurable pull interval (default 5 minutes)
- [x] Implement Tokio-based background pull task
- [x] Add `fyi sync pull` manual trigger command
- [x] Commit: `feat(sync): add pull scheduler and manual trigger`

## Phase 3: Push Synchronization

### Task 3.1: Dirty record push
- [x] Implement push function that sends dirty records to FYI API
- [x] Build outgoing queue for pending submissions
- [x] Record FYI-issued request IDs after successful push
- [x] Commit: `feat(sync): implement push synchronization for dirty records`

### Task 3.2: Retry logic and queue management
- [x] Add exponential backoff retry (max 3 retries)
- [x] Add queue depth monitoring and management
- [x] Add `fyi sync push` manual trigger command
- [x] Commit: `feat(sync): add retry logic and queue management`

## Phase 4: Conflict Resolution

### Task 4.1: Last-write-wins and three-way merge
- [ ] Implement default Last-Write-Wins resolution
- [ ] Implement field-level three-way merge for non-conflicting changes
- [ ] Mark records with conflicting changes
- [ ] Commit: `feat(sync): add conflict resolution (LWW and three-way merge)`

### Task 4.2: Conflict review and resolution UI
- [ ] Add `fyi sync conflicts` CLI command to list conflicts
- [ ] Add conflict resolution view in TUI
- [ ] Add MCP tools for conflict management
- [ ] Commit: `feat(sync): add conflict review and resolution interface`

## Phase 5: Scheduler & Monitoring

### Task 5.1: Background scheduler
- [ ] Implement Tokio-based background sync scheduler
- [ ] Add graceful shutdown handling
- [ ] Add connectivity detection and API reachability probes
- [ ] Commit: `feat(sync): add background sync scheduler with health checks`

### Task 5.2: Sync dashboard and graceful degradation
- [ ] Build sync dashboard in TUI (status, queue depth, last sync time)
- [ ] Add MCP tools for sync monitoring
- [ ] Ensure graceful offline degradation (queue changes for later)
- [ ] Commit: `feat(sync): add sync dashboard and graceful offline operation`

### Task 5.3: Conductor review
- [ ] Run conductor-review for offline-sync-engine track
- [ ] Apply any fix recommendations
- [ ] Push to GitHub
- [ ] Commit: `conductor(track): complete offline-sync-engine after review`

## Archive
- [ ] Archive track: move to archive/ directory
