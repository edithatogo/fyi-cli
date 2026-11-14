//! Terminal lifecycle and asynchronous input for a one-time read-only local view.
use super::adapter::load_snapshot;
use super::contracts::{PresentationError, PresentationSnapshot};
use super::{draw_ui, AppState, TuiCommand};
use crate::OutputFormat;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::style::{Attribute, ResetColor, SetAttribute};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fmt;
use std::future::Future;
use std::io::{self, IsTerminal, Stdout};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::mpsc;

const INPUT_CAPACITY: usize = 32;

/// Closed categories prevent terminal, database and panic payloads reaching stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeError {
    Format,
    Interactive,
    UnsupportedTerminal,
    Terminal,
    Input,
    Runtime,
    Panicked,
    Data(PresentationError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Format => "The TUI requires text output.",
            Self::Interactive => "An interactive terminal is required.",
            Self::UnsupportedTerminal => "This terminal does not support the TUI.",
            Self::Terminal => "The terminal could not be initialized, drawn or restored.",
            Self::Input => "Terminal input is unavailable.",
            Self::Runtime => "The local view runtime is unavailable.",
            Self::Panicked => "The local view stopped unexpectedly.",
            Self::Data(error) => error.message(),
        })
    }
}

/// Run before global logging/profiling initialization so neither can expose view data
/// or produce files. Diagnostics are printed only after the terminal has been restored.
pub fn run(path: &Path, format: OutputFormat) {
    if let Err(error) = launch(path, format) {
        if format == OutputFormat::Json {
            eprintln!(
                "{}",
                serde_json::json!({"error": {"kind": "tui", "message": error.to_string()}})
            );
        } else {
            eprintln!("TUI failed: {error}");
        }
        std::process::exit(1);
    }
}

fn launch(path: &Path, format: OutputFormat) -> Result<(), RuntimeError> {
    if format != OutputFormat::Text {
        return Err(RuntimeError::Format);
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(RuntimeError::Interactive);
    }
    if std::env::var_os("TERM").is_some_and(|term| {
        term.to_str()
            .is_none_or(|value| value.is_empty() || value == "dumb")
    }) {
        return Err(RuntimeError::UnsupportedTerminal);
    }
    catch_runtime(|| execute(path))
}

fn catch_runtime(operation: impl FnOnce() -> Result<(), RuntimeError>) -> Result<(), RuntimeError> {
    // A default panic hook may disclose payloads or paths before unwinding. The CLI
    // owns this process; suppress it for the session, restore it after RAII cleanup,
    // and report only the static category. No presentation data is logged.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
    std::panic::set_hook(previous_hook);
    result.unwrap_or(Err(RuntimeError::Panicked))
}

fn execute(path: &Path) -> Result<(), RuntimeError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| RuntimeError::Runtime)?;
    let mut session = Session::start(CrosstermSurface { terminal: None })?;
    let mut input = InputWorker::start_with(|| {
        if event::poll(Duration::from_millis(50))? {
            event::read().map(Some)
        } else {
            Ok(None)
        }
    })?;
    let result = runtime.block_on(run_loop(
        &mut session,
        &mut input.receiver,
        load_snapshot(path),
        shutdown_signal(),
    ));
    let stopped = input.stop();
    let restored = session.restore();
    restored?;
    stopped?;
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    RawOn,
    Enter,
    Hide,
    Clear,
    ResetStyle,
    Show,
    Leave,
    RawOff,
}

trait Surface {
    fn effect(&mut self, effect: Effect) -> io::Result<()>;
    fn size(&self) -> io::Result<(u16, u16)>;
    fn draw(&mut self, app: &AppState) -> io::Result<()>;
}

struct CrosstermSurface {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
}

impl Surface for CrosstermSurface {
    fn effect(&mut self, effect: Effect) -> io::Result<()> {
        match effect {
            Effect::RawOn => terminal::enable_raw_mode(),
            Effect::Enter => execute!(io::stdout(), EnterAlternateScreen),
            Effect::Hide => execute!(io::stdout(), Hide),
            Effect::Clear => execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0)),
            Effect::ResetStyle => {
                execute!(io::stdout(), ResetColor, SetAttribute(Attribute::Reset))
            }
            Effect::Show => execute!(io::stdout(), Show),
            Effect::Leave => execute!(io::stdout(), LeaveAlternateScreen),
            Effect::RawOff => terminal::disable_raw_mode(),
        }
    }

    fn size(&self) -> io::Result<(u16, u16)> {
        terminal::size()
    }

    fn draw(&mut self, app: &AppState) -> io::Result<()> {
        if self.terminal.is_none() {
            self.terminal = Some(Terminal::new(CrosstermBackend::new(io::stdout()))?);
        }
        if let Some(terminal) = &mut self.terminal {
            terminal.draw(|frame| draw_ui(frame, app))?;
        }
        Ok(())
    }
}

/// Own effects from the first attempted startup operation, including partial failure.
struct Session<S: Surface> {
    surface: S,
    raw: bool,
    enter_attempted: bool,
    entered: bool,
    hide_attempted: bool,
    restored: bool,
}

