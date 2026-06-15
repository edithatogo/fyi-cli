use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Summary,
    Requests,
    Logs,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Summary, Tab::Requests, Tab::Logs]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Summary => "Summary Dashboard",
            Tab::Requests => "Tracked Requests",
            Tab::Logs => "Activity Logs",
        }
    }
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

#[derive(Debug, Clone)]
pub struct AppState {
    pub active_tab: Tab,
    pub total_tracked: usize,
    pub needs_attention_count: usize,
    pub action_now_count: usize,
    pub authorities_count: usize,
    pub recent_updates: usize,
    pub action_now_requests: Vec<RequestItem>,
    pub tracked_requests: Vec<RequestItem>,
    pub selected_request_idx: usize,
    pub logs: Vec<String>,
    pub selected_log_idx: usize,
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
                    self.selected_request_idx = (self.selected_request_idx + 1) % self.tracked_requests.len();
                }
            }
            Tab::Logs => {
                if !self.logs.is_empty() {
                    self.selected_log_idx = (self.selected_log_idx + 1) % self.logs.len();
                }
            }
            Tab::Summary => {}
        }
    }

    pub fn select_prev(&mut self) {
        match self.active_tab {
            Tab::Requests => {
                if !self.tracked_requests.is_empty() {
                    self.selected_request_idx = (self.selected_request_idx + self.tracked_requests.len() - 1)
                        % self.tracked_requests.len();
                }
            }
            Tab::Logs => {
                if !self.logs.is_empty() {
                    self.selected_log_idx = (self.selected_log_idx + self.logs.len() - 1) % self.logs.len();
                }
            }
            Tab::Summary => {}
        }
    }
}

pub fn draw_ui<B: Backend>(f: &mut Frame<B>, state: &AppState) {
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
    let current_tab_idx = Tab::all().iter().position(|t| *t == state.active_tab).unwrap_or(0);

    let tabs = Tabs::new(tabs_titles)
        .block(Block::default().borders(Borders::ALL).title(" FYI Request System "))
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
    }

    // Help Footer Bar
    let help_text = match state.active_tab {
        Tab::Summary => " Tab: Next Tab | Q/Esc: Quit ",
        Tab::Requests => " Tab: Next Tab | Up/Down: Navigate Requests | Q/Esc: Quit ",
        Tab::Logs => " Tab: Next Tab | Up/Down: Scroll Logs | Q/Esc: Quit ",
    };
    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help_paragraph, chunks[2]);
}

fn draw_summary_tab<B: Backend>(f: &mut Frame<B>, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Metric cards row
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
            Row::new(vec!["ID", "Authority", "Request Title", "Status", "Priority"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title(" Needs Action Now "))
        .widths(&[
            Constraint::Length(6),
            Constraint::Percentage(25),
            Constraint::Percentage(45),
            Constraint::Percentage(15),
            Constraint::Length(10),
        ]);

    f.render_widget(table, chunks[1]);
}

fn draw_requests_tab<B: Backend>(f: &mut Frame<B>, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // List of requests
            Constraint::Percentage(40), // Request detail preview pane
        ])
        .split(area);

    // Render Request Table List
    let rows = state.tracked_requests.iter().enumerate().map(|(idx, item)| {
        let mut style = Style::default();
        if idx == state.selected_request_idx {
            style = style.bg(Color::DarkGray).fg(Color::Yellow);
        } else if item.needs_attention {
            style = style.fg(Color::LightRed);
        }
        Row::new(vec![
            Cell::from(item.id.to_string()),
            Cell::from(item.authority.as_str()),
            Cell::from(item.title.as_str()),
            Cell::from(item.status.as_str()),
        ])
        .style(style)
    });

    let table = Table::new(rows)
        .header(
            Row::new(vec!["ID", "Authority", "Title", "Status"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title(" Tracked Requests "))
        .widths(&[
            Constraint::Length(6),
            Constraint::Percentage(30),
            Constraint::Percentage(45),
            Constraint::Percentage(20),
        ]);

    f.render_widget(table, chunks[0]);

    // Detail preview panel
    let preview_content = if let Some(selected_req) = state.tracked_requests.get(state.selected_request_idx) {
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
        .block(Block::default().borders(Borders::ALL).title(" Detail Preview "))
        .style(Style::default().fg(Color::White));

    f.render_widget(preview, chunks[1]);
}

fn draw_logs_tab<B: Backend>(f: &mut Frame<B>, state: &AppState, area: Rect) {
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
        .block(Block::default().borders(Borders::ALL).title(" Activity & Daemon Logs "))
        .style(Style::default().fg(Color::White));

    f.render_widget(logs_p, area);
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
        assert_eq!(state.active_tab, Tab::Summary);

        state.prev_tab();
        assert_eq!(state.active_tab, Tab::Logs);
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
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = AppState::new();

        terminal.draw(|f| {
            draw_ui(f, &state);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let rendered_text = buffer_to_string(buffer);
        assert!(rendered_text.contains("Summary Dashboard"));
        assert!(rendered_text.contains("Tracked Requests"));
        assert!(rendered_text.contains("Total Tracked"));
        assert!(rendered_text.contains("Needs Attention"));
        assert!(rendered_text.contains("COVID-19 Advisory Group Minutes"));
    }

    #[test]
    fn test_mock_rendering_requests_tab() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = AppState::new();
        state.active_tab = Tab::Requests;

        terminal.draw(|f| {
            draw_ui(f, &state);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let rendered_text = buffer_to_string(buffer);
        assert!(rendered_text.contains("Tracked Requests"));
        assert!(rendered_text.contains("Detail Preview"));
        assert!(rendered_text.contains("Smart Motorway Cost-Benefit Analysis"));
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

