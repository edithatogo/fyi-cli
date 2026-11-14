use super::contracts::{TimelineItem, TimelineKind};
use super::*;
use crossterm::event::{Event, KeyEventKind};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
use std::path::PathBuf;

const PRIVATE_TITLE: &str = "PRIVATE SYNTHETIC TITLE";
const PRIVATE_AUTHORITY: &str = "Synthetic Private Authority";
const PRIVATE_TIME: &str = "2026-01-15T09:42:17Z";
const PRIVATE_EVENT: &str = "PRIVATE SYNTHETIC EVENT";

fn request(id: i64, title: &str, status: RequestStatus) -> RequestItem {
    RequestItem {
        id,
        authority: None,
        title: title.to_string(),
        status,
        needs_attention: None,
        updated_at: None,
        timeline: TimelineState::Unavailable,
    }
}

fn snapshot() -> PresentationSnapshot {
    let mut first = request(7101, PRIVATE_TITLE, RequestStatus::AwaitingResponse);
    first.authority = Some(PRIVATE_AUTHORITY.to_string());
    first.needs_attention = Some(true);
    first.updated_at = Some(PRIVATE_TIME.to_string());
    first.timeline = TimelineState::Ready(vec![
        TimelineItem {
            occurred_at: Some("2026-01-12T08:00:00Z".to_string()),
            kind: TimelineKind::Created,
            summary: "Synthetic request recorded".to_string(),
        },
        TimelineItem {
            occurred_at: Some(PRIVATE_TIME.to_string()),
            kind: TimelineKind::StatusChanged,
            summary: PRIVATE_EVENT.to_string(),
        },
        TimelineItem {
            occurred_at: None,
            kind: TimelineKind::Note,
            summary: "Synthetic note with no supplied timestamp".to_string(),
        },
    ]);
    let mut draft = request(7102, "Draft courtyard plans", RequestStatus::Draft);
    draft.authority = Some("Synthetic Planning Authority".to_string());
    draft.needs_attention = Some(false);
    draft.timeline = TimelineState::Ready(Vec::new());
    let mut complete = request(7103, "Bridge records", RequestStatus::Successful);
    complete.authority = Some("Synthetic Planning Authority".to_string());
    complete.needs_attention = Some(false);
    PresentationSnapshot {
        requests: vec![
            first,
            draft,
            complete,
            request(7104, "Forest records", RequestStatus::Unknown),
        ],
        scope: SnapshotScope::Partial { total: Some(40) },
    }
}

fn populated() -> AppState {
    AppState::from_snapshot(snapshot())
}

fn visible() -> AppState {
    let mut state = populated();
    press(&mut state, KeyCode::Char('p'));
    assert_eq!(state.privacy, PrivacyMode::Visible);
    state
}

fn press(state: &mut AppState, code: KeyCode) -> TuiCommand {
    state.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE))
}

fn text(state: &mut AppState, value: &str) {
    for ch in value.chars() {
        press(state, KeyCode::Char(ch));
    }
}

fn buffer_text(buffer: &Buffer) -> String {
    // Delimit complete rows so blank terminal cells survive review without
    // trailing whitespace. This records every cell, including the borders.
    let mut output = format!("{}x{}\n", buffer.area.width, buffer.area.height);
    for y in buffer.area.y..buffer.area.bottom() {
        output.push('|');
        for x in buffer.area.x..buffer.area.right() {
            output.push_str(buffer[(x, y)].symbol());
        }
        output.push_str("|\n");
    }
    output
}

fn render(state: &AppState, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw_ui(frame, state)).unwrap();
    buffer_text(terminal.backend().buffer())
}

fn text_without_layout(rendered: &str) -> String {
    rendered
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '|' && !('\u{2500}'..='\u{257f}').contains(ch))
        .collect()
}

fn assert_snapshot(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/tui/snapshots")
        .join(format!("{name}.txt"));
    if std::env::var("FYI_UPDATE_TUI_SNAPSHOTS").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("Missing render fixture {}: {error}", path.display()));
    assert_eq!(actual, expected, "render fixture {}", path.display());
}

fn assert_private(rendered: &str) {
    for value in [
        PRIVATE_TITLE,
        PRIVATE_AUTHORITY,
        PRIVATE_TIME,
        PRIVATE_EVENT,
        "7101",
        "Draft courtyard plans",
        "Bridge records",
        "Forest records",
    ] {
        assert!(!rendered.contains(value), "private text appeared: {value}");
    }
}

