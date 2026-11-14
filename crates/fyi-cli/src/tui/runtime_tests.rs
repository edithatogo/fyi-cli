use super::*;
use crate::tui::contracts::{RequestItem, RequestStatus, SnapshotScope, TimelineState};
use crate::tui::{LoadState, PrivacyMode, Tab};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::cell::RefCell;
use std::future::{pending, ready};
use std::rc::Rc;

#[derive(Default)]
struct Observation {
    effects: Vec<Effect>,
    frames: Vec<(LoadState, PrivacyMode, Tab)>,
    viewports: Vec<(u16, u16)>,
    size: (u16, u16),
    fail: Option<Effect>,
    fail_draw: bool,
    fail_size: bool,
    panic_draw: bool,
}

struct FakeTerminal(Rc<RefCell<Observation>>);

impl Surface for FakeTerminal {
    fn effect(&mut self, effect: Effect) -> std::io::Result<()> {
        let mut state = self.0.borrow_mut();
        state.effects.push(effect);
        if state.fail == Some(effect) {
            return Err(std::io::Error::other("PRIVATE-TERMINAL-ERROR"));
        }
        Ok(())
    }

    fn size(&self) -> std::io::Result<(u16, u16)> {
        if self.0.borrow().fail_size {
            Err(std::io::Error::other("PRIVATE-SIZE-ERROR"))
        } else {
            Ok(self.0.borrow().size)
        }
    }

    fn draw(&mut self, app: &AppState) -> std::io::Result<()> {
        let mut state = self.0.borrow_mut();
        assert!(!state.panic_draw, "PRIVATE-PANIC-PAYLOAD");
        if state.fail_draw {
            return Err(std::io::Error::other("PRIVATE-DRAW-ERROR"));
        }
        state
            .frames
            .push((app.load_state, app.privacy, app.active_tab));
        state.viewports.push(app.viewport);
        Ok(())
    }
}

fn terminal() -> (FakeTerminal, Rc<RefCell<Observation>>) {
    let state = Rc::new(RefCell::new(Observation::default()));
    state.borrow_mut().size = (40, 12);
    (FakeTerminal(Rc::clone(&state)), state)
}

fn key(code: KeyCode) -> Result<Event, RuntimeError> {
    Ok(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
}

fn snapshot() -> PresentationSnapshot {
    PresentationSnapshot {
        requests: vec![RequestItem {
            id: 42,
            authority: Some("Maple Council".to_owned()),
            title: "Synthetic records".to_owned(),
            status: RequestStatus::Draft,
            needs_attention: None,
            updated_at: None,
            timeline: TimelineState::Unavailable,
        }],
        scope: SnapshotScope::Complete,
    }
}

#[test]
fn startup_failures_restore_every_effect_that_might_have_started() {
    for failure in [Effect::RawOn, Effect::Enter, Effect::Hide, Effect::Clear] {
        let (terminal, state) = terminal();
        state.borrow_mut().fail = Some(failure);
        assert!(Session::start(terminal).is_err());
        let observed = &state.borrow().effects;
        assert_eq!(observed.last(), Some(&Effect::RawOff));
        if failure != Effect::RawOn {
            assert!(observed.contains(&Effect::Leave));
        }
        if matches!(failure, Effect::Hide | Effect::Clear) {
            assert!(observed.contains(&Effect::Show));
        }
    }
}

#[test]
fn cleanup_attempts_all_steps_even_if_one_fails_and_is_idempotent() {
    for failure in [
        Effect::Clear,
        Effect::ResetStyle,
        Effect::Show,
        Effect::Leave,
        Effect::RawOff,
    ] {
        let (terminal, state) = terminal();
        let mut session = Session::start(terminal).unwrap();
        state.borrow_mut().effects.clear();
        state.borrow_mut().fail = Some(failure);
        assert!(session.restore().is_err());
        assert_eq!(
            state.borrow().effects,
            vec![
                Effect::Clear,
                Effect::ResetStyle,
                Effect::Show,
                Effect::Leave,
                Effect::RawOff
            ]
        );
        drop(session);
        assert_eq!(state.borrow().effects.len(), 5);
    }
}

#[tokio::test]
async fn quit_and_shutdown_are_responsive_while_data_loading_is_pending() {
    for shutdown in [false, true] {
        let (terminal, state) = terminal();
        let mut session = Session::start(terminal).unwrap();
        let (sender, mut receiver) = mpsc::channel(1);
        if !shutdown {
            sender.send(key(KeyCode::Char('q'))).await.unwrap();
        }
        let signal = async move {
            if !shutdown {
                pending::<()>().await;
            }
            Ok(())
        };
        tokio::time::timeout(
            Duration::from_secs(1),
            run_loop(&mut session, &mut receiver, pending(), signal),
        )
        .await
        .expect("input must not wait for data")
        .unwrap();
        assert_eq!(state.borrow().frames[0].0, LoadState::Loading);
        assert_eq!(
            state.borrow().frames.len(),
            1,
            "no periodic redraw while idle"
        );
        assert_eq!(state.borrow().viewports[0], (40, 12));
        drop(session);
        assert_eq!(state.borrow().effects.last(), Some(&Effect::RawOff));
    }
}

#[tokio::test]
async fn loaded_data_starts_private_and_input_and_resize_remain_live() {
    let (terminal, state) = terminal();
    let mut session = Session::start(terminal).unwrap();
    let (sender, mut receiver) = mpsc::channel(8);
    let input = async {
        while state.borrow().frames.len() < 2 {
            tokio::task::yield_now().await;
        }
        state.borrow_mut().size = (36, 10);
        for event in [
            key(KeyCode::Tab),
            key(KeyCode::Char('p')),
            Ok(Event::Resize(36, 10)),
            key(KeyCode::Char('q')),
        ] {
            sender.send(event).await.unwrap();
        }
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(
            run_loop(
                &mut session,
                &mut receiver,
                ready(Ok(snapshot())),
                pending()
            ),
            input
        )
    })
    .await
    .expect("data and input must both progress");
    result.unwrap();
    let frames = &state.borrow().frames;
    assert_eq!(
        frames[1],
        (LoadState::Ready, PrivacyMode::Redacted, Tab::Summary)
    );
    assert!(frames.contains(&(LoadState::Ready, PrivacyMode::Visible, Tab::Requests)));
    assert!(state.borrow().viewports.contains(&(36, 10)));
}