impl<S: Surface> Session<S> {
    fn start(surface: S) -> Result<Self, RuntimeError> {
        let mut session = Self {
            surface,
            raw: true,
            enter_attempted: false,
            entered: false,
            hide_attempted: false,
            restored: false,
        };
        session
            .surface
            .effect(Effect::RawOn)
            .map_err(|_| RuntimeError::Terminal)?;
        session.enter_attempted = true;
        session
            .surface
            .effect(Effect::Enter)
            .map_err(|_| RuntimeError::Terminal)?;
        session.entered = true;
        session.hide_attempted = true;
        session
            .surface
            .effect(Effect::Hide)
            .map_err(|_| RuntimeError::Terminal)?;
        session
            .surface
            .effect(Effect::Clear)
            .map_err(|_| RuntimeError::Terminal)?;
        Ok(session)
    }

    fn restore(&mut self) -> Result<(), RuntimeError> {
        if self.restored {
            return Ok(());
        }
        self.restored = true;
        let mut failed = false;
        for (needed, effect) in [
            (self.entered, Effect::Clear),
            (self.entered, Effect::ResetStyle),
            (self.hide_attempted, Effect::Show),
            (self.enter_attempted, Effect::Leave),
            (self.raw, Effect::RawOff),
        ] {
            if needed && self.surface.effect(effect).is_err() {
                failed = true;
            }
        }
        if failed {
            Err(RuntimeError::Terminal)
        } else {
            Ok(())
        }
    }
}

impl<S: Surface> Drop for Session<S> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn draw<S: Surface>(session: &mut Session<S>, app: &mut AppState) -> Result<(), RuntimeError> {
    let (width, height) = session.surface.size().map_err(|_| RuntimeError::Terminal)?;
    app.handle_event(Event::Resize(width, height));
    session
        .surface
        .draw(app)
        .map_err(|_| RuntimeError::Terminal)
}

async fn run_loop<S, L, Q>(
    session: &mut Session<S>,
    events: &mut mpsc::Receiver<Result<Event, RuntimeError>>,
    load: L,
    shutdown: Q,
) -> Result<(), RuntimeError>
where
    S: Surface,
    L: Future<Output = Result<PresentationSnapshot, PresentationError>>,
    Q: Future<Output = Result<(), RuntimeError>>,
{
    let mut app = AppState::new();
    draw(session, &mut app)?;
    tokio::pin!(load, shutdown);
    let mut loaded = false;
    let mut data_error = None;
    loop {
        tokio::select! {
            signal = &mut shutdown => {
                signal?;
                return data_error.map_or(Ok(()), |error| Err(RuntimeError::Data(error)));
            }
            result = &mut load, if !loaded => {
                loaded = true;
                match result {
                    Ok(snapshot) => {
                        app.set_snapshot(snapshot);
                        if let super::LoadState::Failed(error) = app.load_state {
                            data_error = Some(error);
                        }
                    }
                    Err(error) => {
                        data_error = Some(error);
                        app.set_error(error);
                    }
                }
            }
            event = events.recv() => {
                let event = event.ok_or(RuntimeError::Input)??;
                if app.handle_event(event) == TuiCommand::Quit {
                    return data_error.map_or(Ok(()), |error| Err(RuntimeError::Data(error)));
                }
            }
        }
        draw(session, &mut app)?;
    }
}

/// Poll/read stay on one dedicated thread; a bounded queue provides backpressure.
/// Closing the receiver releases a full queue before joining the short-poll worker.
struct InputWorker {
    receiver: mpsc::Receiver<Result<Event, RuntimeError>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl InputWorker {
    fn start_with<F>(mut next: F) -> Result<Self, RuntimeError>
    where
        F: FnMut() -> io::Result<Option<Event>> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(INPUT_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("fyi-tui-input".to_owned())
            .spawn(move || {
                while !worker_stop.load(Ordering::Acquire) {
                    let result = match next() {
                        Ok(Some(event)) => Ok(event),
                        Ok(None) => continue,
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => Err(RuntimeError::Input),
                    };
                    let failed = result.is_err();
                    if sender.blocking_send(result).is_err() || failed {
                        break;
                    }
                }
            })
            .map_err(|_| RuntimeError::Input)?;
        Ok(Self {
            receiver,
            stop,
            thread: Some(thread),
        })
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        self.stop.store(true, Ordering::Release);
        self.receiver.close();
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| RuntimeError::Input)?;
        }
        Ok(())
    }
}

impl Drop for InputWorker {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

async fn shutdown_signal() -> Result<(), RuntimeError> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut interrupt = signal(SignalKind::interrupt()).map_err(|_| RuntimeError::Runtime)?;
        let mut terminate = signal(SignalKind::terminate()).map_err(|_| RuntimeError::Runtime)?;
        let mut hangup = signal(SignalKind::hangup()).map_err(|_| RuntimeError::Runtime)?;
        tokio::select! {
            _ = interrupt.recv() => {},
            _ = terminate.recv() => {},
            _ = hangup.recv() => {},
        }
        Ok(())
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .map_err(|_| RuntimeError::Runtime)
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
