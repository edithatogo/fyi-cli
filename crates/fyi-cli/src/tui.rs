//! Read-only Ratatui presentation. The caller supplies a bounded snapshot;
//! rendering and interaction never receive a database, keyring or network client.

pub mod adapter;
pub mod contracts;
pub mod runtime;
use contracts::{
    PresentationError, PresentationSnapshot, RequestItem, RequestStatus, SnapshotScope,
    TimelineState,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

#[cfg(test)]
#[path = "tui/presentation_tests.rs"]
mod presentation_tests;

const MIN_WIDTH: u16 = 36;
const MIN_HEIGHT: u16 = 10;
const MAX_QUERY_CHARS: usize = 64;
const HELP: &[&str] = &[
    "Read-only local snapshot",
    "Tab / Shift+Tab: switch views",
    "Up/Down or j/k: select a row",
    "Home/End, PageUp/PageDown: navigate",
    "Enter: selected request timeline",
    "d: request/event details; arrows scroll",
    "Timeline: [ / ] select a request",
    "/: filter visible requests; Enter: apply",
    "Filter: Esc clears; q is query text",
    "p: reveal/hide sensitive text locally",
    "F1 / ?: help; Up/Down: scroll help",
    "Esc: close panel or leave timeline",
    "q: quit; Ctrl+C/Ctrl+Q: quit anywhere",
    "No save, export, sync or credential actions",
];

/// Views in the existing dashboard. Drafts are request metadata, not an editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Summary,
    Requests,
    Timeline,
    Drafts,
}

impl Tab {
    pub fn all() -> &'static [Self] {
        &[Self::Summary, Self::Requests, Self::Timeline, Self::Drafts]
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Requests => "Requests",
            Self::Timeline => "Timeline",
            Self::Drafts => "Drafts",
        }
    }
}

/// No write, export, credential or service effects can leave this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiCommand {
    None,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    Loading,
    Ready,
    Failed(PresentationError),
}

/// Redaction includes search: hidden values cannot change filter results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyMode {
    Redacted,
    Visible,
}

