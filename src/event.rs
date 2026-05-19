use crate::app::{App, BranchSlot, FocusedPane, Modal, Screen, ToastKind};
use crate::input::{dispatch_key, KeyAction};
use crate::model::{Commit, FileChange};
use crate::tasks;
use crate::ui;
use anyhow::Result;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event as CtEvent, EventStream, KeyEventKind,
    MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedSender};

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
    let mut ticker = tokio::time::interval(Duration::from_millis(120));

    bootstrap(app, &tx);
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

fn bootstrap(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    if let Some(repo) = app.repo_path.clone() {
        // Validate it looks plausible by checking .git exists; full validation comes from git command.
        if repo.join(".git").exists() || repo.join(".git").is_file() {
            kick_off_load_branches(app, tx);

            // If --base was provided but --compare wasn't, fill compare with
            // the active HEAD branch (when distinct from base) so we can dive
            // straight into the diff. If HEAD == base it'd be a self-diff —
            // leave compare unset and focus the compare picker so the user
            // picks one quickly.
            if app.base_branch.is_some() && app.compare_branch.is_none() {
                if let Some(head) = head_branch(app) {
                    if Some(head.as_str()) != app.base_branch.as_deref() {
                        app.compare_branch = Some(head);
                    }
                }
            }

            if app.base_branch.is_some() && app.compare_branch.is_some() {
                // Both refs known — fire diff/commits and stay on Setup until
                // the Diff handler flips us to Review (avoids a "no files"
                // flash).
                kick_off_load_diff(app, tx);
                kick_off_load_commits(app, tx);
            } else if app.base_branch.is_some() {
                // --base passed but compare can't be inferred (HEAD == base
                // or undetectable). Land focus on Compare for one-keystroke
                // selection.
                app.setup_field = crate::app::SetupField::Compare;
            } else {
                // Default: skip the path field, land on Base.
                app.setup_field = crate::app::SetupField::Base;
            }
        }
    }
}

fn handle_event(app: &mut App, ev: AppEvent, tx: &UnboundedSender<AppEvent>) {
    expire_toasts(app);
    match ev {
        AppEvent::Tick => {
            app.spinner_tick = app.spinner_tick.wrapping_add(1);
        }
        AppEvent::Input(CtEvent::Key(key)) => {
            if key.kind == KeyEventKind::Release {
                return;
            }
            if let Some(action) = dispatch_key(app, key) {
                apply_action(app, action, tx);
                ensure_blame_for_visible(app, tx);
            }
        }
        AppEvent::Input(CtEvent::Mouse(m)) => handle_mouse(app, m),
        AppEvent::Input(CtEvent::Resize(_, _)) => {}
        AppEvent::Input(_) => {}
        AppEvent::Git(result) => handle_git_result(app, result, tx),
    }
}

fn handle_mouse(app: &mut App, ev: MouseEvent) {
    match ev.kind {
        MouseEventKind::ScrollDown => match app.focused {
            FocusedPane::Diff => scroll_diff(app, 3),
            FocusedPane::Sidebar => move_sidebar(app, 3),
            FocusedPane::Commits => move_commits(app, 1),
        },
        MouseEventKind::ScrollUp => match app.focused {
            FocusedPane::Diff => scroll_diff(app, -3),
            FocusedPane::Sidebar => move_sidebar(app, -3),
            FocusedPane::Commits => move_commits(app, -1),
        },
        _ => {}
    }
}

fn move_sidebar(app: &mut App, by: i32) {
    let len = match app.sidebar_mode {
        crate::app::SidebarMode::Flat => app.files.len(),
        crate::app::SidebarMode::Tree => app.tree_rows.len(),
    };
    if len == 0 {
        return;
    }
    let new = (app.sidebar_selected as i32 + by).clamp(0, len as i32 - 1);
    app.sidebar_selected = new as usize;
}

fn move_commits(app: &mut App, by: i32) {
    if app.commits.is_empty() {
        return;
    }
    let new = (app.commits_selected as i32 + by).clamp(0, app.commits.len() as i32 - 1);
    app.commits_selected = new as usize;
}

