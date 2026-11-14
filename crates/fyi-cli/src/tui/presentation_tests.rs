use super::*;
use crossterm::event::KeyEventKind;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn production_state_does_not_invent_requests_or_metrics() {
    let state = AppState::new();
    assert!(state.tracked_requests.is_empty());
    assert_eq!(state.total_tracked, 0);
    assert!(state.logs.is_empty());
}

#[test]
fn key_release_does_not_activate_a_shortcut() {
    let mut state = AppState::new();
    let command = state.handle_key_event(KeyEvent::new_with_kind(
        KeyCode::Char('q'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ));
    assert_eq!(command, TuiCommand::None);
    assert!(!state.should_quit);
}

#[test]
fn control_c_can_exit_from_an_overlay() {
    let mut state = AppState::new();
    state.help_open = true;
    assert_eq!(
        state.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        TuiCommand::Quit
    );
    assert!(state.should_quit);
}

#[test]
fn terminal_resize_preserves_a_drawable_state() {
    let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
    let state = AppState::new();
    for (width, height) in [(120, 32), (40, 12), (1, 1), (0, 0), (80, 24)] {
        terminal.backend_mut().resize(width, height);
        terminal.draw(|frame| draw_ui(frame, &state)).unwrap();
    }
}