/// UI state has no service handles. Intentionally has no content-bearing Debug.
pub struct AppState {
    pub active_tab: Tab,
    pub load_state: LoadState,
    pub privacy: PrivacyMode,
    pub help_open: bool,
    pub details_open: bool,
    pub search_open: bool,
    pub search_query: String,
    pub should_quit: bool,
    pub selected_timeline_idx: usize,
    snapshot: PresentationSnapshot,
    selected_request_id: Option<i64>,
    help_scroll: usize,
    details_scroll: usize,
    summary_scroll: usize,
    viewport: (u16, u16),
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Start in a truthful loading state with no demonstration data.
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Summary,
            load_state: LoadState::Loading,
            privacy: PrivacyMode::Redacted,
            help_open: false,
            details_open: false,
            search_open: false,
            search_query: String::new(),
            should_quit: false,
            selected_timeline_idx: 0,
            snapshot: PresentationSnapshot::default(),
            selected_request_id: None,
            help_scroll: 0,
            details_scroll: 0,
            summary_scroll: 0,
            viewport: (80, 24),
        }
    }

    /// Validate and take ownership of caller-supplied presentation data.
    pub fn from_snapshot(snapshot: PresentationSnapshot) -> Self {
        let mut state = Self::new();
        state.set_snapshot(snapshot);
        state
    }

    /// Preserve selection by stable ID across refreshes, never by stale row index.
    pub fn set_snapshot(&mut self, snapshot: PresentationSnapshot) {
        match snapshot.validate() {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.load_state = LoadState::Ready;
                self.details_open = false;
                self.details_scroll = 0;
                self.summary_scroll = 0;
                self.normalize_selection();
            }
            Err(error) => self.set_error(error),
        }
    }

    /// Clear old data while loading so a later failure cannot expose stale rows.
    pub fn set_loading(&mut self) {
        self.clear_snapshot();
        self.load_state = LoadState::Loading;
    }

    /// A fixed error category is the only error information accepted by the UI.
    pub fn set_error(&mut self, error: PresentationError) {
        self.clear_snapshot();
        self.load_state = LoadState::Failed(error);
    }

    fn clear_snapshot(&mut self) {
        self.snapshot = PresentationSnapshot::default();
        self.selected_request_id = None;
        self.selected_timeline_idx = 0;
        self.search_query.clear();
        self.search_open = false;
        self.details_open = false;
        self.details_scroll = 0;
        self.summary_scroll = 0;
        self.privacy = PrivacyMode::Redacted;
    }

    /// Return only rows in the current local view; never search concealed text.
    pub fn visible_requests(&self) -> Vec<&RequestItem> {
        let query = self.search_query.to_lowercase();
        self.snapshot
            .requests
            .iter()
            .filter(|request| {
                (self.active_tab != Tab::Drafts || request.status == RequestStatus::Draft)
                    && (self.privacy == PrivacyMode::Redacted
                        || query.is_empty()
                        || request.title.to_lowercase().contains(&query)
                        || request
                            .authority
                            .as_deref()
                            .is_some_and(|name| name.to_lowercase().contains(&query))
                        || request.status.label().to_lowercase().contains(&query)
                        || request.id.to_string().contains(&query))
            })
            .collect()
    }

    pub fn selected_request(&self) -> Option<&RequestItem> {
        self.visible_requests()
            .into_iter()
            .find(|request| Some(request.id) == self.selected_request_id)
    }

    fn normalize_selection(&mut self) {
        let visible = self.visible_requests();
        let selected = visible
            .iter()
            .find(|request| Some(request.id) == self.selected_request_id)
            .or_else(|| visible.first())
            .map(|request| request.id);
        if selected != self.selected_request_id {
            self.selected_timeline_idx = 0;
        }
        self.selected_request_id = selected;
        self.selected_timeline_idx = self
            .selected_timeline_idx
            .min(self.timeline_len().saturating_sub(1));
    }

    fn timeline_len(&self) -> usize {
        match self.selected_request().map(|request| &request.timeline) {
            Some(TimelineState::Ready(events)) => events.len(),
            _ => 0,
        }
    }

    pub fn next_tab(&mut self) {
        self.change_tab(true);
    }

    pub fn prev_tab(&mut self) {
        self.change_tab(false);
    }

    fn change_tab(&mut self, forward: bool) {
        let tabs = Tab::all();
        let current = tabs
            .iter()
            .position(|tab| *tab == self.active_tab)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % tabs.len()
        } else {
            (current + tabs.len() - 1) % tabs.len()
        };
        self.active_tab = tabs[next];
        self.normalize_selection();
    }

    pub fn select_next(&mut self) {
        self.move_selection(1, true);
    }

    pub fn select_prev(&mut self) {
        self.move_selection(1, false);
    }

    fn move_selection(&mut self, amount: usize, forward: bool) {
        self.normalize_selection();
        if self.active_tab == Tab::Summary {
            self.summary_scroll = move_index(
                self.summary_scroll,
                self.max_text_scroll(&summary_text(self)) + 1,
                amount,
                forward,
            );
        } else if self.active_tab == Tab::Timeline {
            self.selected_timeline_idx = move_index(
                self.selected_timeline_idx,
                self.timeline_len(),
                amount,
                forward,
            );
        } else if matches!(self.active_tab, Tab::Requests | Tab::Drafts) {
            self.move_request(amount, forward);
        }
    }

    fn move_request(&mut self, amount: usize, forward: bool) {
        let rows = self.visible_requests();
        let current = rows
            .iter()
            .position(|request| Some(request.id) == self.selected_request_id)
            .unwrap_or(0);
        let next = move_index(current, rows.len(), amount, forward);
        let id = rows.get(next).map(|request| request.id);
        if id != self.selected_request_id {
            self.selected_timeline_idx = 0;
        }
        self.selected_request_id = id;
    }

    fn page_size(&self) -> usize {
        usize::from(self.viewport.1.saturating_sub(10)).max(1)
    }

    /// Resize only affects navigation; every draw uses the current frame area.
    pub fn handle_event(&mut self, event: Event) -> TuiCommand {
        match event {
            Event::Key(key) => self.handle_key_event(key),
            Event::Resize(width, height) => {
                self.viewport = (width, height);
                self.normalize_selection();
                self.details_scroll = self
                    .details_scroll
                    .min(self.max_text_scroll(&details_text(self)));
                self.summary_scroll = self
                    .summary_scroll
                    .min(self.max_text_scroll(&summary_text(self)));
                TuiCommand::None
            }
            _ => TuiCommand::None,
        }
    }

    /// Keyboard transitions mutate view state only, with a global emergency exit.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> TuiCommand {
        if key.kind == KeyEventKind::Release {
            return TuiCommand::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'q'))
        {
            self.should_quit = true;
            return TuiCommand::Quit;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return TuiCommand::None;
        }
        let navigation = matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Char('j' | 'k')
        );
        if key.kind == KeyEventKind::Repeat
            && !navigation
            && !(self.search_open && matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace))
        {
            return TuiCommand::None;
        }
        if !self.search_open && matches!(key.code, KeyCode::Char('q' | 'Q')) {
            self.should_quit = true;
            return TuiCommand::Quit;
        }
        if !self.search_open && key.code == KeyCode::Char('p') {
            self.privacy = match self.privacy {
                PrivacyMode::Redacted => PrivacyMode::Visible,
                PrivacyMode::Visible => PrivacyMode::Redacted,
            };
            self.search_query.clear();
            self.details_open = false;
            self.normalize_selection();
            return TuiCommand::None;
        }
        if self.help_open {
            match key.code {
                KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('?' | 'q') => self.help_open = false,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.help_scroll = self.help_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.help_scroll = (self.help_scroll + 1).min(HELP.len() - 1);
                }
                KeyCode::Home | KeyCode::PageUp => self.help_scroll = 0,
                KeyCode::End | KeyCode::PageDown => self.help_scroll = HELP.len() - 1,
                _ => {}
            }
            return TuiCommand::None;
        }
        if self.search_open && self.privacy == PrivacyMode::Visible {
            self.handle_search_key(key.code);
            return TuiCommand::None;
        }
        if self.details_open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('d') => self.details_open = false,
                KeyCode::Up | KeyCode::Char('k') => self.scroll_details(1, false),
                KeyCode::Down | KeyCode::Char('j') => self.scroll_details(1, true),
                KeyCode::PageUp => self.scroll_details(self.page_size(), false),
                KeyCode::PageDown => self.scroll_details(self.page_size(), true),
                KeyCode::Home => self.scroll_details(usize::MAX, false),
                KeyCode::End => self.scroll_details(usize::MAX, true),
                _ => {}
            }
            return TuiCommand::None;
        }
        match key.code {
            KeyCode::Char('q' | 'Q') => {
                self.should_quit = true;
                return TuiCommand::Quit;
            }
            KeyCode::Esc if self.active_tab == Tab::Timeline => {
                self.active_tab = Tab::Requests;
                self.normalize_selection();
            }
            KeyCode::Esc => {
                self.should_quit = true;
                return TuiCommand::Quit;
            }
            KeyCode::F(1) | KeyCode::Char('?') => {
                self.help_open = true;
                self.help_scroll = 0;
            }
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => self.prev_tab(),
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab => self.prev_tab(),
            KeyCode::Char('d') | KeyCode::Enter
                if self.privacy == PrivacyMode::Visible
                    && (key.code == KeyCode::Char('d') || self.active_tab == Tab::Timeline)
                    && self.selected_request().is_some() =>
            {
                self.details_open = true;
                self.details_scroll = 0;
            }
            KeyCode::Char('/')
                if self.privacy == PrivacyMode::Visible && self.load_state == LoadState::Ready =>
            {
                self.search_open = true;
                self.search_query.clear();
                self.active_tab = Tab::Requests;
                self.normalize_selection();
            }
            KeyCode::Enter if matches!(self.active_tab, Tab::Requests | Tab::Drafts) => {
                self.active_tab = Tab::Timeline;
                self.normalize_selection();
            }
            KeyCode::Char('[') if self.active_tab == Tab::Timeline => self.move_request(1, false),
            KeyCode::Char(']') if self.active_tab == Tab::Timeline => self.move_request(1, true),
            KeyCode::Up | KeyCode::Char('k') => self.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::PageUp => self.move_selection(self.page_size(), false),
            KeyCode::PageDown => self.move_selection(self.page_size(), true),
            KeyCode::Home => self.move_selection(usize::MAX, false),
            KeyCode::End => self.move_selection(usize::MAX, true),
            _ => {}
        }
        TuiCommand::None
    }

    fn max_text_scroll(&self, text: &str) -> usize {
        wrap_lines(text, self.viewport.0.saturating_sub(2).max(1))
            .len()
            .saturating_sub(usize::from(self.viewport.1.saturating_sub(8)).max(1))
    }

    fn scroll_details(&mut self, amount: usize, forward: bool) {
        self.details_scroll = move_index(
            self.details_scroll,
            self.max_text_scroll(&details_text(self)) + 1,
            amount,
            forward,
        );
    }

    fn handle_search_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.search_query.clear();
                self.search_open = false;
            }
            KeyCode::Enter => self.search_open = false,
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            KeyCode::Char(ch)
                if !ch.is_control() && self.search_query.chars().count() < MAX_QUERY_CHARS =>
            {
                self.search_query.push(ch);
                self.search_query = contracts::safe_text(&self.search_query);
            }
            _ => {}
        }
        self.normalize_selection();
    }
}

