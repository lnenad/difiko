//! Event loop and dispatch. The loop in `run_loop` selects over three
//! sources (terminal input, async git results, tick), then dispatches via
//! `handle_event`. Key handling lives in `input::dispatch_key` (pure) and
//! the resulting `KeyAction` is applied here via `actions::apply_action`.

mod actions;
mod git_results;
mod loaders;
mod modal;
mod mouse;
mod setup;

use crate::app::App;
use crate::input::dispatch_key;
use crate::model::{Commit, FileChange};
use crate::ui;
use anyhow::Result;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event as CtEvent, EventStream, KeyEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedSender};

/// Spinner tick / animation cadence. Drives no game logic — only the frame
/// rate of the "loading…" indicator while we're idle.
const TICK_INTERVAL_MS: u64 = 120;

pub enum AppEvent {
    Input(CtEvent),
    Git(GitResult),
    Tick,
}

pub struct GitResult {
    pub req_id: u64,
    pub kind: GitResultKind,
}

pub enum GitResultKind {
    Branches(Result<Vec<String>>),
    Diff(Result<Vec<FileChange>>),
    Commits(Result<Vec<Commit>>),
    CommitDiff {
        hash: String,
        result: Result<Vec<FileChange>>,
    },
    Blame {
        git_ref: String,
        file: String,
        result: Result<crate::git::Blame>,
    },
}

pub async fn run(mut app: App) -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    install_panic_hook();

    let result = run_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original(info);
    }));
}

async fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    let mut input_stream = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(TICK_INTERVAL_MS));

    setup::bootstrap(app, &tx);
    terminal.draw(|f| ui::render(f, app))?;

    loop {
        tokio::select! {
            maybe_input = input_stream.next() => {
                if let Some(Ok(ev)) = maybe_input {
                    if tx.send(AppEvent::Input(ev)).is_err() { break }
                }
            }
            _ = ticker.tick() => {
                if tx.send(AppEvent::Tick).is_err() { break }
            }
            ev = rx.recv() => {
                let Some(ev) = ev else { break };
                handle_event(app, ev, &tx);
                terminal.draw(|f| ui::render(f, app))?;
                if app.should_quit { break }
            }
        }
    }
    app.save_review();
    Ok(())
}

fn handle_event(app: &mut App, ev: AppEvent, tx: &UnboundedSender<AppEvent>) {
    mouse::expire_toasts(app);
    match ev {
        AppEvent::Tick => {
            app.spinner_tick = app.spinner_tick.wrapping_add(1);
        }
        AppEvent::Input(CtEvent::Key(key)) => {
            if key.kind == KeyEventKind::Release {
                return;
            }
            if let Some(action) = dispatch_key(app, key) {
                actions::apply_action(app, action, tx);
                loaders::ensure_blame_for_visible(app, tx);
            }
        }
        AppEvent::Input(CtEvent::Mouse(m)) => mouse::handle_mouse(app, m),
        AppEvent::Input(CtEvent::Resize(_, _)) => {}
        AppEvent::Input(_) => {}
        AppEvent::Git(result) => git_results::handle_git_result(app, result, tx),
    }
}