fn scroll_diff(app: &mut App, by: i32) {
    let path = match app.screen {
        Screen::Fullscreen => app.files.get(app.fullscreen_idx).map(|f| f.path.clone()),
        _ => app.current_file().map(|f| f.path.clone()),
    };
    let Some(key) = path else { return };
    let cur = *app.diff_scroll.get(&key).unwrap_or(&0u16);
    let new = (cur as i32 + by).max(0) as u16;
    app.diff_scroll.insert(key, new);
}

fn scroll_diff_h(app: &mut App, by: i32) {
    let path = match app.screen {
        Screen::Fullscreen => app.files.get(app.fullscreen_idx).map(|f| f.path.clone()),
        _ => app.current_file().map(|f| f.path.clone()),
    };
    let Some(key) = path else { return };
    let cur = *app.diff_scroll_h.get(&key).unwrap_or(&0u16);
    let new = (cur as i32 + by).max(0) as u16;
    app.diff_scroll_h.insert(key, new);
}

fn expire_toasts(app: &mut App) {
    let now = Instant::now();
    while let Some(t) = app.toasts.front() {
        if now.duration_since(t.created) > Duration::from_secs(4) {
            app.toasts.pop_front();
        } else {
            break;
        }
    }
}

fn handle_git_result(app: &mut App, result: GitResult, tx: &UnboundedSender<AppEvent>) {
    match result.kind {
        GitResultKind::Branches(res) => {
            if result.req_id != app.req_ids.branches {
                return;
            }
            app.pending.branches = false;
            match res {
                Ok(b) => {
                    app.branches = b;
                    if app.base_branch.is_none() {
                        app.base_branch = app
                            .branches
                            .iter()
                            .find(|b| *b == "main" || *b == "master")
                            .cloned()
                            .or_else(|| app.branches.first().cloned());
                    }
                    if app.compare_branch.is_none() {
                        if let Some(head) = head_branch(app) {
                            // Avoid base == compare; let the user pick compare
                            // explicitly when HEAD already is the base branch.
                            if app.base_branch.as_deref() != Some(head.as_str()) {
                                app.compare_branch = Some(head);
                            }
                        }
                    }
                }
                Err(e) => {
                    app.toast(format!("Failed to load branches: {e}"), ToastKind::Error);
                }
            }
        }
        GitResultKind::Diff(res) => {
            if result.req_id != app.req_ids.diff {
                return;
            }
            app.pending.diff = false;
            match res {
                Ok(files) => {
                    app.files = files;
                    app.all_files_backup = Some(app.files.clone());
                    app.syntax_cache.borrow_mut().clear();
                    app.rebuild_tree();
                    app.sidebar_selected = 0;
                    app.fullscreen_idx = 0;
                    app.load_review();
                    app.screen = Screen::Review;
                    if let Some(target) = app.initial_file.take() {
                        app.select_file_by_path(&target);
                        if let Some(idx) = app.current_file_index() {
                            app.fullscreen_idx = idx;
                        }
                        if app.initial_fullscreen {
                            app.screen = Screen::Fullscreen;
                            app.initial_fullscreen = false;
                        }
                    }
                    ensure_blame_for_visible(app, tx);
                }
                Err(e) => {
                    app.toast(format!("Failed to load diff: {e}"), ToastKind::Error);
                }
            }
        }
        GitResultKind::Commits(res) => {
            if result.req_id != app.req_ids.commits {
                return;
            }
            app.pending.commits = false;
            match res {
                Ok(c) => app.commits = c,
                Err(e) => app.toast(format!("Failed to load commits: {e}"), ToastKind::Error),
            }
        }
        GitResultKind::Blame {
            git_ref,
            file,
            result: res,
        } => {
            let key = (git_ref, file);
            app.blame_pending.remove(&key);
            match res {
                Ok(b) => {
                    app.blame_cache.insert(key, b);
                }
                Err(_e) => {
                    // Silently ignore — blame is best-effort.
                }
            }
        }
        GitResultKind::CommitDiff { hash, result: res } => {
            if app.pending.commit_diff.as_deref() != Some(hash.as_str())
                || result.req_id != app.req_ids.commit_diff
            {
                return;
            }
            app.pending.commit_diff = None;
            match res {
                Ok(files) => {
                    app.commit_diff_cache.insert(hash.clone(), files.clone());
                    if app.all_files_backup.is_none() {
                        app.all_files_backup = Some(app.files.clone());
                    }
                    app.files = files;
                    app.rebuild_tree();
                    app.sidebar_selected = 0;
                    app.fullscreen_idx = 0;
                }
                Err(e) => {
                    app.selected_commit = None;
                    app.toast(format!("Failed to load commit diff: {e}"), ToastKind::Error);
                }
            }
        }
    }
}