fn move_index(current: usize, len: usize, amount: usize, forward: bool) -> usize {
    let last = len.saturating_sub(1);
    let current = current.min(last);
    if forward {
        current.saturating_add(amount).min(last)
    } else {
        current.saturating_sub(amount)
    }
}

fn text_value(value: &str, privacy: PrivacyMode) -> String {
    match privacy {
        PrivacyMode::Redacted => "[hidden]".to_string(),
        PrivacyMode::Visible => contracts::safe_text(value),
    }
}

fn optional_value(value: Option<&str>, privacy: PrivacyMode) -> String {
    value.map_or_else(
        || "Unavailable".to_string(),
        |value| text_value(value, privacy),
    )
}

fn selection_style(selected: bool) -> Style {
    if selected {
        Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default()
    }
}

/// Render solely from the supplied state and frame size; no clock or I/O reads.
pub fn draw_ui(frame: &mut Frame<'_>, state: &AppState) {
    let area = frame.area();
    if area.is_empty() {
        return;
    }
    frame.render_widget(Clear, area);
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(
            Paragraph::new("FYI read-only\nResize terminal to at least 36x10.\nCtrl+C / q: quit")
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);
    draw_header(frame, state, sections[0]);
    if state.help_open {
        let text = HELP
            .iter()
            .skip(state.help_scroll)
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        draw_message(
            frame,
            sections[1],
            "Help (Up/Down scroll, Esc close)",
            &text,
        );
    } else {
        match state.load_state {
            LoadState::Loading => draw_message(
                frame,
                sections[1],
                "Loading",
                "Waiting for a local presentation snapshot.\nNo requests or metrics loaded.",
            ),
            LoadState::Failed(error) => {
                draw_message(
                    frame,
                    sections[1],
                    "Unable to display requests",
                    error.message(),
                );
            }
            LoadState::Ready if state.privacy == PrivacyMode::Redacted => draw_message(
                frame,
                sections[1],
                "Privacy view",
                "Requests, counts and timelines are hidden.\nPress p to reveal this local view.",
            ),
            LoadState::Ready if state.details_open => draw_scrolled_message(
                frame,
                sections[1],
                "Details (Up/Down scroll, Esc close)",
                &details_text(state),
                state.details_scroll,
            ),
            LoadState::Ready => match state.active_tab {
                Tab::Summary => draw_summary_tab(frame, state, sections[1]),
                Tab::Requests | Tab::Drafts => draw_requests_tab(frame, state, sections[1]),
                Tab::Timeline => draw_timeline_tab(frame, state, sections[1]),
            },
        }
    }
    draw_controls(frame, state, sections[2]);
}

