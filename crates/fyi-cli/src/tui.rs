use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs},
    Frame,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInput {
    value: String,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn insert_char(&mut self, ch: char) {
        self.value.push(ch);
    }

    pub fn insert_str(&mut self, text: &str) {
        self.value.push_str(text);
    }

    pub fn backspace(&mut self) {
        self.value.pop();
    }

    pub fn replace(&mut self, value: impl Into<String>) {
        self.value = value.into();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Summary,
    Requests,
    Logs,
    Mfa,
    Editor,
    Keyring,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Summary,
            Tab::Requests,
            Tab::Logs,
            Tab::Mfa,
            Tab::Editor,
            Tab::Keyring,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Summary => "Summary Dashboard",
            Tab::Requests => "Tracked Requests",
            Tab::Logs => "Activity Logs",
            Tab::Mfa => "MFA Security",
            Tab::Editor => "Request Editor",
            Tab::Keyring => "Keyring Browser",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorPane {
    Draft,
    Preview,
    Drafts,
}

impl EditorPane {
    fn next(self) -> Self {
        match self {
            Self::Draft => Self::Preview,
            Self::Preview => Self::Drafts,
            Self::Drafts => Self::Draft,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorField {
    Title,
    Body,
    Tags,
}

impl EditorField {
    pub fn label(&self) -> &'static str {
        match self {
            EditorField::Title => "Title",
            EditorField::Body => "Body",
            EditorField::Tags => "Tags",
        }
    }

    fn next(self) -> Self {
        match self {
            EditorField::Title => EditorField::Body,
            EditorField::Body => EditorField::Tags,
            EditorField::Tags => EditorField::Title,
        }
    }

    fn prev(self) -> Self {
        match self {
            EditorField::Title => EditorField::Tags,
            EditorField::Body => EditorField::Title,
            EditorField::Tags => EditorField::Body,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiCommand {
    None,
    SaveDraft,
    CloseEditor,
    TestCredential,
    Quit,
}

#[derive(Debug, Clone)]
pub struct RequestItem {
    pub id: i64,
    pub authority: String,
    pub title: String,
    pub status: String,
    pub needs_attention: bool,
    pub priority: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftItem {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialAccount {
    pub username: String,
}

impl CredentialAccount {
    pub fn new(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyringEntryItem {
    pub kind: String,
    pub username: String,
    pub service: String,
    pub created_at: String,
    pub last_used_at: String,
    pub key_strength: String,
    pub key_age: String,
    pub rotation_status: String,
}

impl KeyringEntryItem {
    pub fn new(
        kind: impl Into<String>,
        username: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            username: username.into(),
            service: service.into(),
            created_at: "unknown".to_string(),
            last_used_at: "unknown".to_string(),
            key_strength: "Unknown".to_string(),
            key_age: "unknown".to_string(),
            rotation_status: "Unknown".to_string(),
        }
    }

    pub fn with_security(
        mut self,
        key_strength: impl Into<String>,
        key_age: impl Into<String>,
        rotation_status: impl Into<String>,
    ) -> Self {
        self.key_strength = key_strength.into();
        self.key_age = key_age.into();
        self.rotation_status = rotation_status.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultItem {
    pub kind: String,
    pub title: String,
    pub detail: String,
}

impl SearchResultItem {
    pub fn new(
        kind: impl Into<String>,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            title: title.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSessionStatus {
    NotTested,
    Verified,
    Missing,
    MfaRequired,
    Error,
}

impl CredentialSessionStatus {
    pub fn label(&self) -> &'static str {
        match self {
            CredentialSessionStatus::NotTested => "Not tested",
            CredentialSessionStatus::Verified => "Verified",
            CredentialSessionStatus::Missing => "Missing",
            CredentialSessionStatus::MfaRequired => "MFA required",
            CredentialSessionStatus::Error => "Error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub active_tab: Tab,
    pub total_tracked: usize,
    pub needs_attention_count: usize,
    pub action_now_count: usize,
    pub authorities_count: usize,
    pub recent_updates: usize,
    pub sync_clean_count: i64,
    pub sync_dirty_count: i64,
    pub sync_pending_count: i64,
    pub sync_conflict_count: i64,
    pub action_now_requests: Vec<RequestItem>,
    pub tracked_requests: Vec<RequestItem>,
    pub selected_request_idx: usize,
    pub logs: Vec<String>,
    pub selected_log_idx: usize,
    pub mfa_enabled_accounts: Vec<String>,
    pub mfa_setup_account: Option<String>,
    pub mfa_session_verified: bool,
    pub editor_request_id: Option<i64>,
    pub editor_title: String,
    pub editor_body: String,
    pub editor_tags: String,
    pub editor_dirty: bool,
    pub editor_last_saved_label: Option<String>,
    pub editor_drafts: Vec<DraftItem>,
    pub selected_editor_draft_idx: usize,
    pub active_editor_pane: EditorPane,
    pub editor_active_field: EditorField,
    pub editor_save_requested: bool,
    pub credential_dialog_open: bool,
    pub credentials: Vec<CredentialAccount>,
    pub selected_credential_idx: usize,
    pub active_credential_account: Option<String>,
    pub credential_session_status: CredentialSessionStatus,
    pub credential_test_message: Option<String>,
    pub credential_test_requested: bool,
    pub keyring_entries: Vec<KeyringEntryItem>,
    pub selected_keyring_entry_idx: usize,
    pub encryption_key_managed: bool,
    pub encryption_key_strength: Option<String>,
    pub encryption_key_rotation_status: Option<String>,
    pub search_open: bool,
    pub search_query: String,
    pub search_results: Vec<SearchResultItem>,
    pub selected_search_result_idx: usize,
    pub request_multi_select: bool,
    pub selected_request_ids: BTreeSet<i64>,
    pub export_actions: Vec<String>,
    pub last_export_message: Option<String>,
    pub help_open: bool,
    pub should_quit: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Summary,
            total_tracked: 42,
            needs_attention_count: 5,
            action_now_count: 3,
            authorities_count: 12,
            recent_updates: 8,
            sync_clean_count: 0,
            sync_dirty_count: 0,
            sync_pending_count: 0,
            sync_conflict_count: 0,
            action_now_requests: vec![
                RequestItem {
                    id: 101,
                    authority: "ministry-of-health".to_string(),
                    title: "COVID-19 Advisory Group Minutes".to_string(),
                    status: "overdue".to_string(),
                    needs_attention: true,
                    priority: "High".to_string(),
                    updated_at: "2026-06-14".to_string(),
                },
                RequestItem {
                    id: 102,
                    authority: "treasury".to_string(),
                    title: "Budget 2026 Advice".to_string(),
                    status: "waiting_clarification".to_string(),
                    needs_attention: true,
                    priority: "High".to_string(),
                    updated_at: "2026-06-13".to_string(),
                },
                RequestItem {
                    id: 103,
                    authority: "police".to_string(),
                    title: "Facial Recognition Trial".to_string(),
                    status: "waiting_response".to_string(),
                    needs_attention: true,
                    priority: "Medium".to_string(),
                    updated_at: "2026-06-12".to_string(),
                },
            ],
            tracked_requests: vec![
                RequestItem {
                    id: 101,
                    authority: "ministry-of-health".to_string(),
                    title: "COVID-19 Advisory Group Minutes".to_string(),
                    status: "overdue".to_string(),
                    needs_attention: true,
                    priority: "High".to_string(),
                    updated_at: "2026-06-14".to_string(),
                },
                RequestItem {
                    id: 102,
                    authority: "treasury".to_string(),
                    title: "Budget 2026 Advice".to_string(),
                    status: "waiting_clarification".to_string(),
                    needs_attention: true,
                    priority: "High".to_string(),
                    updated_at: "2026-06-13".to_string(),
                },
                RequestItem {
                    id: 103,
                    authority: "police".to_string(),
                    title: "Facial Recognition Trial".to_string(),
                    status: "waiting_response".to_string(),
                    needs_attention: true,
                    priority: "Medium".to_string(),
                    updated_at: "2026-06-12".to_string(),
                },
                RequestItem {
                    id: 104,
                    authority: "nzta".to_string(),
                    title: "Smart Motorway Cost-Benefit Analysis".to_string(),
                    status: "successful".to_string(),
                    needs_attention: false,
                    priority: "Low".to_string(),
                    updated_at: "2026-06-10".to_string(),
                },
            ],
            selected_request_idx: 0,
            logs: vec![
                "[2026-06-15 00:01:10] Loaded 4 requests from local db".to_string(),
                "[2026-06-15 00:02:15] Refreshed status of request #101".to_string(),
                "[2026-06-15 00:03:00] Tor daemon connected successfully".to_string(),
                "[2026-06-15 00:04:12] Updated dashboard summary statistics".to_string(),
            ],
            selected_log_idx: 0,
            mfa_enabled_accounts: Vec::new(),
            mfa_setup_account: None,
            mfa_session_verified: false,
            editor_request_id: None,
            editor_title: "New FYI request".to_string(),
            editor_body: "Draft request body in markdown.".to_string(),
            editor_tags: String::new(),
            editor_dirty: false,
            editor_last_saved_label: None,
            editor_drafts: Vec::new(),
            selected_editor_draft_idx: 0,
            active_editor_pane: EditorPane::Draft,
            editor_active_field: EditorField::Title,
            editor_save_requested: false,
            credential_dialog_open: false,
            credentials: Vec::new(),
            selected_credential_idx: 0,
            active_credential_account: None,
            credential_session_status: CredentialSessionStatus::NotTested,
            credential_test_message: None,
            credential_test_requested: false,
            keyring_entries: Vec::new(),
            selected_keyring_entry_idx: 0,
            encryption_key_managed: false,
            encryption_key_strength: None,
            encryption_key_rotation_status: None,
            search_open: false,
            search_query: String::new(),
            search_results: Vec::new(),
            selected_search_result_idx: 0,
            request_multi_select: false,
            selected_request_ids: BTreeSet::new(),
            export_actions: Vec::new(),
            last_export_message: None,
            help_open: false,
            should_quit: false,
        }
    }

    pub fn next_tab(&mut self) {
        let tabs = Tab::all();
        let current_idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        let next_idx = (current_idx + 1) % tabs.len();
        self.active_tab = tabs[next_idx];
    }

    pub fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let current_idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        let prev_idx = (current_idx + tabs.len() - 1) % tabs.len();
        self.active_tab = tabs[prev_idx];
    }

    pub fn select_next(&mut self) {
        match self.active_tab {
            Tab::Requests => {
                if !self.tracked_requests.is_empty() {
                    self.selected_request_idx =
                        (self.selected_request_idx + 1) % self.tracked_requests.len();
                }
            }
            Tab::Logs => {
                if !self.logs.is_empty() {
                    self.selected_log_idx = (self.selected_log_idx + 1) % self.logs.len();
                }
            }
            Tab::Keyring => {
                if !self.keyring_entries.is_empty() {
                    self.selected_keyring_entry_idx =
                        (self.selected_keyring_entry_idx + 1) % self.keyring_entries.len();
                }
            }
            Tab::Summary | Tab::Mfa | Tab::Editor => {}
        }
    }

    pub fn select_prev(&mut self) {
        match self.active_tab {
            Tab::Requests => {
                if !self.tracked_requests.is_empty() {
                    self.selected_request_idx =
                        (self.selected_request_idx + self.tracked_requests.len() - 1)
                            % self.tracked_requests.len();
                }
            }
            Tab::Logs => {
                if !self.logs.is_empty() {
                    self.selected_log_idx =
                        (self.selected_log_idx + self.logs.len() - 1) % self.logs.len();
                }
            }
            Tab::Keyring => {
                if !self.keyring_entries.is_empty() {
                    self.selected_keyring_entry_idx =
                        (self.selected_keyring_entry_idx + self.keyring_entries.len() - 1)
                            % self.keyring_entries.len();
                }
            }
            Tab::Summary | Tab::Mfa | Tab::Editor => {}
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> TuiCommand {
        if self.help_open {
            if matches!(key.code, KeyCode::Esc | KeyCode::F(1))
                || matches!(
                    (key.code, key.modifiers),
                    (KeyCode::Char('h'), KeyModifiers::CONTROL)
                )
            {
                self.help_open = false;
            }
            return TuiCommand::None;
        }

        if self.search_open {
            self.handle_search_key_event(key);
            return TuiCommand::None;
        }

        if self.credential_dialog_open {
            return self.handle_credential_dialog_key_event(key);
        }

        match (key.code, key.modifiers) {
            (KeyCode::F(1), KeyModifiers::NONE) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                self.help_open = true;
                TuiCommand::None
            }
            (KeyCode::Char('/'), KeyModifiers::NONE) => {
                self.search_open = true;
                self.search_query.clear();
                self.refresh_search_results();
                TuiCommand::None
            }
            (KeyCode::Char('m'), KeyModifiers::NONE) if self.active_tab == Tab::Requests => {
                self.toggle_request_multi_select();
                TuiCommand::None
            }
            (KeyCode::Char(' '), KeyModifiers::NONE)
                if self.active_tab == Tab::Requests && self.request_multi_select =>
            {
                self.toggle_selected_request();
                TuiCommand::None
            }
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.trigger_export("json");
                TuiCommand::None
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.trigger_export("csv");
                TuiCommand::None
            }
            (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.trigger_export("pdf");
                TuiCommand::None
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.credential_dialog_open = true;
                TuiCommand::None
            }
            (KeyCode::Char('q'), KeyModifiers::CONTROL) if self.active_tab == Tab::Editor => {
                self.should_quit = true;
                TuiCommand::CloseEditor
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) if self.active_tab == Tab::Editor => {
                self.editor_save_requested = true;
                self.editor_last_saved_label = Some("Save requested".to_string());
                TuiCommand::SaveDraft
            }
            (KeyCode::Tab, KeyModifiers::NONE) if self.active_tab == Tab::Editor => {
                self.next_tab();
                TuiCommand::None
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.next_tab();
                TuiCommand::None
            }
            (KeyCode::Right, KeyModifiers::NONE) if self.active_tab == Tab::Editor => {
                self.active_editor_pane = self.active_editor_pane.next();
                TuiCommand::None
            }
            (KeyCode::BackTab, KeyModifiers::SHIFT) => {
                self.prev_tab();
                TuiCommand::None
            }
            (KeyCode::Down, KeyModifiers::NONE) if self.active_tab == Tab::Editor => {
                self.editor_active_field = self.editor_active_field.next();
                TuiCommand::None
            }
            (KeyCode::Up, KeyModifiers::NONE) if self.active_tab == Tab::Editor => {
                self.editor_active_field = self.editor_active_field.prev();
                TuiCommand::None
            }
            (KeyCode::Backspace, KeyModifiers::NONE) if self.active_tab == Tab::Editor => {
                self.backspace_editor_field();
                TuiCommand::None
            }
            (KeyCode::Enter, KeyModifiers::NONE)
                if self.active_tab == Tab::Editor
                    && self.editor_active_field == EditorField::Body =>
            {
                self.push_editor_char('\n');
                TuiCommand::None
            }
            (KeyCode::Char(ch), KeyModifiers::NONE) if self.active_tab == Tab::Editor => {
                self.push_editor_char(ch);
                TuiCommand::None
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                self.select_next();
                TuiCommand::None
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                self.select_prev();
                TuiCommand::None
            }
            (KeyCode::Esc, KeyModifiers::NONE) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
                self.should_quit = true;
                TuiCommand::Quit
            }
            _ => TuiCommand::None,
        }
    }

    fn handle_credential_dialog_key_event(&mut self, key: KeyEvent) -> TuiCommand {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, KeyModifiers::NONE) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
                self.credential_dialog_open = false;
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                if !self.credentials.is_empty() {
                    self.selected_credential_idx =
                        (self.selected_credential_idx + 1) % self.credentials.len();
                }
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                if !self.credentials.is_empty() {
                    self.selected_credential_idx =
                        (self.selected_credential_idx + self.credentials.len() - 1)
                            % self.credentials.len();
                }
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.activate_selected_credential();
            }
            (KeyCode::Char('t'), KeyModifiers::NONE) => {
                self.credential_test_requested = true;
                return TuiCommand::TestCredential;
            }
            _ => {}
        }
        TuiCommand::None
    }

    fn handle_search_key_event(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.search_open = false;
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                self.search_query.pop();
                self.refresh_search_results();
            }
            (KeyCode::Down, KeyModifiers::NONE) => self.select_next_search_result(),
            (KeyCode::Up, KeyModifiers::NONE) => self.select_prev_search_result(),
            (KeyCode::Char(ch), KeyModifiers::NONE) => {
                self.search_query.push(ch);
                self.refresh_search_results();
            }
            _ => {}
        }
    }

    fn push_editor_char(&mut self, ch: char) {
        match self.editor_active_field {
            EditorField::Title => self.editor_title.push(ch),
            EditorField::Body => self.editor_body.push(ch),
            EditorField::Tags => self.editor_tags.push(ch),
        }
        self.editor_dirty = true;
    }

    fn backspace_editor_field(&mut self) {
        let removed = match self.editor_active_field {
            EditorField::Title => self.editor_title.pop(),
            EditorField::Body => self.editor_body.pop(),
            EditorField::Tags => self.editor_tags.pop(),
        };
        if removed.is_some() {
            self.editor_dirty = true;
        }
    }

    pub async fn load_editor_from_db(
        &mut self,
        db: &fyi_core::db::DbPool,
        request_id: i64,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(request) = db.get_request(request_id).await? else {
            return Ok(false);
        };

        self.editor_request_id = Some(request.id);
        self.editor_title = request.title;
        self.editor_body = request.body;
        self.editor_tags = request.tags.unwrap_or_default().join(",");
        self.editor_dirty = false;
        self.editor_last_saved_label = None;
        Ok(true)
    }

    pub async fn save_editor_to_db(
        &self,
        db: &fyi_core::db::DbPool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(request_id) = self.editor_request_id else {
            return Ok(false);
        };
        let Some(mut request) = db.get_request(request_id).await? else {
            return Ok(false);
        };

        request.title = self.editor_title.clone();
        request.body = self.editor_body.clone();
        request.tags = parse_editor_tags(&self.editor_tags);
        Ok(db.update_request(&request).await?)
    }

    pub fn update_editor_title(&mut self, title: impl Into<String>) {
        self.editor_title = title.into();
        self.editor_dirty = true;
    }

    pub fn update_editor_body(&mut self, body: impl Into<String>) {
        self.editor_body = body.into();
        self.editor_dirty = true;
    }

    pub fn update_editor_tags(&mut self, tags: impl Into<String>) {
        self.editor_tags = tags.into();
        self.editor_dirty = true;
    }

    pub async fn autosave_editor_to_db(
        &mut self,
        db: &fyi_core::db::DbPool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if !self.editor_dirty {
            return Ok(false);
        }

        let saved = self.save_editor_to_db(db).await?;
        if saved {
            self.editor_dirty = false;
            self.editor_last_saved_label = Some("Autosaved".to_string());
        }
        Ok(saved)
    }

    pub async fn refresh_editor_drafts(
        &mut self,
        db: &fyi_core::db::DbPool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.editor_drafts = db
            .list_requests(500)
            .await?
            .into_iter()
            .filter(|request| request.status.as_deref() == Some("draft"))
            .map(|request| DraftItem {
                id: request.id,
                title: request.title,
                status: request.status.unwrap_or_else(|| "draft".to_string()),
                updated_at: request.updated_at.unwrap_or_else(|| "unknown".to_string()),
            })
            .collect();

        if self.selected_editor_draft_idx >= self.editor_drafts.len() {
            self.selected_editor_draft_idx = self.editor_drafts.len().saturating_sub(1);
        }
        Ok(())
    }

    pub async fn refresh_sync_status_from_db(
        &mut self,
        db: &fyi_core::db::DbPool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let status = db.get_global_sync_status().await?;
        self.sync_clean_count = status.clean;
        self.sync_dirty_count = status.dirty;
        self.sync_pending_count = status.pending;
        self.sync_conflict_count = status.conflict;
        Ok(())
    }

    pub async fn open_selected_editor_draft(
        &mut self,
        db: &fyi_core::db::DbPool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(draft) = self.editor_drafts.get(self.selected_editor_draft_idx) else {
            return Ok(false);
        };
        self.load_editor_from_db(db, draft.id).await
    }

    pub async fn discard_selected_editor_draft(
        &mut self,
        db: &fyi_core::db::DbPool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(draft) = self.editor_drafts.get(self.selected_editor_draft_idx) else {
            return Ok(false);
        };
        let deleted = db.delete_request(draft.id).await?;
        if deleted {
            self.refresh_editor_drafts(db).await?;
        }
        Ok(deleted)
    }

    pub fn refresh_credentials_from_keyring(
        &mut self,
        store: &fyi_core::security::KeyringStore,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.credentials = store
            .list_credentials()?
            .into_iter()
            .map(CredentialAccount::new)
            .collect();

        if self.selected_credential_idx >= self.credentials.len() {
            self.selected_credential_idx = self.credentials.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn activate_selected_credential(&mut self) -> bool {
        let Some(account) = self.credentials.get(self.selected_credential_idx) else {
            return false;
        };
        self.active_credential_account = Some(account.username.clone());
        self.credential_session_status = CredentialSessionStatus::NotTested;
        self.credential_test_message = None;
        true
    }

    pub fn test_active_credential(
        &mut self,
        store: &fyi_core::security::KeyringStore,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(account) = self.active_credential_account.as_deref() else {
            self.credential_session_status = CredentialSessionStatus::Missing;
            self.credential_test_message = Some("No active credential".to_string());
            return Ok(false);
        };

        match store.get_credential(account) {
            Ok(_) => {
                self.credential_session_status = CredentialSessionStatus::Verified;
                self.credential_test_message = Some("Credential verified".to_string());
                Ok(true)
            }
            Err(fyi_core::security::SecurityError::MfaRequired(_)) => {
                self.credential_session_status = CredentialSessionStatus::MfaRequired;
                self.credential_test_message = Some("MFA verification required".to_string());
                Ok(false)
            }
            Err(fyi_core::security::SecurityError::KeyringError(error))
                if error.contains("No keyring entry found") =>
            {
                self.credential_session_status = CredentialSessionStatus::Missing;
                self.credential_test_message = Some("Credential not found".to_string());
                Ok(false)
            }
            Err(error) => {
                self.credential_session_status = CredentialSessionStatus::Error;
                self.credential_test_message = Some(error.to_string());
                Ok(false)
            }
        }
    }

    pub fn refresh_keyring_entries(
        &mut self,
        store: &fyi_core::security::KeyringStore,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut entries = store
            .list_credentials()?
            .into_iter()
            .map(|username| KeyringEntryItem::new("Credential", username, "fyi-cli"))
            .collect::<Vec<_>>();

        entries.extend(
            store
                .list_totp_secrets()?
                .into_iter()
                .map(|username| KeyringEntryItem::new("MFA TOTP", username, "fyi-cli")),
        );

        self.keyring_entries = entries;
        if self.selected_keyring_entry_idx >= self.keyring_entries.len() {
            self.selected_keyring_entry_idx = self.keyring_entries.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn upsert_keyring_credential(
        &mut self,
        store: &fyi_core::security::KeyringStore,
        username: &str,
        secret: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        store.set_credential(
            username,
            &fyi_core::security::ZeroizedString::new(secret.to_string()),
        )?;
        self.refresh_keyring_entries(store)?;
        Ok(true)
    }

    pub fn delete_selected_keyring_entry(
        &mut self,
        store: &fyi_core::security::KeyringStore,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(entry) = self.keyring_entries.get(self.selected_keyring_entry_idx) else {
            return Ok(false);
        };

        match entry.kind.as_str() {
            "Credential" => store.delete_credential(&entry.username)?,
            "MFA TOTP" => store.delete_totp_secret(&entry.username)?,
            _ => return Ok(false),
        }

        self.refresh_keyring_entries(store)?;
        Ok(true)
    }

    pub fn generate_managed_encryption_key(&mut self) {
        let _key = fyi_core::security::EncryptionKey::generate();
        self.encryption_key_managed = true;
        self.encryption_key_strength = Some("Strong".to_string());
        self.encryption_key_rotation_status = Some("Current".to_string());
    }

    pub fn refresh_search_results(&mut self) {
        let query = self.search_query.to_ascii_lowercase();
        self.search_results.clear();

        if query.is_empty() {
            self.search_results.push(SearchResultItem::new(
                "Hint",
                "Type to search",
                "Requests, authorities, logs",
            ));
        } else {
            let mut results = Vec::new();
            for request in &self.tracked_requests {
                if fuzzy_matches(&request.title, &query)
                    || fuzzy_matches(&request.authority, &query)
                    || fuzzy_matches(&request.status, &query)
                {
                    results.push(SearchResultItem::new(
                        "Request",
                        request.title.clone(),
                        format!("{} | {}", request.authority, request.status),
                    ));
                }
                if fuzzy_matches(&request.authority, &query) {
                    results.push(SearchResultItem::new(
                        "Authority",
                        request.authority.clone(),
                        format!("Request #{}", request.id),
                    ));
                }
            }
            for log in &self.logs {
                if fuzzy_matches(log, &query) {
                    results.push(SearchResultItem::new(
                        "Activity Logs",
                        log.clone(),
                        "Log entry",
                    ));
                }
            }
            for entry in &self.keyring_entries {
                if fuzzy_matches(&entry.username, &query) || fuzzy_matches(&entry.kind, &query) {
                    results.push(SearchResultItem::new(
                        "Keyring",
                        entry.username.clone(),
                        entry.kind.clone(),
                    ));
                }
            }
            self.search_results = results;
        }

        if self.selected_search_result_idx >= self.search_results.len() {
            self.selected_search_result_idx = self.search_results.len().saturating_sub(1);
        }
    }

    pub fn select_next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.selected_search_result_idx =
                (self.selected_search_result_idx + 1) % self.search_results.len();
        }
    }

    pub fn select_prev_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.selected_search_result_idx =
                (self.selected_search_result_idx + self.search_results.len() - 1)
                    % self.search_results.len();
        }
    }

    pub fn toggle_request_multi_select(&mut self) {
        self.request_multi_select = !self.request_multi_select;
        if !self.request_multi_select {
            self.selected_request_ids.clear();
        }
    }

    pub fn toggle_selected_request(&mut self) {
        let Some(request) = self.tracked_requests.get(self.selected_request_idx) else {
            return;
        };
        if !self.selected_request_ids.insert(request.id) {
            self.selected_request_ids.remove(&request.id);
        }
    }

    pub fn bulk_update_selected_status(&mut self, status: &str) -> usize {
        let mut updated = 0;
        for request in &mut self.tracked_requests {
            if self.selected_request_ids.contains(&request.id) {
                request.status = status.to_string();
                updated += 1;
            }
        }
        updated
    }

    pub fn trigger_export(&mut self, format: &str) {
        self.export_actions.push(format.to_string());
        self.last_export_message = Some(format!("Queued {format} export"));
    }
}

fn fuzzy_matches(value: &str, query: &str) -> bool {
    let value = value.to_ascii_lowercase();
    if value.contains(query) {
        return true;
    }

    let mut chars = value.chars();
    query
        .chars()
        .all(|needle| chars.by_ref().any(|candidate| candidate == needle))
}

fn parse_editor_tags(tags: &str) -> Option<Vec<String>> {
    let parsed = tags
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    (!parsed.is_empty()).then_some(parsed)
}

fn markdown_preview(markdown: &str) -> String {
    markdown
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if let Some(heading) = trimmed.strip_prefix("# ") {
                heading.to_uppercase()
            } else if let Some(item) = trimmed.strip_prefix("- ") {
                format!("* {}", strip_inline_markdown(item))
            } else {
                strip_inline_markdown(trimmed)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_inline_markdown(text: &str) -> String {
    text.replace("**", "")
        .replace("__", "")
        .replace(['`', '*', '_'], "")
}

fn editor_pane_title(label: &str, pane: EditorPane, active_pane: EditorPane) -> String {
    if pane == active_pane {
        format!(" {label} [active] ")
    } else {
        format!(" {label} ")
    }
}

pub fn draw_ui(f: &mut Frame<'_>, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top Bar (Tabs)
            Constraint::Min(10),   // Main Viewport
            Constraint::Length(3), // Bottom Info bar
        ])
        .split(f.size());

    // Title and Tabs Block
    let tabs_titles: Vec<&str> = Tab::all().iter().map(|t| t.title()).collect();
    let current_tab_idx = Tab::all()
        .iter()
        .position(|t| *t == state.active_tab)
        .unwrap_or(0);

    let active_account = state.active_credential_account.as_deref().unwrap_or("None");
    let tabs = Tabs::new(tabs_titles)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " FYI Request System | Account: {active_account} | Session: {} ",
            state.credential_session_status.label()
        )))
        .select(current_tab_idx)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[0]);

    // Draw active tab content
    match state.active_tab {
        Tab::Summary => draw_summary_tab(f, state, chunks[1]),
        Tab::Requests => draw_requests_tab(f, state, chunks[1]),
        Tab::Logs => draw_logs_tab(f, state, chunks[1]),
        Tab::Mfa => draw_mfa_tab(f, state, chunks[1]),
        Tab::Editor => draw_editor_tab(f, state, chunks[1]),
        Tab::Keyring => draw_keyring_tab(f, state, chunks[1]),
    }

    // Help Footer Bar
    let help_text = match state.active_tab {
        Tab::Summary => " Tab: Next Tab | Q/Esc: Quit ",
        Tab::Requests => " Tab: Next Tab | Up/Down: Navigate Requests | Q/Esc: Quit ",
        Tab::Logs => " Tab: Next Tab | Up/Down: Scroll Logs | Q/Esc: Quit ",
        Tab::Mfa => {
            " Tab: Next Tab | Use fyi mfa setup/verify/remove for MFA actions | Q/Esc: Quit "
        }
        Tab::Editor => {
            " Tab: Next Tab | Right: Switch Pane | Ctrl+S: Save Draft | Ctrl+Q: Close Editor | Esc: Quit "
        }
        Tab::Keyring => " Tab: Next Tab | Up/Down: Navigate Entries | Q/Esc: Quit ",
    };
    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help_paragraph, chunks[2]);

    if state.credential_dialog_open {
        draw_credential_dialog(f, state, centered_rect(70, 55, f.size()));
    }

    if state.search_open {
        draw_search_overlay(f, state, centered_rect(76, 60, f.size()));
    }

    if state.help_open {
        draw_help_overlay(f, centered_rect(72, 62, f.size()));
    }
}

fn draw_summary_tab(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Metric cards row
            Constraint::Length(3), // Sync status row
            Constraint::Min(5),    // Action table
        ])
        .split(area);

    // Render Metrics Cards
    let card_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(chunks[0]);

    let metrics = vec![
        ("Total Tracked", state.total_tracked, Color::White),
        ("Needs Attention", state.needs_attention_count, Color::Red),
        ("Action Now", state.action_now_count, Color::LightRed),
        ("Authorities", state.authorities_count, Color::Blue),
        ("Recent (7d)", state.recent_updates, Color::Green),
    ];

    for (i, (label, val, color)) in metrics.into_iter().enumerate() {
        let card = Paragraph::new(format!("\n   {}", val))
            .block(Block::default().borders(Borders::ALL).title(label))
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD));
        f.render_widget(card, card_cols[i]);
    }

    let sync_line = format!(
        "Clean: {} | Dirty: {} | Pending: {} | Conflicts: {}",
        state.sync_clean_count,
        state.sync_dirty_count,
        state.sync_pending_count,
        state.sync_conflict_count
    );
    let sync_panel = Paragraph::new(sync_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Sync Status "),
        )
        .style(Style::default().fg(if state.sync_conflict_count > 0 {
            Color::Red
        } else if state.sync_dirty_count > 0 || state.sync_pending_count > 0 {
            Color::Yellow
        } else {
            Color::Green
        }));
    f.render_widget(sync_panel, chunks[1]);

    // Render Needs Action Table
    let rows = state.action_now_requests.iter().map(|item| {
        let priority_style = if item.priority == "High" {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };
        Row::new(vec![
            Cell::from(item.id.to_string()),
            Cell::from(item.authority.as_str()),
            Cell::from(item.title.as_str()),
            Cell::from(item.status.as_str()),
            Cell::from(item.priority.as_str()).style(priority_style),
        ])
    });

    let table = Table::new(rows)
        .header(
            Row::new(vec![
                "ID",
                "Authority",
                "Request Title",
                "Status",
                "Priority",
            ])
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Needs Action Now "),
        )
        .widths(&[
            Constraint::Length(6),
            Constraint::Percentage(25),
            Constraint::Percentage(45),
            Constraint::Percentage(15),
            Constraint::Length(10),
        ]);

    f.render_widget(table, chunks[2]);
}