fn head_branch(app: &App) -> Option<String> {
    let repo = app.repo_path.as_ref()?;
    // For a normal repo, .git is a directory containing HEAD. For a linked
    // worktree, .git is a file ("gitdir: <path>") pointing at
    // <main>/worktrees/<name>, and HEAD lives there.
    let dot_git = repo.join(".git");
    let head_dir = if dot_git.is_file() {
        let content = std::fs::read_to_string(&dot_git).ok()?;
        let gitdir = content
            .lines()
            .find_map(|l| l.strip_prefix("gitdir: "))?
            .trim()
            .to_string();
        let path = std::path::PathBuf::from(&gitdir);
        if path.is_absolute() {
            path
        } else {
            repo.join(&gitdir)
        }
    } else {
        dot_git
    };
    let txt = std::fs::read_to_string(head_dir.join("HEAD")).ok()?;
    let s = txt.trim().strip_prefix("ref: refs/heads/")?;
    Some(s.to_string())
}

fn apply_action(app: &mut App, action: KeyAction, tx: &UnboundedSender<AppEvent>) {
    use KeyAction::*;
    match action {
        Quit => app.should_quit = true,
        FocusNextPane => app.focus_next_pane(),
        FocusPrevPane => app.focus_prev_pane(),
        FocusSidebar => app.focused = FocusedPane::Sidebar,
        FocusDiff => app.focused = FocusedPane::Diff,
        FocusCommits => {
            if app.commits_panel_visible {
                app.focused = FocusedPane::Commits;
            }
        }
        ToggleHelp => {
            app.modal = if matches!(app.modal, Some(Modal::HelpOverlay)) {
                None
            } else {
                Some(Modal::HelpOverlay)
            }
        }
        OpenCommandPalette => {
            let items = vec![
                "branches".into(),
                "reload".into(),
                "tree".into(),
                "flat".into(),
                "fullscreen".into(),
                "clear-reviewed".into(),
                "toggle-commits".into(),
                "split-diff".into(),
                "unified-diff".into(),
                "toggle-word-diff".into(),
                "toggle-syntax".into(),
                "quit".into(),
            ];
            app.modal = Some(Modal::CommandPalette {
                picker: crate::app::Picker::new(items),
            });
        }
        OpenFileFilter => {
            let items: Vec<String> = app.files.iter().map(|f| f.path.clone()).collect();
            app.modal = Some(Modal::FileFilter {
                picker: crate::app::Picker::new(items),
            });
        }
        OpenBranchPicker(slot) => {
            let current = match slot {
                BranchSlot::Base => app.base_branch.as_deref(),
                BranchSlot::Compare => app.compare_branch.as_deref(),
            };
            let items = picker_items_for(app, slot);
            app.modal = Some(Modal::BranchPicker {
                which: slot,
                picker: crate::app::Picker::with_selected(items, current),
            });
        }
        ToggleSidebarMode => {
            app.sidebar_mode = match app.sidebar_mode {
                crate::app::SidebarMode::Flat => crate::app::SidebarMode::Tree,
                crate::app::SidebarMode::Tree => crate::app::SidebarMode::Flat,
            };
            app.sidebar_selected = 0;
            app.rebuild_tree();
        }
        ToggleCommitsPanel => {
            app.commits_panel_visible = !app.commits_panel_visible;
            if !app.commits_panel_visible && matches!(app.focused, FocusedPane::Commits) {
                app.focused = FocusedPane::Diff;
            }
        }
        ToggleDiffMode => {
            app.diff_mode = match app.diff_mode {
                crate::app::DiffMode::Unified => crate::app::DiffMode::Split,
                crate::app::DiffMode::Split => crate::app::DiffMode::Unified,
            };
        }
        ToggleBlame => {
            app.blame_enabled = !app.blame_enabled;
            if app.blame_enabled {
                ensure_blame_for_visible(app, tx);
            }
        }
        ToggleWordDiff => {
            app.toggle_word_diff();
            let on = app.config.word_diff;
            app.toast(
                format!("Word diff: {}", if on { "on" } else { "off" }),
                ToastKind::Info,
            );
        }
        ToggleSyntaxHighlight => {
            app.toggle_syntax_highlight();
            let on = app.config.syntax_highlight;
            app.toast(
                format!("Syntax highlight: {}", if on { "on" } else { "off" }),
                ToastKind::Info,
            );
        }
        EnterFullscreen => {
            if let Some(idx) = app.current_file_index() {
                app.fullscreen_idx = idx;
                app.screen = Screen::Fullscreen;
            }
        }
        ExitFullscreen => {
            app.screen = Screen::Review;
        }
        FullscreenNext => {
            if app.files.is_empty() {
                return;
            }
            app.fullscreen_idx = (app.fullscreen_idx + 1) % app.files.len();
        }
        FullscreenPrev => {
            if app.files.is_empty() {
                return;
            }
            if app.fullscreen_idx == 0 {
                app.fullscreen_idx = app.files.len() - 1;
            } else {
                app.fullscreen_idx -= 1;
            }
        }
        ToggleReviewed => {
            let path = match app.screen {
                Screen::Fullscreen => app.files.get(app.fullscreen_idx).map(|f| f.path.clone()),
                _ => app.current_file().map(|f| f.path.clone()),
            };
            if let Some(p) = path {
                if app.reviewed.contains(&p) {
                    app.reviewed.remove(&p);
                } else {
                    app.reviewed.insert(p);
                }
                app.save_review();
            }
        }
        ClearReviewed => {
            let confirm_window = Duration::from_secs(2);
            let now = Instant::now();
            let confirmed = app
                .pending_clear_reviewed
                .map(|t| now.duration_since(t) <= confirm_window)
                .unwrap_or(false);
            if confirmed {
                app.pending_clear_reviewed = None;
                app.reviewed.clear();
                app.save_review();
                app.toast("Cleared reviewed set", ToastKind::Info);
            } else {
                app.pending_clear_reviewed = Some(now);
                app.toast("Press R again within 2s to clear reviewed", ToastKind::Info);
            }
        }
        ReloadDiff => {
            kick_off_load_diff(app, tx);
            kick_off_load_commits(app, tx);
        }
        ScrollDiff(delta) => {
            scroll_diff(app, delta);
        }
        ScrollDiffH(delta) => {
            scroll_diff_h(app, delta);
        }
        ScrollDiffTop => {
            let path = match app.screen {
                Screen::Fullscreen => app.files.get(app.fullscreen_idx).map(|f| f.path.clone()),
                _ => app.current_file().map(|f| f.path.clone()),
            };
            if let Some(key) = path {
                app.diff_scroll.insert(key, 0);
            }
        }
        ScrollDiffBottom => {
            let path = match app.screen {
                Screen::Fullscreen => app.files.get(app.fullscreen_idx).map(|f| f.path.clone()),
                _ => app.current_file().map(|f| f.path.clone()),
            };
            if let Some(key) = path {
                app.diff_scroll.insert(key, u16::MAX);
            }
        }
        SidebarMove(delta) => {
            move_sidebar(app, delta);
        }
        SidebarTop => {
            app.sidebar_selected = 0;
        }
        SidebarBottom => {
            let len = match app.sidebar_mode {
                crate::app::SidebarMode::Flat => app.files.len(),
                crate::app::SidebarMode::Tree => app.tree_rows.len(),
            };
            if len > 0 {
                app.sidebar_selected = len - 1;
            }
        }
        SidebarOpenFile => {
            app.focused = FocusedPane::Diff;
            if let Some(idx) = app.current_file_index() {
                app.fullscreen_idx = idx;
            }
        }
        SidebarToggleFolder => {
            if let Some(crate::tree::TreeRow::Dir { path, .. }) =
                app.tree_rows.get(app.sidebar_selected).cloned()
            {
                if app.sidebar_collapsed.contains(&path) {
                    app.sidebar_collapsed.remove(&path);
                } else {
                    app.sidebar_collapsed.insert(path);
                }
                app.rebuild_tree();
            }
        }
        NextFile => {
            if app.files.is_empty() {
                return;
            }
            let idx = app.current_file_index().unwrap_or(0);
            let next = (idx + 1).min(app.files.len() - 1);
            let path = app.files[next].path.clone();
            app.select_file_by_path(&path);
            app.fullscreen_idx = next;
        }
        PrevFile => {
            if app.files.is_empty() {
                return;
            }
            let idx = app.current_file_index().unwrap_or(0);
            let prev = idx.saturating_sub(1);
            let path = app.files[prev].path.clone();
            app.select_file_by_path(&path);
            app.fullscreen_idx = prev;
        }
        CommitsMove(delta) => move_commits(app, delta),
        CommitsToggleSelect => {
            let Some(c) = app.commits.get(app.commits_selected).cloned() else {
                return;
            };
            if app.selected_commit.as_deref() == Some(c.hash.as_str()) {
                app.selected_commit = None;
                if let Some(backup) = app.all_files_backup.clone() {
                    app.files = backup;
                }
                app.rebuild_tree();
                app.sidebar_selected = 0;
            } else {
                app.selected_commit = Some(c.hash.clone());
                if let Some(cached) = app.commit_diff_cache.get(&c.hash).cloned() {
                    app.files = cached;
                    app.rebuild_tree();
                    app.sidebar_selected = 0;
                } else if let Some(repo) = app.repo_path.clone() {
                    app.req_ids.commit_diff += 1;
                    app.pending.commit_diff = Some(c.hash.clone());
                    tasks::spawn_load_commit_diff(
                        tx.clone(),
                        app.req_ids.commit_diff,
                        repo,
                        c.hash,
                    );
                }
            }
        }
        CommitsToggleExpand => {
            let Some(c) = app.commits.get(app.commits_selected) else {
                return;
            };
            if app.expanded_commit.as_deref() == Some(c.hash.as_str()) {
                app.expanded_commit = None;
            } else {
                app.expanded_commit = Some(c.hash.clone());
            }
        }
        SetupNextField => app.setup_field = next_field(app.setup_field),
        SetupPrevField => app.setup_field = prev_field(app.setup_field),
        SetupToggleRemote => {
            app.include_remote_branches = !app.include_remote_branches;
            kick_off_load_branches(app, tx);
        }
        SetupSubmit => {
            handle_setup_submit(app, tx);
        }
        BackToSetupSoft => {
            // Drop any in-flight diff/commit loads so an arriving result
            // can't auto-flip the user back to Review after the soft-back.
            // The user can re-fire a load by accepting a new branch in the
            // picker (or pressing Submit on the Submit field).
            if app.pending.diff {
                app.req_ids.diff += 1;
                app.pending.diff = false;
            }
            if app.pending.commits {
                app.req_ids.commits += 1;
                app.pending.commits = false;
            }
            if app.pending.commit_diff.is_some() {
                app.req_ids.commit_diff += 1;
                app.pending.commit_diff = None;
            }
            app.screen = crate::app::Screen::Setup;
            app.setup_field = crate::app::SetupField::Compare;
            app.focused = crate::app::FocusedPane::Sidebar;
            app.modal = None;
        }
        SetupReset => {
            app.save_review();
            app.base_branch = None;
            app.compare_branch = None;
            app.branches.clear();
            app.files.clear();
            app.commits.clear();
            app.commit_diff_cache.clear();
            app.all_files_backup = None;
            app.selected_commit = None;
            app.reviewed.clear();
            app.diff_scroll.clear();
            app.tree_rows.clear();
            app.sidebar_selected = 0;
            app.fullscreen_idx = 0;
            app.pending.branches = false;
            app.pending.diff = false;
            app.pending.commits = false;
            app.pending.commit_diff = None;
            // Bump req IDs so any in-flight git results from the previous session are dropped.
            app.req_ids.branches += 1;
            app.req_ids.diff += 1;
            app.req_ids.commits += 1;
            app.req_ids.commit_diff += 1;
            app.screen = crate::app::Screen::Setup;
            app.setup_field = crate::app::SetupField::Repo;
            app.focused = crate::app::FocusedPane::Sidebar;
            app.modal = None;
            app.update_repo_completions();
        }
        SetupTextInput(c) => {
            if matches!(app.setup_field, crate::app::SetupField::Repo) {
                app.repo_input.insert(c);
                app.repo_dropdown_hidden = false;
                app.update_repo_completions();
            }
        }
        SetupBackspace => {
            if matches!(app.setup_field, crate::app::SetupField::Repo) {
                app.repo_input.backspace();
                app.repo_dropdown_hidden = false;
                app.update_repo_completions();
            }
        }
        RepoCompleteAccept => app.accept_repo_completion(),
        RepoCompleteCyclePrev => {
            app.repo_dropdown_hidden = false;
            app.cycle_repo_completion(-1);
        }
        RepoCompleteCycleNext => {
            app.repo_dropdown_hidden = false;
            app.cycle_repo_completion(1);
        }
        SetupCursorLeft => {
            if matches!(app.setup_field, crate::app::SetupField::Repo) {
                app.repo_input.move_left();
            }
        }
        SetupCursorRight => {
            if matches!(app.setup_field, crate::app::SetupField::Repo) {
                app.repo_input.move_right();
            }
        }
        ModalClose => {
            app.modal = None;
        }
        ModalAccept => {
            let modal = app.modal.take();
            if let Some(m) = modal {
                handle_modal_accept(app, m, tx);
            }
        }
        ModalMoveUp => modal_move(app, -1),
        ModalMoveDown => modal_move(app, 1),
        ModalInputChar(c) => modal_input(app, ModalInputOp::Insert(c)),
        ModalInputBackspace => modal_input(app, ModalInputOp::Backspace),

        DiffSearchOpen => {
            if app.diff_search.is_none() {
                app.diff_search = Some(crate::app::DiffSearch::default());
            }
            app.focused = FocusedPane::Diff;
        }
        DiffSearchClose => {
            app.diff_search = None;
        }
        DiffSearchInput(c) => {
            if let Some(search) = app.diff_search.as_mut() {
                search.query.insert(c);
            }
            app.recompute_diff_search();
            app.scroll_to_current_match();
        }
        DiffSearchBackspace => {
            if let Some(search) = app.diff_search.as_mut() {
                search.query.backspace();
            }
            app.recompute_diff_search();
            app.scroll_to_current_match();
        }
        DiffSearchNext => {
            if let Some(search) = app.diff_search.as_mut() {
                if !search.matches.is_empty() {
                    search.current = (search.current + 1) % search.matches.len();
                }
            }
            app.scroll_to_current_match();
        }
        DiffSearchPrev => {
            if let Some(search) = app.diff_search.as_mut() {
                if !search.matches.is_empty() {
                    if search.current == 0 {
                        search.current = search.matches.len() - 1;
                    } else {
                        search.current -= 1;
                    }
                }
            }
            app.scroll_to_current_match();
        }
        DiffSearchToggleCase => {
            if let Some(search) = app.diff_search.as_mut() {
                search.case_sensitive = !search.case_sensitive;
            }
            app.recompute_diff_search();
            app.scroll_to_current_match();
        }
    }
    // The visible file (and thus the match set) may have changed for actions
    // like NextFile, CommitsToggleSelect, modal file pick, etc. Cheap to
    // recompute against the current diff lines and keeps matches in sync.
    if app.diff_search.is_some() {
        app.recompute_diff_search();
    }
}