fn draw_header(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let privacy = match state.privacy {
        PrivacyMode::Redacted => "data hidden",
        PrivacyMode::Visible => "data visible",
    };
    let title = format!(" FYI | read-only | {privacy} ");
    if area.width < 56 {
        frame.render_widget(
            Paragraph::new(format!("[{}]  Tab: switch", state.active_tab.title()))
                .block(Block::default().borders(Borders::ALL).title(title)),
            area,
        );
    } else {
        let tabs = Tabs::new(Tab::all().iter().map(|tab| {
            Line::from(if *tab == state.active_tab {
                format!("[{}]", tab.title())
            } else {
                tab.title().to_string()
            })
        }))
        .block(Block::default().borders(Borders::ALL).title(title))
        .select(
            Tab::all()
                .iter()
                .position(|tab| *tab == state.active_tab)
                .unwrap_or(0),
        )
        .highlight_style(selection_style(true));
        frame.render_widget(tabs, area);
    }
}

fn draw_controls(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let message = if state.search_open && state.privacy == PrivacyMode::Visible {
        format!(
            "Filter: {}_ | Enter apply; Esc clear",
            contracts::safe_text(&state.search_query)
        )
    } else if !state.search_query.is_empty() && state.privacy == PrivacyMode::Visible {
        format!(
            "Filter: {} | / edit; p hide; Ctrl+C quit",
            contracts::safe_text(&state.search_query)
        )
    } else if area.width < 70 {
        "Tab view | p privacy | ? help | q quit".to_string()
    } else {
        "Tab view | arrows/Home/End/PgUp/PgDn | / filter | p privacy | ? help | q quit".to_string()
    };
    frame.render_widget(
        Paragraph::new(message).block(Block::default().borders(Borders::ALL).title(" Controls ")),
        area,
    );
}