#[test]
fn production_state_does_not_invent_requests_or_metrics() {
    let state = AppState::new();
    assert_eq!(state.load_state, LoadState::Loading);
    assert_eq!(state.privacy, PrivacyMode::Redacted);
    assert!(state.snapshot.requests.is_empty());
    assert_eq!(state.snapshot.summary().loaded, 0);
    assert!(state.selected_request().is_none());
    assert_snapshot("loading_80x20", &render(&state, 80, 20));
}

#[test]
fn key_release_does_not_activate_a_shortcut() {
    let mut state = populated();
    for code in [
        KeyCode::Char('q'),
        KeyCode::Char('p'),
        KeyCode::Char('/'),
        KeyCode::Tab,
        KeyCode::F(1),
    ] {
        assert_eq!(
            state.handle_key_event(KeyEvent::new_with_kind(
                code,
                KeyModifiers::NONE,
                KeyEventKind::Release,
            )),
            TuiCommand::None
        );
    }
    assert!(!state.should_quit);
    assert!(!state.search_open);
    assert!(!state.help_open);
    assert_eq!(state.active_tab, Tab::Summary);
    assert_eq!(state.privacy, PrivacyMode::Redacted);
}

#[test]
fn control_c_can_exit_from_every_overlay_and_load_state() {
    for state_kind in 0..4 {
        for overlay in 0..4 {
            for shortcut in ['c', 'q'] {
                let mut state = match state_kind {
                    0 => AppState::new(),
                    1 => AppState::from_snapshot(PresentationSnapshot::default()),
                    2 => populated(),
                    _ => {
                        let mut state = populated();
                        state.set_error(PresentationError::Unavailable);
                        state
                    }
                };
                state.help_open = overlay == 1;
                state.search_open = overlay == 2;
                state.details_open = overlay == 3;
                assert_eq!(
                    state.handle_key_event(KeyEvent::new(
                        KeyCode::Char(shortcut),
                        KeyModifiers::CONTROL,
                    )),
                    TuiCommand::Quit
                );
                assert!(state.should_quit);
            }
        }
    }
}

#[test]
fn repeated_toggles_are_ignored_but_navigation_repeats() {
    let mut state = visible();
    state.active_tab = Tab::Requests;
    let initial = state.selected_request().unwrap().id;
    for code in [
        KeyCode::Char('p'),
        KeyCode::F(1),
        KeyCode::Tab,
        KeyCode::Char('q'),
    ] {
        state.handle_key_event(KeyEvent::new_with_kind(
            code,
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        ));
    }
    assert_eq!(state.active_tab, Tab::Requests);
    assert_eq!(state.privacy, PrivacyMode::Visible);
    assert!(!state.help_open);
    assert!(!state.should_quit);
    state.handle_key_event(KeyEvent::new_with_kind(
        KeyCode::Down,
        KeyModifiers::NONE,
        KeyEventKind::Repeat,
    ));
    assert_ne!(state.selected_request().unwrap().id, initial);
}

#[test]
fn tabs_wrap_forward_and_backward() {
    let mut state = populated();
    for expected in [Tab::Requests, Tab::Timeline, Tab::Drafts, Tab::Summary] {
        press(&mut state, KeyCode::Tab);
        assert_eq!(state.active_tab, expected);
    }
    for expected in [Tab::Drafts, Tab::Timeline, Tab::Requests, Tab::Summary] {
        state.handle_key_event(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(state.active_tab, expected);
    }
}

#[test]
fn drafts_and_timeline_keep_request_identity() {
    let mut state = visible();
    for _ in 0..3 {
        press(&mut state, KeyCode::Tab);
    }
    assert_eq!(state.active_tab, Tab::Drafts);
    assert_eq!(state.visible_requests().len(), 1);
    assert_eq!(state.selected_request().unwrap().id, 7102);
    press(&mut state, KeyCode::Enter);
    assert_eq!(state.active_tab, Tab::Timeline);
    assert_eq!(state.selected_request().unwrap().id, 7102);
    press(&mut state, KeyCode::Esc);
    assert_eq!(state.active_tab, Tab::Requests);
    assert!(!state.should_quit);
}

#[test]
fn replace_snapshot_preserves_identity_and_removes_stale_selection() {
    let mut state = visible();
    state.active_tab = Tab::Requests;
    state.select_next();
    state.select_next();
    assert_eq!(state.selected_request().unwrap().id, 7103);
    let mut reordered = snapshot();
    reordered.requests.reverse();
    state.set_snapshot(reordered);
    assert_eq!(state.selected_request().unwrap().id, 7103);
    let mut removed = snapshot();
    removed.requests.retain(|item| item.id != 7103);
    state.set_snapshot(removed);
    assert!(state.selected_request().is_some());
    assert_ne!(state.selected_request().unwrap().id, 7103);
    state.set_snapshot(PresentationSnapshot::default());
    assert!(state.selected_request().is_none());
    for code in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageDown,
    ] {
        press(&mut state, code);
        assert!(state.selected_request().is_none());
    }
}