enum ModalInputOp {
    Insert(char),
    Backspace,
}

fn modal_input(app: &mut App, op: ModalInputOp) {
    let Some(modal) = app.modal.as_mut() else {
        return;
    };
    let picker = match modal {
        Modal::BranchPicker { picker, .. }
        | Modal::FileFilter { picker }
        | Modal::CommandPalette { picker } => picker,
        _ => return,
    };
    match op {
        ModalInputOp::Insert(c) => picker.query.insert(c),
        ModalInputOp::Backspace => picker.query.backspace(),
    }
    picker.refilter();
}

fn modal_move(app: &mut App, delta: i32) {
    let Some(modal) = app.modal.as_mut() else {
        return;
    };
    let picker = match modal {
        Modal::BranchPicker { picker, .. }
        | Modal::FileFilter { picker }
        | Modal::CommandPalette { picker } => picker,
        _ => return,
    };
    if delta < 0 {
        picker.move_up();
    } else {
        picker.move_down();
    }
}

fn handle_modal_accept(app: &mut App, modal: Modal, tx: &UnboundedSender<AppEvent>) {
    match modal {
        Modal::BranchPicker { which, picker } => {
            if let Some(b) = picker.current().cloned() {
                match which {
                    BranchSlot::Base => app.base_branch = Some(b),
                    BranchSlot::Compare => app.compare_branch = Some(b),
                }
                let just_picked_base = matches!(which, BranchSlot::Base);
                if matches!(app.screen, Screen::Setup) && just_picked_base {
                    // Force the user through the compare step before loading.
                    app.setup_field = crate::app::SetupField::Compare;
                    let items = picker_items_for(app, BranchSlot::Compare);
                    app.modal = Some(Modal::BranchPicker {
                        which: BranchSlot::Compare,
                        picker: crate::app::Picker::with_selected(
                            items,
                            app.compare_branch.as_deref(),
                        ),
                    });
                } else if app.base_branch.is_some() && app.compare_branch.is_some() {
                    kick_off_load_diff(app, tx);
                    kick_off_load_commits(app, tx);
                }
            }
        }
        Modal::FileFilter { picker } => {
            if let Some(p) = picker.current().cloned() {
                app.select_file_by_path(&p);
                app.focused = FocusedPane::Diff;
            }
        }
        Modal::CommandPalette { picker } => {
            if let Some(cmd) = picker.current().cloned() {
                run_command(app, &cmd, tx);
            }
        }
        _ => {}
    }
}