fn draw_message(frame: &mut Frame<'_>, area: Rect, title: &str, text: &str) {
    draw_scrolled_message(frame, area, title, text, 0);
}

fn draw_scrolled_message(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    text: &str,
    scroll: usize,
) {
    let lines = wrap_lines(text, area.width.saturating_sub(2).max(1));
    let max_scroll = lines
        .len()
        .saturating_sub(usize::from(area.height.saturating_sub(2)).max(1));
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .scroll((u16::try_from(scroll.min(max_scroll)).unwrap_or(u16::MAX), 0)),
        area,
    );
}

// Use the same bounded grapheme wrapping for both drawing and scroll bounds.
// Ratatui's Paragraph line-count API is unstable in the pinned dependency.
fn wrap_lines(text: &str, width: u16) -> Vec<Line<'static>> {
    let width = usize::from(width.max(1));
    let mut lines = Vec::new();
    for raw_line in text.split('\n') {
        let source = Line::from(raw_line);
        let mut current = String::new();
        let mut used = 0;
        for grapheme in source.styled_graphemes(Style::default()) {
            let cells = Span::raw(grapheme.symbol).width();
            if used + cells > width && !current.is_empty() {
                lines.push(Line::from(std::mem::take(&mut current)));
                used = 0;
            }
            current.push_str(grapheme.symbol);
            used += cells;
        }
        lines.push(Line::from(current));
    }
    lines
}

fn draw_summary_tab(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    draw_scrolled_message(
        frame,
        area,
        "Summary (supplied data only)",
        &summary_text(state),
        state.summary_scroll,
    );
}