#[tokio::test]
async fn failure_paths_have_static_errors_and_restore_the_terminal() {
    for failure in ["draw", "size", "input", "closed", "data", "invalid"] {
        let (terminal, state) = terminal();
        state.borrow_mut().fail_draw = failure == "draw";
        state.borrow_mut().fail_size = failure == "size";
        let mut session = Session::start(terminal).unwrap();
        let (sender, mut receiver) = mpsc::channel(1);
        let expected = match failure {
            "draw" | "size" => RuntimeError::Terminal,
            "data" => RuntimeError::Data(PresentationError::Unavailable),
            "invalid" => RuntimeError::Data(PresentationError::InvalidSnapshot),
            _ => RuntimeError::Input,
        };
        let data = async {
            if failure == "data" {
                Err(PresentationError::Unavailable)
            } else if failure == "invalid" {
                let mut invalid = snapshot();
                invalid.requests.push(invalid.requests[0].clone());
                Ok(invalid)
            } else {
                pending().await
            }
        };
        let input = async {
            if matches!(failure, "data" | "invalid") {
                while state.borrow().frames.len() < 2 {
                    tokio::task::yield_now().await;
                }
                sender.send(key(KeyCode::Char('q'))).await.unwrap();
            } else if failure == "input" {
                sender.send(Err(RuntimeError::Input)).await.unwrap();
            }
            drop(sender);
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(
                run_loop(&mut session, &mut receiver, data, pending()),
                input
            )
        })
        .await
        .expect("failure view must remain responsive");
        assert_eq!(result, Err(expected));
        assert!(!expected.to_string().contains("PRIVATE"));
        drop(session);
        assert_eq!(state.borrow().effects.last(), Some(&Effect::RawOff));
    }
}

#[test]
fn unwinding_a_draw_panic_restores_the_terminal() {
    let (terminal, state) = terminal();
    state.borrow_mut().panic_draw = true;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut session = Session::start(terminal).unwrap();
        let _ = session.surface.draw(&AppState::new());
    }));
    assert!(result.is_err());
    assert_eq!(state.borrow().effects.last(), Some(&Effect::RawOff));
}

#[test]
fn input_worker_stops_with_a_full_queue_and_converts_diagnostics() {
    let mut worker = InputWorker::start_with(|| Ok(Some(Event::FocusGained))).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while worker.receiver.len() < INPUT_CAPACITY {
        assert!(std::time::Instant::now() < deadline, "input worker stalled");
        std::thread::yield_now();
    }
    worker.stop().unwrap();
    let mut failed =
        InputWorker::start_with(|| Err(std::io::Error::other("PRIVATE-INPUT"))).unwrap();
    assert_eq!(
        failed.receiver.blocking_recv(),
        Some(Err(RuntimeError::Input))
    );
    failed.stop().unwrap();
}

#[test]
fn interrupted_poll_is_retried_without_exposing_a_signal_as_an_input_failure() {
    let mut first = true;
    let mut worker = InputWorker::start_with(move || {
        if std::mem::take(&mut first) {
            Err(std::io::Error::from(std::io::ErrorKind::Interrupted))
        } else {
            Ok(Some(Event::FocusGained))
        }
    })
    .unwrap();
    assert_eq!(worker.receiver.blocking_recv(), Some(Ok(Event::FocusGained)));
    worker.stop().unwrap();
}

#[test]
fn private_panic_worker() {
    if std::env::var_os("FYI_TUI_PANIC_WORKER").is_none() {
        return;
    }
    std::panic::set_hook(Box::new(|_| eprintln!("restored-test-hook")));
    let result = catch_runtime(|| panic!("PRIVATE-PANIC-PAYLOAD"));
    assert_eq!(result, Err(RuntimeError::Panicked));
    eprintln!("{}", result.unwrap_err());
    let _ = std::panic::catch_unwind(|| panic!("control after hook restoration"));
}

#[test]
fn caught_panic_payloads_and_backtraces_are_not_written_to_the_terminal() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tui::runtime::tests::private_panic_worker",
            "--nocapture",
        ])
        .env("FYI_TUI_PANIC_WORKER", "1")
        .env("RUST_BACKTRACE", "full")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("The local view stopped unexpectedly."));
    assert_eq!(stderr.matches("restored-test-hook").count(), 1);
    assert!(!stderr.contains("PRIVATE"));
    assert!(!stderr.contains("stack backtrace"));
}