fn run_command(app: &mut App, cmd: &str, tx: &UnboundedSender<AppEvent>) {
    match cmd {
        "branches" => {
            let items = picker_items_for(app, BranchSlot::Compare);
            app.modal = Some(Modal::BranchPicker {
                which: BranchSlot::Compare,
                picker: crate::app::Picker::with_selected(items, app.compare_branch.as_deref()),
            });
        }
        "reload" => {
            kick_off_load_diff(app, tx);
            kick_off_load_commits(app, tx);
        }
        "tree" => {
            app.sidebar_mode = crate::app::SidebarMode::Tree;
            app.sidebar_selected = 0;
            app.rebuild_tree();
        }
        "flat" => {
            app.sidebar_mode = crate::app::SidebarMode::Flat;
            app.sidebar_selected = 0;
        }
        "fullscreen" => {
            if let Some(idx) = app.current_file_index() {
                app.fullscreen_idx = idx;
                app.screen = Screen::Fullscreen;
            }
        }
        "clear-reviewed" => {
            app.reviewed.clear();
            app.save_review();
            app.toast("Cleared reviewed set", ToastKind::Info);
        }
        "toggle-commits" => app.commits_panel_visible = !app.commits_panel_visible,
        "split-diff" => app.diff_mode = crate::app::DiffMode::Split,
        "unified-diff" => app.diff_mode = crate::app::DiffMode::Unified,
        "toggle-word-diff" => {
            app.toggle_word_diff();
            let on = app.config.word_diff;
            app.toast(
                format!("Word diff: {}", if on { "on" } else { "off" }),
                ToastKind::Info,
            );
        }
        "toggle-syntax" => {
            app.toggle_syntax_highlight();
            let on = app.config.syntax_highlight;
            app.toast(
                format!("Syntax highlight: {}", if on { "on" } else { "off" }),
                ToastKind::Info,
            );
        }
        "quit" => app.should_quit = true,
        _ => {}
    }
}