fn summary_text(state: &AppState) -> String {
    let metrics = state.snapshot.summary();
    let scope = match state.snapshot.scope {
        SnapshotScope::Complete => format!("Complete snapshot; backend total: {}", metrics.loaded),
        SnapshotScope::Partial { total: Some(total) } => {
            format!("Partial snapshot; backend total: {total}")
        }
        SnapshotScope::Partial { total: None } => {
            "Partial snapshot; backend total unavailable".to_string()
        }
    };
    let mut lines = vec![
        format!("Loaded requests: {}", metrics.loaded),
        scope,
        format!(
            "Drafts: {} | Authority labels: {}",
            metrics.drafts, metrics.known_authorities
        ),
        format!(
            "Needs attention: {} | Unknown: {}",
            metrics.needs_attention, metrics.unknown_attention
        ),
    ];
    if metrics.loaded == 0 {
        lines.push("No requests in this snapshot.".to_string());
    } else {
        lines.push("".to_string());
        lines.push("Statuses in the loaded snapshot:".to_string());
        let mut statuses = std::collections::BTreeMap::new();
        for request in &state.snapshot.requests {
            *statuses.entry(request.status.label()).or_insert(0usize) += 1;
        }
        lines.extend(
            statuses
                .into_iter()
                .map(|(status, count)| format!("{status}: {count}")),
        );
    }
    lines.join("\n")
}

fn details_text(state: &AppState) -> String {
    let Some(request) = state.selected_request() else {
        return "No request selected in this snapshot.".to_string();
    };
    let mut lines = vec![
        format!("ID: {}", text_value(&request.id.to_string(), state.privacy)),
        format!("Status: {}", request.status.label()),
        format!(
            "Authority: {}",
            optional_value(request.authority.as_deref(), state.privacy)
        ),
        format!(
            "Updated: {}",
            optional_value(request.updated_at.as_deref(), state.privacy)
        ),
        format!(
            "Attention: {}",
            match request.needs_attention {
                Some(true) => "Yes",
                Some(false) => "No",
                None => "Unavailable",
            }
        ),
        "Title:".to_string(),
        text_value(&request.title, state.privacy),
    ];
    if state.active_tab == Tab::Timeline {
        if let TimelineState::Ready(events) = &request.timeline {
            if let Some(event) = events.get(state.selected_timeline_idx) {
                lines.extend([
                    "".to_string(),
                    format!("Event: {}", event.kind.label()),
                    format!(
                        "Time: {}",
                        optional_value(event.occurred_at.as_deref(), state.privacy)
                    ),
                    "Summary:".to_string(),
                    text_value(&event.summary, state.privacy),
                ]);
            }
        }
    }
    lines.join("\n")
}

