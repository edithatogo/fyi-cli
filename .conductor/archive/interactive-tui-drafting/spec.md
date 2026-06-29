# Specification: interactive-tui-drafting

## Overview
Enhance the Ratatui TUI dashboard (`fyi-cli::tui`) to support inline markdown request editing, credential switching dialogs, and keyring management dashboards. This transforms the TUI from a read-only dashboard into an interactive management console.

## Functional Requirements

### Phase 1: Inline Markdown Editing
1. **Request Editor View:** New TUI tab/view for editing request title, body, and tags
2. **Markdown Support:** Syntax-highlighted markdown editing with preview split-pane
3. **Auto-save:** Debounced auto-save to SQLite via fyi-core
4. **Draft Management:** List, open, edit, save, discard drafts
5. **Keyboard Shortcuts:** Consistent keybindings (Ctrl+S save, Ctrl+Q quit, Tab switch panes)

### Phase 2: Credential Switching Dialogs
1. **Credential Manager Dialog:** Modal/popup dialog for managing FYI.org.nz credentials
2. **Account Switching:** Switch between multiple FYI.org.nz accounts from within TUI
3. **Credential Test:** Test credential validity against FYI API
4. **Session Status:** Display current active account and session expiry

### Phase 3: Keyring Management Dashboard
1. **Keyring Browser:** Browse stored credentials in OS keyring
2. **Add/Edit/Remove:** Manage keyring entries from TUI
3. **Encryption Key Management:** View/re-encrypt/rotate encryption keys
4. **Security Indicators:** Visual indicators for key strength, last rotation, age

### Phase 4: Enhanced Navigation & UX
1. **Search within TUI:** Fuzzy-find requests, authorities, and logs
2. **Status Bulk-Update:** Multi-select requests and batch-update status
3. **Export Triggers:** Trigger JSON/CSV/PDF exports from TUI
4. **Help System:** Built-in keybinding reference (Ctrl+H or F1)

## Non-Functional Requirements
- **Responsiveness:** All operations < 100ms perceived latency
- **Keyboard-Navigable:** Complete operation without mouse
- **Consistency:** Follow existing ratatui patterns and color schemes
- **Testability:** All new components unit-testable with `TestBackend`

## Acceptance Criteria
- Open and edit a request body with markdown preview side-by-side
- Switch between saved credentials and verify active account
- Browse, add, and delete keyring entries from TUI
- Search requests with fuzzy matching
- All operations work with keyboard only
- Render tests pass for all new TUI components