fn handle_setup_submit(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    use crate::app::SetupField::*;
    match app.setup_field {
        Repo => {
            let path = std::path::PathBuf::from(app.repo_input.buffer.trim());
            if path.as_os_str().is_empty() {
                app.toast("Repo path is empty", ToastKind::Error);
                return;
            }
            // If the repo actually changed, drop stale branch state so the
            // Branches handler will re-run preselection against the new repo.
            // Without this, base/compare from the previous repo carry over
            // and prevent any preselection (and won't match any row in the
            // new branch list, so nothing renders highlighted).
            let path_changed = app.repo_path.as_ref() != Some(&path);
            app.repo_path = Some(path);
            if path_changed {
                app.base_branch = None;
                app.compare_branch = None;
                app.branches.clear();
            }
            kick_off_load_branches(app, tx);
            app.setup_field = Base;
        }
        Base => app.setup_field = Compare,
        Compare => app.setup_field = Remote,
        Remote => {
            app.include_remote_branches = !app.include_remote_branches;
            kick_off_load_branches(app, tx);
            app.setup_field = Submit;
        }
        Submit => {
            if app.repo_path.is_some() && app.base_branch.is_some() && app.compare_branch.is_some()
            {
                kick_off_load_diff(app, tx);
                kick_off_load_commits(app, tx);
            } else {
                app.toast("Pick repo, base, and compare first", ToastKind::Error);
            }
        }
    }
}