fn draw_requests_tab(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let requests = state.visible_requests();
    if requests.is_empty() {
        let text = if !state.search_query.is_empty() && state.privacy == PrivacyMode::Visible {
            "No requests match this local filter.\nPress / then Esc to clear it."
        } else if state.active_tab == Tab::Drafts {
            "No drafts in this snapshot."
        } else {
            "No requests in this snapshot."
        };
        draw_message(frame, area, state.active_tab.title(), text);
        return;
    }
    let (list_area, detail_area) = if area.width >= 110 && area.height >= 12 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };
    let selected = requests
        .iter()
        .position(|request| Some(request.id) == state.selected_request_id)
        .unwrap_or(0);
    let capacity = usize::from(list_area.height.saturating_sub(3)).max(1);
    let start = selected.saturating_sub(capacity - 1);
    let wide = list_area.width >= 70;
    let rows = requests
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, request)| {
            let marker = if index == selected { ">" } else { " " };
            let attention = match request.needs_attention {
                Some(true) => "!",
                Some(false) => " ",
                None => "?",
            };
            let mut cells = vec![
                Cell::from(format!("{marker}{attention}")),
                Cell::from(text_value(&request.id.to_string(), state.privacy)),
                Cell::from(request.status.label()),
            ];
            if wide {
                cells.push(Cell::from(optional_value(
                    request.authority.as_deref(),
                    state.privacy,
                )));
            }
            cells.push(Cell::from(text_value(&request.title, state.privacy)));
            Row::new(cells).style(selection_style(index == selected))
        });
    let mut widths = vec![
        Constraint::Length(2),
        Constraint::Length(8),
        Constraint::Length(18),
    ];
    let mut headings = vec!["  ", "ID", "Status"];
    if wide {
        widths.push(Constraint::Length(18));
        headings.push("Authority");
    } else {
        widths[1] = Constraint::Length(6);
        widths[2] = Constraint::Length(12);
    }
    widths.push(Constraint::Min(1));
    headings.push("Title");
    let table = Table::new(rows, widths)
        .header(Row::new(headings).style(Style::default().add_modifier(Modifier::BOLD)))
        .block(Block::default().borders(Borders::ALL).title(format!(
            " {} {}-{} / {} ",
            state.active_tab.title(),
            start + 1,
            (start + capacity).min(requests.len()),
            requests.len()
        )));
    frame.render_widget(table, list_area);
    if let Some(detail_area) = detail_area {
        if let Some(request) = state.selected_request() {
            let text = format!(
                "ID: {}\nTitle: {}\nAuthority: {}\nStatus: {}\nUpdated: {}\nAttention: {}\n\nEnter: supplied timeline\n!: needs attention; ?: unknown",
                text_value(&request.id.to_string(), state.privacy),
                text_value(&request.title, state.privacy),
                optional_value(request.authority.as_deref(), state.privacy),
                request.status.label(),
                optional_value(request.updated_at.as_deref(), state.privacy),
                match request.needs_attention { Some(true) => "Yes", Some(false) => "No", None => "Unavailable" },
            );
            draw_message(frame, detail_area, "Request details", &text);
        }
    }
}

fn draw_timeline_tab(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let Some(request) = state.selected_request() else {
        draw_message(
            frame,
            area,
            "Timeline",
            "No request selected in this snapshot.",
        );
        return;
    };
    let title = format!(
        " Timeline | request {} | [ / ] request ",
        text_value(&request.id.to_string(), state.privacy)
    );
    let events = match &request.timeline {
        TimelineState::Unavailable => {
            draw_message(
                frame,
                area,
                &title,
                "Timeline unavailable. No history has been supplied.",
            );
            return;
        }
        TimelineState::Loading => {
            draw_message(
                frame,
                area,
                &title,
                "Loading the supplied request timeline.",
            );
            return;
        }
        TimelineState::Failed(error) => {
            draw_message(frame, area, &title, error.message());
            return;
        }
        TimelineState::Ready(events) if events.is_empty() => {
            draw_message(frame, area, &title, "No timeline events in this snapshot.");
            return;
        }
        TimelineState::Ready(events) => events,
    };
    let selected = state
        .selected_timeline_idx
        .min(events.len().saturating_sub(1));
    let capacity = usize::from(area.height.saturating_sub(3)).max(1);
    let start = selected.saturating_sub(capacity - 1);
    let rows = events
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .map(|(index, event)| {
            Row::new(vec![
                Cell::from(if index == selected { ">" } else { " " }),
                Cell::from(optional_value(event.occurred_at.as_deref(), state.privacy)),
                Cell::from(event.kind.label()),
                Cell::from(text_value(&event.summary, state.privacy)),
            ])
            .style(selection_style(index == selected))
        });
    let table = Table::new(
        rows,
        [
            Constraint::Length(1),
            Constraint::Length(if area.width >= 75 { 20 } else { 10 }),
            Constraint::Length(if area.width >= 75 { 16 } else { 10 }),
            Constraint::Min(1),
        ],
    )
    .header(
        Row::new(["", "Time", "Event", "Summary"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(table, area);
}