#[test]
fn loading_errors_and_invalid_replacements_clear_stale_content() {
    let mut state = visible();
    state.set_loading();
    assert_eq!(state.load_state, LoadState::Loading);
    assert!(state.snapshot.requests.is_empty());
    assert_private(&render(&state, 100, 24));
    state.set_snapshot(snapshot());
    state.set_error(PresentationError::Unavailable);
    assert_eq!(
        state.load_state,
        LoadState::Failed(PresentationError::Unavailable)
    );
    assert!(state.snapshot.requests.is_empty());
    assert_private(&render(&state, 100, 24));
    state.set_snapshot(snapshot());
    let mut invalid = snapshot();
    invalid.requests.push(invalid.requests[0].clone());
    state.set_snapshot(invalid);
    assert_eq!(
        state.load_state,
        LoadState::Failed(PresentationError::InvalidSnapshot)
    );
    assert!(state.snapshot.requests.is_empty());
    assert!(state.selected_request().is_none());
}

#[test]
fn private_text_cannot_be_queried_or_rendered_on_any_tab() {
    let mut state = populated();
    press(&mut state, KeyCode::Char('/'));
    assert!(!state.search_open);
    assert!(state.search_query.is_empty());
    for tab in [Tab::Summary, Tab::Requests, Tab::Timeline, Tab::Drafts] {
        state.active_tab = tab;
        for (width, height) in [(120, 30), (80, 24), (40, 12)] {
            assert_private(&render(&state, width, height));
        }
    }
}

#[test]
fn redacted_ready_views_do_not_disclose_counts_statuses_or_event_existence() {
    let mut populated = populated();
    let mut empty = AppState::from_snapshot(PresentationSnapshot::default());
    let mut different = snapshot();
    different.requests.truncate(1);
    different.requests[0].status = RequestStatus::Refused;
    different.requests[0].timeline = TimelineState::Failed(PresentationError::Unavailable);
    let mut different = AppState::from_snapshot(different);
    for tab in [Tab::Summary, Tab::Requests, Tab::Timeline, Tab::Drafts] {
        populated.active_tab = tab;
        empty.active_tab = tab;
        different.active_tab = tab;
        for (width, height) in [(120, 30), (80, 24), (40, 12)] {
            let expected = render(&empty, width, height);
            assert_eq!(render(&populated, width, height), expected);
            assert_eq!(render(&different, width, height), expected);
        }
    }
}

#[test]
fn redaction_clears_search_and_stale_terminal_cells() {
    let mut state = visible();
    state.active_tab = Tab::Requests;
    press(&mut state, KeyCode::Char('/'));
    text(&mut state, "PRIVATE");
    assert_eq!(state.visible_requests().len(), 1);
    press(&mut state, KeyCode::Enter);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|frame| draw_ui(frame, &state)).unwrap();
    assert!(buffer_text(terminal.backend().buffer()).contains(PRIVATE_TITLE));
    press(&mut state, KeyCode::Char('p'));
    assert_eq!(state.privacy, PrivacyMode::Redacted);
    assert!(state.search_query.is_empty());
    assert!(!state.search_open);
    assert_eq!(state.visible_requests().len(), 4);
    terminal.draw(|frame| draw_ui(frame, &state)).unwrap();
    assert_private(&buffer_text(terminal.backend().buffer()));
}

