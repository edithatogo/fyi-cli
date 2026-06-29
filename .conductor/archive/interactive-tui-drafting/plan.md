# Plan: interactive-tui-drafting

## Phase 1: Inline Markdown Editing

### Task 1.1: Request editor view
- [x] Add new `Tab::Editor` variant and navigation
- [x] Build request editor view with title/body/tags fields
- [x] Add text input widget for inline editing in ratatui
- [x] Wire editor to fyi-core DB for save/load
- [x] Commit: `feat(tui): add request editor view with inline editing` [18f3d8f]

### Task 1.2: Markdown preview and auto-save
- [x] Add markdown preview split-pane (edit left, preview right)
- [x] Implement debounced auto-save to SQLite
- [x] Add draft management (list, open, save, discard)
- [x] Commit: `feat(tui): add markdown preview and auto-save for request editing` [f54eba9]

### Task 1.3: Keyboard shortcuts for editor
- [x] Define consistent keybindings (Ctrl+S save, Ctrl+Q quit, Tab switch)
- [x] Update `handle_key_event()` for editor mode
- [x] Show keybinding hints in status bar
- [x] Commit: `feat(tui): add keyboard shortcuts and status bar hints` [52c038c]

## Phase 2: Credential Switching Dialogs

### Task 2.1: Credential manager dialog
- [x] Build modal dialog for credential management
- [x] List saved credentials from keyring
- [x] Add credential switching (activate different account)
- [x] Commit: `feat(tui): add credential manager dialog` [42d025f]

### Task 2.2: Credential testing and session status
- [x] Add "Test Credential" button that verifies against FYI API
- [x] Display current active account and session status in header
- [x] Commit: `feat(tui): add credential testing and session status display` [7e48ea1]

## Phase 3: Keyring Management Dashboard

### Task 3.1: Keyring browser
- [x] Build keyring browser view (list stored credentials)
- [x] Show key metadata (service, username, created, last used)
- [x] Commit: `feat(tui): add keyring browser dashboard` [b16c479]

### Task 3.2: Keyring management actions
- [x] Add/Edit/Delete keyring entries from TUI
- [x] Add encryption key management view
- [x] Show security indicators (key strength, age, rotation status)
- [x] Commit: `feat(tui): add keyring management actions and security indicators` [cd50c3b]

## Phase 4: Enhanced Navigation & UX

### Task 4.1: Fuzzy search
- [x] Add `/` command to open fuzzy search
- [x] Search across requests, authorities, and logs
- [x] Show search results with navigation
- [x] Commit: `feat(tui): add fuzzy search across all entities` [c8d389b]

### Task 4.2: Bulk operations and help system
- [x] Add multi-select mode for requests
- [x] Add bulk status update action
- [x] Add export trigger actions from TUI
- [x] Add help overlay (Ctrl+H/F1 shows keybindings)
- [x] Commit: `feat(tui): add bulk operations, export triggers, and help system` [a2fa5e6]

### Task 4.3: Conductor review
- [x] Run conductor-review for interactive-tui-drafting track
- [x] Apply any fix recommendations
- [x] Push to GitHub
- [x] Commit: `conductor(track): complete interactive-tui-drafting after review`

## Archive
- [x] Archive track: move to archive/ directory