fn draw_requests_tab(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // List of requests
            Constraint::Percentage(40), // Request detail preview pane
        ])
        .split(area);

    // Render Request Table List
    let rows = state
        .tracked_requests
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let mut style = Style::default();
            if idx == state.selected_request_idx {
                style = style.bg(Color::DarkGray).fg(Color::Yellow);
            } else if item.needs_attention {
                style = style.fg(Color::LightRed);
            }
            let selected = if state.selected_request_ids.contains(&item.id) {
                "*"
            } else {
                " "
            };
            Row::new(vec![
                Cell::from(selected),
                Cell::from(item.id.to_string()),
                Cell::from(item.authority.as_str()),
                Cell::from(item.title.as_str()),
                Cell::from(item.status.as_str()),
            ])
            .style(style)
        });

    let table = Table::new(rows)
        .header(
            Row::new(vec!["Sel", "ID", "Authority", "Title", "Status"])
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Tracked Requests "),
        )
        .widths(&[
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Percentage(30),
            Constraint::Percentage(45),
            Constraint::Percentage(20),
        ]);

    f.render_widget(table, chunks[0]);

    // Detail preview panel
    let preview_content = if let Some(selected_req) =
        state.tracked_requests.get(state.selected_request_idx)
    {
        format!(
            "ID: {}\nAuthority: {}\nTitle: {}\nStatus: {}\nPriority: {}\nUpdated At: {}\nNeeds Attention: {}\n\n(Use local web UI or MCP client to reply or update states)",
            selected_req.id,
            selected_req.authority,
            selected_req.title,
            selected_req.status,
            selected_req.priority,
            selected_req.updated_at,
            if selected_req.needs_attention { "YES" } else { "NO" }
        )
    } else {
        "No requests selected".to_string()
    };

    let preview = Paragraph::new(preview_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Detail Preview "),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(preview, chunks[1]);
}