#[test]
fn filter_is_local_unicode_capable_and_treats_q_as_text() {
    let mut supplied = snapshot();
    supplied.requests[0].title = "quiet Élan 水道 records".to_string();
    let mut state = AppState::from_snapshot(supplied);
    press(&mut state, KeyCode::Char('p'));
    state.active_tab = Tab::Requests;
    press(&mut state, KeyCode::Char('/'));
    text(&mut state, "q");
    assert!(!state.should_quit);
    assert_eq!(state.search_query, "q");
    assert_eq!(state.visible_requests().len(), 1);
    press(&mut state, KeyCode::Backspace);
    text(&mut state, "Élan");
    assert_eq!(state.visible_requests().len(), 1);
    assert_eq!(state.selected_request().unwrap().id, 7101);
    press(&mut state, KeyCode::Esc);
    assert!(!state.search_open);
    assert!(!state.should_quit);
    assert!(state.search_query.is_empty());
    assert_eq!(state.visible_requests().len(), 4);
    press(&mut state, KeyCode::Char('/'));
    text(&mut state, "水道");
    press(&mut state, KeyCode::Enter);
    assert!(!state.search_open);
    assert_eq!(state.visible_requests().len(), 1);
}

#[test]
fn filter_length_is_bounded_and_missing_matches_have_no_selection() {
    let mut state = visible();
    state.active_tab = Tab::Requests;
    press(&mut state, KeyCode::Char('/'));
    text(&mut state, &"界".repeat(100));
    assert_eq!(state.search_query.chars().count(), 64);
    assert!(state.visible_requests().is_empty());
    assert!(state.selected_request().is_none());
    for code in [KeyCode::Up, KeyCode::Down, KeyCode::Home, KeyCode::End] {
        press(&mut state, code);
    }
    press(&mut state, KeyCode::Esc);
    assert!(state.selected_request().is_some());
}

#[test]
fn selected_request_is_visible_after_navigation_and_resize() {
    let supplied = PresentationSnapshot {
        requests: (0..75)
            .map(|i| request(1000 + i, &format!("row-{i:03}"), RequestStatus::Draft))
            .collect(),
        scope: SnapshotScope::Complete,
    };
    let mut state = AppState::from_snapshot(supplied);
    press(&mut state, KeyCode::Char('p'));
    state.active_tab = Tab::Requests;
    press(&mut state, KeyCode::End);
    assert_eq!(state.selected_request().unwrap().id, 1074);
    for (width, height) in [(120, 30), (80, 20), (40, 12), (100, 24)] {
        state.handle_event(Event::Resize(width, height));
        let rendered = render(&state, width, height);
        assert!(rendered
            .lines()
            .any(|line| line.contains('>') && line.contains("1074")));
        assert_eq!(state.selected_request().unwrap().id, 1074);
    }
    press(&mut state, KeyCode::PageUp);
    assert_ne!(state.selected_request().unwrap().id, 1074);
    press(&mut state, KeyCode::Home);
    assert_eq!(state.selected_request().unwrap().id, 1000);
    press(&mut state, KeyCode::PageDown);
    assert_ne!(state.selected_request().unwrap().id, 1000);
}

#[test]
fn timeline_navigation_keeps_the_selected_event_visible() {
    let mut supplied = snapshot();
    supplied.requests[0].timeline = TimelineState::Ready(
        (0..75)
            .map(|i| TimelineItem {
                occurred_at: None,
                kind: TimelineKind::Note,
                summary: format!("event-{i:03}"),
            })
            .collect(),
    );
    let mut state = AppState::from_snapshot(supplied);
    press(&mut state, KeyCode::Char('p'));
    state.active_tab = Tab::Requests;
    press(&mut state, KeyCode::Enter);
    press(&mut state, KeyCode::End);
    assert_eq!(state.selected_timeline_idx, 74);
    let rendered = render(&state, 80, 20);
    assert!(rendered
        .lines()
        .any(|line| line.contains('>') && line.contains("event-074")));
    press(&mut state, KeyCode::Char(']'));
    assert_eq!(state.selected_request().unwrap().id, 7102);
    assert_eq!(state.selected_timeline_idx, 0);
    press(&mut state, KeyCode::Char('['));
    assert_eq!(state.selected_request().unwrap().id, 7101);
    assert_eq!(state.selected_timeline_idx, 0);
}