fn next_field(f: crate::app::SetupField) -> crate::app::SetupField {
    use crate::app::SetupField::*;
    match f {
        Repo => Base,
        Base => Compare,
        Compare => Remote,
        Remote => Submit,
        Submit => Repo,
    }
}

fn prev_field(f: crate::app::SetupField) -> crate::app::SetupField {
    use crate::app::SetupField::*;
    match f {
        Repo => Submit,
        Base => Repo,
        Compare => Base,
        Remote => Compare,
        Submit => Remote,
    }
}

fn ensure_blame_for_visible(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    if !app.blame_enabled {
        return;
    }
    let Some(repo) = app.repo_path.clone() else {
        return;
    };
    let file = match app.screen {
        Screen::Fullscreen => app.files.get(app.fullscreen_idx).cloned(),
        _ => app.current_file().cloned(),
    };
    let Some(file) = file else { return };
    let Some(target) = app.blame_target_for(&file) else {
        return;
    };
    if app.blame_cache.contains_key(&target) || app.blame_pending.contains(&target) {
        return;
    }
    app.blame_pending.insert(target.clone());
    tasks::spawn_load_blame(tx.clone(), repo, target.0, target.1);
}

/// Items shown in the branch fuzzy-picker for a given slot. The Compare slot
/// gets a `[working tree]` pseudo-ref pinned at the top so users can review
/// uncommitted changes against any base.
fn picker_items_for(app: &App, slot: BranchSlot) -> Vec<String> {
    match slot {
        BranchSlot::Compare => {
            let mut items = Vec::with_capacity(app.branches.len() + 1);
            items.push(crate::git::WORKING_TREE_REF.to_string());
            items.extend(app.branches.iter().cloned());
            items
        }
        BranchSlot::Base => app.branches.clone(),
    }
}

fn kick_off_load_branches(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let Some(repo) = app.repo_path.clone() else {
        return;
    };
    app.req_ids.branches += 1;
    app.pending.branches = true;
    tasks::spawn_load_branches(
        tx.clone(),
        app.req_ids.branches,
        repo,
        app.include_remote_branches,
    );
}

fn kick_off_load_diff(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let (Some(repo), Some(base), Some(compare)) = (
        app.repo_path.clone(),
        app.base_branch.clone(),
        app.compare_branch.clone(),
    ) else {
        return;
    };
    app.req_ids.diff += 1;
    app.pending.diff = true;
    tasks::spawn_load_diff(tx.clone(), app.req_ids.diff, repo, base, compare);
}

fn kick_off_load_commits(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let (Some(repo), Some(base), Some(compare)) = (
        app.repo_path.clone(),
        app.base_branch.clone(),
        app.compare_branch.clone(),
    ) else {
        return;
    };
    app.req_ids.commits += 1;
    app.pending.commits = true;
    tasks::spawn_load_commits(tx.clone(), app.req_ids.commits, repo, base, compare);
}