fn draw_logs_tab(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    // Simple scrollable list of logs
    let mut log_lines = Vec::new();
    for (idx, log) in state.logs.iter().enumerate() {
        let style = if idx == state.selected_log_idx {
            Style::default().bg(Color::DarkGray).fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        log_lines.push(Line::from(Span::styled(log.clone(), style)));
    }

    let logs_p = Paragraph::new(log_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Activity & Daemon Logs "),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(logs_p, area);
}

fn draw_mfa_tab(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let accounts = if state.mfa_enabled_accounts.is_empty() {
        "No MFA-enabled accounts".to_string()
    } else {
        state.mfa_enabled_accounts.join("\n")
    };
    let setup_account = state
        .mfa_setup_account
        .as_deref()
        .unwrap_or("No setup wizard active");
    let session = if state.mfa_session_verified {
        "Verified session active"
    } else {
        "Credential access guarded"
    };

    let content = format!(
        "Setup Wizard\nAccount: {setup_account}\n\nMFA-enabled accounts\n{accounts}\n\nCredential access\n{session}"
    );
    let panel = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" MFA Security "),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(panel, area);
}

fn draw_keyring_tab(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(6)])
        .split(area);

    let rows = state
        .keyring_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let style = if idx == state.selected_keyring_entry_idx {
                Style::default().bg(Color::DarkGray).fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            Row::new(vec![
                Cell::from(entry.kind.as_str()),
                Cell::from(entry.username.as_str()),
                Cell::from(entry.service.as_str()),
                Cell::from(entry.created_at.as_str()),
                Cell::from(entry.last_used_at.as_str()),
                Cell::from(entry.key_strength.as_str()),
                Cell::from(entry.key_age.as_str()),
                Cell::from(entry.rotation_status.as_str()),
            ])
            .style(style)
        });

    let table = Table::new(rows)
        .header(
            Row::new(vec![
                "Kind",
                "Username",
                "Service",
                "Created",
                "Last Used",
                "Security",
                "Age",
                "Rotation",
            ])
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Keyring Browser "),
        )
        .widths(&[
            Constraint::Length(14),
            Constraint::Percentage(35),
            Constraint::Length(16),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(10),
        ]);

    f.render_widget(table, chunks[0]);

    let key_status = if state.encryption_key_managed {
        format!(
            "Encryption Key: Managed\nSecurity: {}\nRotation: {}",
            state
                .encryption_key_strength
                .as_deref()
                .unwrap_or("Unknown"),
            state
                .encryption_key_rotation_status
                .as_deref()
                .unwrap_or("Unknown")
        )
    } else {
        "Encryption Key: Not managed\nSecurity: Unknown\nRotation: Unknown".to_string()
    };
    let key_panel = Paragraph::new(format!(
        "{key_status}\nActions: A add/update credential | D delete selected | R rotate key"
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Encryption Key Management "),
    )
    .style(Style::default().fg(Color::White));
    f.render_widget(key_panel, chunks[1]);
}