#[test]
fn complete_request_and_event_text_can_be_read_at_narrow_widths() {
    let request_tail = "REQUEST-TAIL";
    let event_tail = "EVENT-TAIL";
    let mut supplied = snapshot();
    supplied.requests[0].title = format!("{}{request_tail}", "synthetic words ".repeat(28));
    supplied.requests[0].timeline = TimelineState::Ready(vec![TimelineItem {
        occurred_at: None,
        kind: TimelineKind::Note,
        summary: format!("{}{event_tail}", "synthetic event ".repeat(28)),
    }]);
    let mut state = AppState::from_snapshot(supplied);
    press(&mut state, KeyCode::Char('p'));
    state.active_tab = Tab::Requests;
    state.handle_event(Event::Resize(40, 12));
    press(&mut state, KeyCode::Char('d'));
    assert!(state.details_open);
    let mut reached_tail = false;
    for _ in 0..80 {
        reached_tail |= text_without_layout(&render(&state, 40, 12)).contains(request_tail);
        press(&mut state, KeyCode::Down);
    }
    assert!(reached_tail, "request title tail was inaccessible");
    press(&mut state, KeyCode::Esc);
    assert!(!state.details_open);
    assert!(!state.should_quit);
    press(&mut state, KeyCode::Enter);
    assert_eq!(state.active_tab, Tab::Timeline);
    press(&mut state, KeyCode::Enter);
    assert!(state.details_open);
    let mut reached_tail = false;
    for _ in 0..80 {
        reached_tail |= text_without_layout(&render(&state, 40, 12)).contains(event_tail);
        press(&mut state, KeyCode::Down);
    }
    assert!(reached_tail, "timeline summary tail was inaccessible");
}

#[test]
fn details_cannot_open_in_private_mode_and_hiding_closes_the_panel() {
    let mut state = populated();
    state.active_tab = Tab::Requests;
    press(&mut state, KeyCode::Char('d'));
    assert!(!state.details_open);
    press(&mut state, KeyCode::Char('p'));
    press(&mut state, KeyCode::Char('d'));
    assert!(state.details_open);
    assert!(render(&state, 80, 24).contains(PRIVATE_TITLE));
    press(&mut state, KeyCode::Char('p'));
    assert_eq!(state.privacy, PrivacyMode::Redacted);
    assert!(!state.details_open);
    assert_private(&render(&state, 80, 24));
}

#[test]
fn every_shortcut_preserves_the_supplied_data_and_emits_only_read_only_intents() {
    let codes = [
        KeyCode::Char('s'),
        KeyCode::Char('e'),
        KeyCode::Char('j'),
        KeyCode::Char('p'),
        KeyCode::Char('c'),
        KeyCode::Char('t'),
        KeyCode::Char('d'),
        KeyCode::Char('r'),
        KeyCode::Char('q'),
        KeyCode::Char('/'),
        KeyCode::Delete,
        KeyCode::Backspace,
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Tab,
        KeyCode::F(1),
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Home,
        KeyCode::End,
    ];
    for tab in [Tab::Summary, Tab::Requests, Tab::Timeline, Tab::Drafts] {
        for code in codes {
            for modifiers in [KeyModifiers::NONE, KeyModifiers::CONTROL, KeyModifiers::ALT] {
                let mut state = visible();
                state.active_tab = tab;
                let before = state.snapshot.clone();
                match state.handle_key_event(KeyEvent::new(code, modifiers)) {
                    TuiCommand::None | TuiCommand::Quit => {}
                }
                assert_eq!(state.snapshot, before);
                let before_draw = state.snapshot.clone();
                render(&state, 80, 24);
                assert_eq!(state.snapshot, before_draw);
            }
        }
    }
}

#[test]
fn terminal_resize_preserves_a_drawable_state() {
    let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
    let mut state = visible();
    for tab in [Tab::Summary, Tab::Requests, Tab::Timeline, Tab::Drafts] {
        state.active_tab = tab;
        for help in [false, true] {
            state.help_open = help;
            for (width, height) in [(120, 32), (40, 12), (1, 1), (0, 0), (35, 9), (80, 24)] {
                terminal.backend_mut().resize(width, height);
                state.handle_event(Event::Resize(width, height));
                terminal.draw(|frame| draw_ui(frame, &state)).unwrap();
                assert_eq!(terminal.backend().buffer().area.width, width);
                assert_eq!(terminal.backend().buffer().area.height, height);
            }
        }
    }
}

