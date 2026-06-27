# Specification: offline-sync-engine

## Overview
Design and implement a SQLite sync service to handle periodic database caching, OIA request tracking, and conflict reconciliation with the upstream FYI API. This enables the system to operate fully offline while keeping data synchronized when connectivity is available.

## Functional Requirements

### Phase 1: Sync State Tracking
1. **Sync Metadata Table:** New SQLite table tracking sync state per request (last_synced_at, remote_updated_at, local_updated_at, sync_status, conflict_version)
2. **Dirty Flagging:** Mark locally changed records as "dirty" for sync
3. **Change Tracking:** Track field-level changes for granular conflict resolution
4. **Sync Status API:** Query sync status for any request or globally

### Phase 2: Pull Synchronization
1. **Periodic Pull:** Configurable interval (default: 5 min) to fetch updates from FYI API
2. **Incremental Sync:** Only fetch records updated since last sync using `updated_at` timestamps
3. **Feed Integration:** Pull updates from RSS/Atom feeds for watched requests
4. **Status Reconciliation:** Update local request status from remote data

### Phase 3: Push Synchronization
1. **Dirty Record Push:** Sync locally-created/updated requests to FYI API
2. **Queue Management:** Outgoing queue for pending submissions
3. **Retry Logic:** Exponential backoff for failed pushes (max 3 retries)
4. **Submission Confirmation:** Record FYI-issued request IDs after successful push

### Phase 4: Conflict Resolution
1. **Last-Write-Wins (Default):** Simple timestamp-based conflict resolution
2. **Three-Way Merge:** Field-level merge for non-conflicting changes
3. **Conflict Marking:** Flag records with conflicting changes for manual resolution
4. **Conflict Review UI:** CLI command and TUI view for reviewing/resolving conflicts

### Phase 5: Scheduler & Monitoring
1. **Background Scheduler:** Tokio-based background task for periodic sync
2. **Sync Dashboard:** MCP tool and TUI view showing sync status, queue depth, last sync time
3. **Health Checks:** Network connectivity detection, API reachability probes
4. **Graceful Degradation:** Continue operating offline; queue changes for later sync

## Non-Functional Requirements
- **Reliability:** No data loss on crash; sync state is persistent
- **Performance:** Sync operations non-blocking; use background Tokio tasks
- **Network Efficiency:** Minimize API calls; use ETags/If-Modified-Since where possible
- **Conflict Safety:** Never overwrite local data without confirmation in case of conflicts

## Acceptance Criteria
- Records marked dirty after local changes
- Pull sync fetches and applies remote updates
- Push sync sends local changes to FYI API
- Conflicts detected and flagged for resolution
- Background sync runs on configurable schedule
- System operates fully offline with queued changes
- All sync operations have >90% test coverage