fn draw_credential_dialog(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    f.render_widget(Clear, area);
    let active = state.active_credential_account.as_deref().unwrap_or("None");
    let accounts = if state.credentials.is_empty() {
        "No stored credentials".to_string()
    } else {
        state
            .credentials
            .iter()
            .enumerate()
            .map(|(idx, account)| {
                let marker = if idx == state.selected_credential_idx {
                    ">"
                } else {
                    " "
                };
                let active_marker = if Some(account.username.as_str())
                    == state.active_credential_account.as_deref()
                {
                    " Active"
                } else {
                    ""
                };
                format!("{marker} {}{active_marker}", account.username)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let message = state
        .credential_test_message
        .as_deref()
        .unwrap_or("Not tested");
    let content = format!(
        "Active: {active}\nSession: {}\nLast test: {message}\n\nAccounts\n{accounts}\n\nEnter: Switch | T: Test Credential | Up/Down: Select | Esc: Close",
        state.credential_session_status.label()
    );
    let dialog = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Credential Manager "),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(dialog, area);
}

fn draw_search_overlay(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    f.render_widget(Clear, area);
    let results = if state.search_results.is_empty() {
        "No results".to_string()
    } else {
        state
            .search_results
            .iter()
            .enumerate()
            .map(|(idx, result)| {
                let marker = if idx == state.selected_search_result_idx {
                    ">"
                } else {
                    " "
                };
                format!(
                    "{marker} [{}] {}\n    {}",
                    result.kind, result.title, result.detail
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let content = format!(
        "Query: {}\n\n{}\n\nUp/Down: Navigate | Esc: Close",
        state.search_query, results
    );
    let overlay = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Fuzzy Search "),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(overlay, area);
}

fn draw_help_overlay(f: &mut Frame<'_>, area: Rect) {
    f.render_widget(Clear, area);
    let content = [
        "Navigation",
        "Tab / Shift+Tab: Switch tabs",
        "Up/Down: Move within lists",
        "/: Fuzzy search",
        "",
        "Request Bulk Operations",
        "M: Toggle multi-select",
        "Space: Select request",
        "Bulk status: apply to selected requests",
        "",
        "Export",
        "Ctrl+J: Export JSON",
        "Ctrl+E: Export CSV",
        "Ctrl+P: Export PDF",
        "",
        "Panels",
        "Ctrl+C: Credential manager",
        "F1/Ctrl+H: Help",
    ]
    .join("\n");
    let overlay = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(" Help "))
        .style(Style::default().fg(Color::White));
    f.render_widget(overlay, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn draw_editor_tab(f: &mut Frame<'_>, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(7),
        ])
        .split(area);

    let title = Paragraph::new(state.editor_title.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Title "))
        .style(Style::default().fg(Color::White));
    f.render_widget(title, chunks[0]);

    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    let body = Paragraph::new(state.editor_body.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(editor_pane_title(
                    "Markdown Draft Body",
                    EditorPane::Draft,
                    state.active_editor_pane,
                )),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(body, editor_chunks[0]);

    let preview = Paragraph::new(markdown_preview(&state.editor_body))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(editor_pane_title(
                    "Preview",
                    EditorPane::Preview,
                    state.active_editor_pane,
                )),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(preview, editor_chunks[1]);

    let save_state = if state.editor_dirty {
        "Unsaved changes".to_string()
    } else {
        state
            .editor_last_saved_label
            .clone()
            .unwrap_or_else(|| "Saved".to_string())
    };
    let drafts = if state.editor_drafts.is_empty() {
        "No saved drafts".to_string()
    } else {
        state
            .editor_drafts
            .iter()
            .enumerate()
            .map(|(idx, draft)| {
                let marker = if idx == state.selected_editor_draft_idx {
                    ">"
                } else {
                    " "
                };
                format!(
                    "{marker} #{} {} ({})",
                    draft.id, draft.title, draft.updated_at
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let tags = Paragraph::new(format!(
        "Tags: {}\nStatus: {}\nField: {}\n\nDrafts\n{}",
        state.editor_tags,
        save_state,
        state.editor_active_field.label(),
        drafts
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(editor_pane_title(
                "Tags & Drafts",
                EditorPane::Drafts,
                state.active_editor_pane,
            )),
    )
    .style(Style::default().fg(Color::White));
    f.render_widget(tags, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_app_state_transitions() {
        let mut state = AppState::new();
        assert_eq!(state.active_tab, Tab::Summary);

        state.next_tab();
        assert_eq!(state.active_tab, Tab::Requests);

        state.next_tab();
        assert_eq!(state.active_tab, Tab::Logs);

        state.next_tab();
        assert_eq!(state.active_tab, Tab::Mfa);

        state.next_tab();
        assert_eq!(state.active_tab, Tab::Editor);

        state.next_tab();
        assert_eq!(state.active_tab, Tab::Keyring);

        state.next_tab();
        assert_eq!(state.active_tab, Tab::Summary);

        state.prev_tab();
        assert_eq!(state.active_tab, Tab::Keyring);
    }

    #[test]
    fn test_list_navigation() {
        let mut state = AppState::new();
        state.active_tab = Tab::Requests;
        state.selected_request_idx = 0;

        state.select_next();
        assert_eq!(state.selected_request_idx, 1);

        state.select_prev();
        assert_eq!(state.selected_request_idx, 0);
    }

    #[test]
    fn test_mock_rendering_summary() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = AppState::new();

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered_text = buffer_to_string(buffer);
        assert!(rendered_text.contains("Summary Dashboard"));
        assert!(rendered_text.contains("Tracked Requests"));
        assert!(rendered_text.contains("Total Tracked"));
        assert!(rendered_text.contains("Action Now"));
        assert!(rendered_text.contains("Sync Status"));
        assert!(rendered_text.contains("COVID-19 Advisory Group Minutes"));
    }

    #[tokio::test]
    async fn test_tui_refreshes_sync_status_from_db() {
        let db = fyi_core::db::DbPool::new_in_memory()
            .await
            .expect("Failed to create in-memory db");
        db.run_migrations().await.expect("Failed to run migrations");

        db.insert_request(&fyi_core::api::AlaveteliRequest {
            id: 901,
            title: "Dirty request".to_string(),
            body: "Body".to_string(),
            user_name: None,
            status: Some("draft".to_string()),
            created_at: None,
            updated_at: None,
            url: None,
            tags: None,
        })
        .await
        .expect("Failed to insert request");

        let mut state = AppState::new();
        state
            .refresh_sync_status_from_db(&db)
            .await
            .expect("Failed to refresh sync status");

        assert_eq!(state.sync_dirty_count, 1);
        assert_eq!(state.sync_clean_count, 0);
    }

    #[test]
    fn test_mock_rendering_requests_tab() {
        let backend = TestBackend::new(120, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_tab = Tab::Requests;
        state.selected_request_idx = 3;

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered_text = buffer_to_string(buffer);
        assert!(rendered_text.contains("Tracked Requests"));
        assert!(rendered_text.contains("Detail Preview"));
        assert!(rendered_text.contains("Smart Motorway"));
    }

    #[test]
    fn test_mock_rendering_mfa_tab() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_tab = Tab::Mfa;
        state.mfa_enabled_accounts = vec!["alice@example.org".to_string()];
        state.mfa_setup_account = Some("alice@example.org".to_string());
        state.mfa_session_verified = false;

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("MFA Security"));
        assert!(rendered_text.contains("Setup Wizard"));
        assert!(rendered_text.contains("alice@example.org"));
        assert!(rendered_text.contains("Credential access guarded"));
    }

    #[test]
    fn test_mock_rendering_editor_tab() {
        let backend = TestBackend::new(120, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_tab = Tab::Editor;
        state.editor_title = "Request title draft".to_string();
        state.editor_body = "Please provide the latest briefing notes.".to_string();
        state.editor_tags = "health,oia".to_string();

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("Request Editor"));
        assert!(rendered_text.contains("Title"));
        assert!(rendered_text.contains("Request title draft"));
        assert!(rendered_text.contains("Body"));
        assert!(rendered_text.contains("latest briefing notes"));
        assert!(rendered_text.contains("Tags"));
        assert!(rendered_text.contains("health,oia"));
    }

    #[test]
    fn test_editor_text_input_operations() {
        let mut input = TextInput::new("Initial");

        input.insert_char('!');
        assert_eq!(input.value(), "Initial!");

        input.backspace();
        input.insert_str(" draft");
        assert_eq!(input.value(), "Initial draft");

        input.replace("Final");
        assert_eq!(input.value(), "Final");
    }

    #[tokio::test]
    async fn test_editor_loads_and_saves_request_via_db() {
        let db = fyi_core::db::DbPool::new_in_memory()
            .await
            .expect("Failed to create in-memory db");
        db.run_migrations().await.expect("Failed to run migrations");
        let request = fyi_core::api::AlaveteliRequest {
            id: 501,
            title: "Original title".to_string(),
            body: "Original body".to_string(),
            user_name: Some("Alice".to_string()),
            status: Some("draft".to_string()),
            created_at: Some("2026-06-30T00:00:00Z".to_string()),
            updated_at: Some("2026-06-30T00:00:00Z".to_string()),
            url: Some("https://fyi.org.nz/request/501".to_string()),
            tags: Some(vec!["old".to_string()]),
        };
        db.insert_request(&request)
            .await
            .expect("Failed to insert request");

        let mut state = AppState::new();
        state
            .load_editor_from_db(&db, 501)
            .await
            .expect("Failed to load editor");
        assert_eq!(state.editor_title, "Original title");
        assert_eq!(state.editor_body, "Original body");
        assert_eq!(state.editor_tags, "old");

        state.editor_title = "Updated title".to_string();
        state.editor_body = "Updated body".to_string();
        state.editor_tags = "health,oia".to_string();
        assert!(state
            .save_editor_to_db(&db)
            .await
            .expect("Failed to save editor"));

        let saved = db
            .get_request(501)
            .await
            .expect("Failed to fetch request")
            .expect("Request not found");
        assert_eq!(saved.title, "Updated title");
        assert_eq!(saved.body, "Updated body");
        assert_eq!(
            saved.tags,
            Some(vec!["health".to_string(), "oia".to_string()])
        );
    }

    #[test]
    fn test_editor_markdown_preview_split_pane() {
        let backend = TestBackend::new(140, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_tab = Tab::Editor;
        state.editor_body =
            "# Request heading\nPlease provide **release notes**\n- Cabinet paper".to_string();

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("Markdown Draft"));
        assert!(rendered_text.contains("Preview"));
        assert!(rendered_text.contains("REQUEST HEADING"));
        assert!(rendered_text.contains("Please provide release notes"));
        assert!(rendered_text.contains("Cabinet paper"));
    }

    #[tokio::test]
    async fn test_editor_autosaves_and_tracks_dirty_state() {
        let db = fyi_core::db::DbPool::new_in_memory()
            .await
            .expect("Failed to create in-memory db");
        db.run_migrations().await.expect("Failed to run migrations");
        let request = fyi_core::api::AlaveteliRequest {
            id: 601,
            title: "Draft title".to_string(),
            body: "Draft body".to_string(),
            user_name: Some("Alice".to_string()),
            status: Some("draft".to_string()),
            created_at: Some("2026-06-30T00:00:00Z".to_string()),
            updated_at: Some("2026-06-30T00:00:00Z".to_string()),
            url: Some("https://fyi.org.nz/request/601".to_string()),
            tags: None,
        };
        db.insert_request(&request)
            .await
            .expect("Failed to insert request");

        let mut state = AppState::new();
        state
            .load_editor_from_db(&db, 601)
            .await
            .expect("Failed to load editor");
        assert!(!state.editor_dirty);

        state.update_editor_body("Autosaved body");
        assert!(state.editor_dirty);
        assert!(state
            .autosave_editor_to_db(&db)
            .await
            .expect("Failed to autosave editor"));
        assert!(!state.editor_dirty);
        assert_eq!(state.editor_last_saved_label.as_deref(), Some("Autosaved"));

        let saved = db
            .get_request(601)
            .await
            .expect("Failed to fetch request")
            .expect("Request not found");
        assert_eq!(saved.body, "Autosaved body");
    }

    #[tokio::test]
    async fn test_editor_loads_draft_list_from_db() {
        let db = fyi_core::db::DbPool::new_in_memory()
            .await
            .expect("Failed to create in-memory db");
        db.run_migrations().await.expect("Failed to run migrations");
        for (id, title, status) in [
            (701, "First draft", "draft"),
            (702, "Sent request", "awaiting_response"),
            (703, "Second draft", "draft"),
        ] {
            db.insert_request(&fyi_core::api::AlaveteliRequest {
                id,
                title: title.to_string(),
                body: "Body".to_string(),
                user_name: Some("Alice".to_string()),
                status: Some(status.to_string()),
                created_at: Some("2026-06-30T00:00:00Z".to_string()),
                updated_at: Some(format!("2026-06-30T00:0{id}:00Z")),
                url: Some(format!("https://fyi.org.nz/request/{id}")),
                tags: None,
            })
            .await
            .expect("Failed to insert request");
        }

        let mut state = AppState::new();
        state
            .refresh_editor_drafts(&db)
            .await
            .expect("Failed to refresh drafts");

        assert_eq!(state.editor_drafts.len(), 2);
        assert!(state
            .editor_drafts
            .iter()
            .all(|draft| draft.status == "draft"));
        assert!(state
            .editor_drafts
            .iter()
            .any(|draft| draft.title == "First draft"));
        assert!(state
            .editor_drafts
            .iter()
            .any(|draft| draft.title == "Second draft"));
    }

    #[test]
    fn test_editor_keybindings_update_fields_and_save_flag() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut state = AppState::new();
        state.active_tab = Tab::Editor;

        assert_eq!(state.editor_active_field, EditorField::Title);
        state.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));
        assert_eq!(state.editor_title, "New FYI requestA");
        assert!(state.editor_dirty);

        state.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(state.editor_active_field, EditorField::Body);
        state.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE));
        assert!(state.editor_body.ends_with('!'));

        state.handle_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert!(state.editor_save_requested);

        state.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(state.active_tab, Tab::Keyring);

        state.active_tab = Tab::Editor;
        state.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(state.should_quit);
    }

    #[test]
    fn test_editor_status_bar_shows_active_field_and_save_state() {
        let backend = TestBackend::new(140, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_tab = Tab::Editor;
        state.editor_active_field = EditorField::Tags;
        state.editor_dirty = true;

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("Field: Tags"));
        assert!(rendered_text.contains("Unsaved changes"));
        assert!(rendered_text.contains("Ctrl+S: Save"));
        assert!(rendered_text.contains("Ctrl+Q: Close"));
    }

    #[test]
    fn test_credential_manager_dialog_renders_accounts() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.credential_dialog_open = true;
        state.credentials = vec![
            CredentialAccount::new("alice@example.org"),
            CredentialAccount::new("bob@example.org"),
        ];
        state.active_credential_account = Some("alice@example.org".to_string());

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("Credential Manager"));
        assert!(rendered_text.contains("alice@example.org"));
        assert!(rendered_text.contains("bob@example.org"));
        assert!(rendered_text.contains("Active"));
        assert!(rendered_text.contains("Enter: Switch"));
    }

    #[test]
    fn test_credential_manager_loads_and_switches_accounts() {
        let store = fyi_core::security::KeyringStore::new_in_memory("fyi-cli-test");
        store
            .set_credential(
                "alice@example.org",
                &fyi_core::security::ZeroizedString::new("secret-a".to_string()),
            )
            .expect("Failed to store Alice credential");
        store
            .set_credential(
                "bob@example.org",
                &fyi_core::security::ZeroizedString::new("secret-b".to_string()),
            )
            .expect("Failed to store Bob credential");

        let mut state = AppState::new();
        state
            .refresh_credentials_from_keyring(&store)
            .expect("Failed to refresh credentials");
        assert_eq!(state.credentials.len(), 2);

        state.selected_credential_idx = state
            .credentials
            .iter()
            .position(|credential| credential.username == "bob@example.org")
            .expect("Bob credential not found");
        assert!(state.activate_selected_credential());
        assert_eq!(
            state.active_credential_account.as_deref(),
            Some("bob@example.org")
        );
    }

    #[test]
    fn test_credential_dialog_keybindings_open_switch_and_close() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut state = AppState::new();
        state.credentials = vec![
            CredentialAccount::new("alice@example.org"),
            CredentialAccount::new("bob@example.org"),
        ];

        state.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(state.credential_dialog_open);

        state.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(state.selected_credential_idx, 1);

        state.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(
            state.active_credential_account.as_deref(),
            Some("bob@example.org")
        );

        state.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!state.credential_dialog_open);
    }

    #[test]
    fn test_active_credential_status_renders_in_header() {
        let backend = TestBackend::new(140, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_credential_account = Some("alice@example.org".to_string());
        state.credential_session_status = CredentialSessionStatus::Verified;

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("alice@example.org"));
        assert!(rendered_text.contains("Session: Verified"));
    }

    #[test]
    fn test_active_credential_can_be_tested_against_keyring() {
        let store = fyi_core::security::KeyringStore::new_in_memory("fyi-cli-test");
        store
            .set_credential(
                "alice@example.org",
                &fyi_core::security::ZeroizedString::new("secret-a".to_string()),
            )
            .expect("Failed to store credential");

        let mut state = AppState::new();
        state.active_credential_account = Some("alice@example.org".to_string());

        assert!(state
            .test_active_credential(&store)
            .expect("Failed to test credential"));
        assert_eq!(
            state.credential_session_status,
            CredentialSessionStatus::Verified
        );
        assert_eq!(
            state.credential_test_message.as_deref(),
            Some("Credential verified")
        );
    }

    #[test]
    fn test_credential_dialog_test_key_sets_request_flag() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut state = AppState::new();
        state.credential_dialog_open = true;
        state.credentials = vec![CredentialAccount::new("alice@example.org")];
        state.activate_selected_credential();

        assert_eq!(
            state.handle_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
            TuiCommand::TestCredential
        );
        assert!(state.credential_test_requested);
    }

    #[test]
    fn test_keyring_browser_tab_renders_entries() {
        let backend = TestBackend::new(140, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_tab = Tab::Keyring;
        state.keyring_entries = vec![
            KeyringEntryItem::new("Credential", "alice@example.org", "fyi-cli"),
            KeyringEntryItem::new("MFA TOTP", "alice@example.org", "fyi-cli"),
        ];

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("Keyring Browser"));
        assert!(rendered_text.contains("Credential"));
        assert!(rendered_text.contains("MFA TOTP"));
        assert!(rendered_text.contains("alice@example.org"));
        assert!(rendered_text.contains("Created"));
        assert!(rendered_text.contains("Last Used"));
    }

    #[test]
    fn test_keyring_browser_loads_credentials_and_totp_entries() {
        let store = fyi_core::security::KeyringStore::new_in_memory("fyi-cli-test");
        store
            .set_credential(
                "alice@example.org",
                &fyi_core::security::ZeroizedString::new("secret-a".to_string()),
            )
            .expect("Failed to store credential");
        store
            .store_totp_secret(
                "alice@example.org",
                &fyi_core::security::generate_totp_secret().expect("Failed to generate TOTP"),
            )
            .expect("Failed to store TOTP");

        let mut state = AppState::new();
        state
            .refresh_keyring_entries(&store)
            .expect("Failed to refresh keyring entries");

        assert_eq!(state.keyring_entries.len(), 2);
        assert!(state
            .keyring_entries
            .iter()
            .any(|entry| entry.kind == "Credential"));
        assert!(state
            .keyring_entries
            .iter()
            .any(|entry| entry.kind == "MFA TOTP"));
    }

    #[test]
    fn test_keyring_actions_add_update_and_delete_credential() {
        let store = fyi_core::security::KeyringStore::new_in_memory("fyi-cli-test");
        let mut state = AppState::new();

        assert!(state
            .upsert_keyring_credential(&store, "alice@example.org", "secret-a")
            .expect("Failed to add credential"));
        assert_eq!(state.keyring_entries.len(), 1);
        assert_eq!(state.keyring_entries[0].username, "alice@example.org");

        assert!(state
            .upsert_keyring_credential(&store, "alice@example.org", "secret-b")
            .expect("Failed to update credential"));
        let saved = store
            .get_credential("alice@example.org")
            .expect("Failed to get credential");
        assert_eq!(saved.as_str(), "secret-b");

        assert!(state
            .delete_selected_keyring_entry(&store)
            .expect("Failed to delete selected credential"));
        assert!(state.keyring_entries.is_empty());
        assert!(store.get_credential("alice@example.org").is_err());
    }

    #[test]
    fn test_keyring_encryption_key_management_state() {
        let mut state = AppState::new();

        state.generate_managed_encryption_key();

        assert!(state.encryption_key_managed);
        assert_eq!(state.encryption_key_strength.as_deref(), Some("Strong"));
        assert_eq!(
            state.encryption_key_rotation_status.as_deref(),
            Some("Current")
        );
    }

    #[test]
    fn test_keyring_browser_renders_security_indicators() {
        let backend = TestBackend::new(150, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_tab = Tab::Keyring;
        state.keyring_entries =
            vec![
                KeyringEntryItem::new("Credential", "alice@example.org", "fyi-cli")
                    .with_security("Strong", "1d", "Current"),
            ];
        state.generate_managed_encryption_key();

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("Security"));
        assert!(rendered_text.contains("Strong"));
        assert!(rendered_text.contains("Current"));
        assert!(rendered_text.contains("Encryption Key"));
    }

    #[test]
    fn test_fuzzy_search_opens_with_slash_and_accepts_query() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut state = AppState::new();

        state.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(state.search_open);

        state.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        state.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(state.search_query, "he");
        assert!(!state.search_results.is_empty());
    }

    #[test]
    fn test_fuzzy_search_finds_requests_authorities_and_logs() {
        let mut state = AppState::new();
        state.search_query = "health".to_string();
        state.refresh_search_results();

        assert!(state
            .search_results
            .iter()
            .any(|result| result.kind == "Request"));
        assert!(state
            .search_results
            .iter()
            .any(|result| result.kind == "Authority"));
        assert!(state
            .search_results
            .iter()
            .any(|result| result.title.contains("COVID-19")));
    }

    #[test]
    fn test_fuzzy_search_overlay_renders_results_and_navigation() {
        let backend = TestBackend::new(140, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.search_open = true;
        state.search_query = "tor".to_string();
        state.refresh_search_results();

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("Fuzzy Search"));
        assert!(rendered_text.contains("tor"));
        assert!(rendered_text.contains("Activity Logs"));

        let initial_idx = state.selected_search_result_idx;
        state.select_next_search_result();
        assert_ne!(state.selected_search_result_idx, initial_idx);
    }

    #[test]
    fn test_bulk_request_selection_and_status_update() {
        let mut state = AppState::new();
        state.active_tab = Tab::Requests;

        assert!(!state.request_multi_select);
        state.toggle_request_multi_select();
        assert!(state.request_multi_select);

        state.toggle_selected_request();
        state.select_next();
        state.toggle_selected_request();
        assert_eq!(state.selected_request_ids.len(), 2);

        assert_eq!(state.bulk_update_selected_status("reviewed"), 2);
        assert!(state
            .tracked_requests
            .iter()
            .filter(|request| state.selected_request_ids.contains(&request.id))
            .all(|request| request.status == "reviewed"));
    }

    #[test]
    fn test_export_trigger_actions_are_recorded() {
        let mut state = AppState::new();

        state.trigger_export("json");
        state.trigger_export("csv");
        state.trigger_export("pdf");

        assert_eq!(state.export_actions, vec!["json", "csv", "pdf"]);
        assert_eq!(
            state.last_export_message.as_deref(),
            Some("Queued pdf export")
        );
    }

    #[test]
    fn test_help_overlay_renders_keybindings() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let backend = TestBackend::new(140, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();

        state.handle_key_event(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
        assert!(state.help_open);

        terminal
            .draw(|f| {
                draw_ui(f, &state);
            })
            .unwrap();

        let rendered_text = buffer_to_string(terminal.backend().buffer());
        assert!(rendered_text.contains("Help"));
        assert!(rendered_text.contains("Space"));
        assert!(rendered_text.contains("Bulk"));
        assert!(rendered_text.contains("Export"));
    }

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut result = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = buffer.get(x, y);
                result.push_str(&cell.symbol);
            }
            result.push('\n');
        }
        result
    }
}