#[test]
fn summary_status_counts_remain_accessible_in_the_smallest_supported_terminal() {
    let mut state = visible();
    state.handle_event(Event::Resize(36, 10));
    let initial = render(&state, 36, 10);
    press(&mut state, KeyCode::End);
    let last = render(&state, 36, 10);
    assert_ne!(initial, last);
    assert!(last.contains("Unknown: 1"));
    press(&mut state, KeyCode::Home);
    assert_eq!(render(&state, 36, 10), initial);
}

#[test]
fn non_keyboard_events_do_not_import_or_mutate_data() {
    let mut state = populated();
    let original = state.snapshot.clone();
    for event in [
        Event::FocusGained,
        Event::FocusLost,
        Event::Paste("UNTRUSTED SYNTHETIC PASTE\u{1b}[31m".to_string()),
    ] {
        assert_eq!(state.handle_event(event), TuiCommand::None);
        assert_eq!(state.snapshot, original);
        assert!(state.search_query.is_empty());
        assert!(!render(&state, 80, 24).contains("UNTRUSTED"));
    }
}

#[test]
fn untrusted_unicode_text_is_safe_in_every_view() {
    let mut supplied = snapshot();
    let hostile = "水道 Élan 👩‍🔬\u{1b}[31m\u{7}\r\n\t\u{202e}suffix";
    supplied.requests[0].title = hostile.repeat(100);
    supplied.requests[0].authority = Some(hostile.to_string());
    supplied.requests[0].updated_at = Some(hostile.to_string());
    supplied.requests[0].timeline = TimelineState::Ready(vec![TimelineItem {
        occurred_at: Some(hostile.to_string()),
        kind: TimelineKind::Note,
        summary: hostile.repeat(100),
    }]);
    let mut state = AppState::from_snapshot(supplied);
    press(&mut state, KeyCode::Char('p'));
    for tab in [Tab::Summary, Tab::Requests, Tab::Timeline, Tab::Drafts] {
        state.active_tab = tab;
        for (width, height) in [(120, 24), (40, 12)] {
            let rendered = render(&state, width, height);
            for forbidden in ['\u{1b}', '\u{7}', '\r', '\t', '\u{202e}'] {
                assert!(!rendered.contains(forbidden));
            }
        }
    }
}

#[test]
fn representative_views_have_reviewable_complete_buffer_snapshots() {
    let mut state = visible();
    assert_snapshot("summary_visible_100x24", &render(&state, 100, 24));
    state.active_tab = Tab::Requests;
    assert_snapshot("requests_visible_120x24", &render(&state, 120, 24));
    assert_snapshot("requests_narrow_40x12", &render(&state, 40, 12));
    state.active_tab = Tab::Timeline;
    assert_snapshot("timeline_visible_120x24", &render(&state, 120, 24));
    state.next_tab();
    assert_eq!(state.active_tab, Tab::Drafts);
    assert_snapshot("drafts_visible_80x20", &render(&state, 80, 20));
    let mut private = populated();
    private.active_tab = Tab::Requests;
    assert_snapshot("requests_private_100x24", &render(&private, 100, 24));
    private.help_open = true;
    assert_snapshot("help_80x24", &render(&private, 80, 24));
    let mut empty = AppState::from_snapshot(PresentationSnapshot::default());
    press(&mut empty, KeyCode::Char('p'));
    assert_snapshot("empty_80x20", &render(&empty, 80, 20));
    let mut failed = visible();
    failed.set_error(PresentationError::Unavailable);
    assert_snapshot("error_80x20", &render(&failed, 80, 20));
}

#[test]
fn unavailable_empty_loading_and_failed_timelines_are_distinct() {
    let cases = [
        ("unavailable", TimelineState::Unavailable),
        ("empty", TimelineState::Ready(Vec::new())),
        ("loading", TimelineState::Loading),
        (
            "failed",
            TimelineState::Failed(PresentationError::Unavailable),
        ),
    ];
    let mut rendered_views = Vec::new();
    for (name, timeline) in cases {
        let mut supplied = snapshot();
        supplied.requests[0].timeline = timeline;
        let mut state = AppState::from_snapshot(supplied);
        press(&mut state, KeyCode::Char('p'));
        state.active_tab = Tab::Timeline;
        let rendered = render(&state, 80, 20);
        assert_snapshot(&format!("timeline_{name}_80x20"), &rendered);
        assert!(!rendered_views.contains(&rendered));
        rendered_views.push(rendered);
    }
}
