use super::loaders::{
    kick_off_load_branches, kick_off_load_commits, kick_off_load_diff, picker_items_for,
};
use super::modal::{handle_modal_accept, modal_input, modal_move, ModalInputOp};
use super::mouse::{move_commits, move_sidebar, scroll_diff, scroll_diff_h};
use super::setup::{handle_setup_submit, next_field, prev_field};
use super::AppEvent;
use crate::app::{App, BranchSlot, FocusedPane, Modal, Picker, Screen, ToastKind};
use crate::input::KeyAction;
use crate::tasks;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

/// Second press of `R` within this window clears the reviewed set.
const CLEAR_REVIEWED_CONFIRM_SECS: u64 = 2;

pub(super) fn apply_action(app: &mut App, action: KeyAction, tx: &UnboundedSender<AppEvent>) {
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
                picker: Picker::new(items),
            });
        }
        OpenFileFilter => {
            let items: Vec<String> = app.files.iter().map(|f| f.path.clone()).collect();
            app.modal = Some(Modal::FileFilter {
                picker: Picker::new(items),
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
                picker: Picker::with_selected(items, current),
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
                super::loaders::ensure_blame_for_visible(app, tx);
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
            if let Some(p) = app.diff_visible_path() {
                if app.reviewed.contains(&p) {
                    app.reviewed.remove(&p);
                } else {
                    app.reviewed.insert(p);
                }
                app.save_review();
            }
        }
        ClearReviewed => {
            let confirm_window = Duration::from_secs(CLEAR_REVIEWED_CONFIRM_SECS);
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
                app.toast(
                    format!(
                        "Press R again within {CLEAR_REVIEWED_CONFIRM_SECS}s to clear reviewed"
                    ),
                    ToastKind::Info,
                );
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
            if let Some(key) = app.diff_visible_path() {
                app.diff_scroll.insert(key, 0);
            }
        }
        ScrollDiffBottom => {
            if let Some(key) = app.diff_visible_path() {
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
                    let req_id = app.start_commit_diff_load(c.hash.clone());
                    tasks::spawn_load_commit_diff(tx.clone(), req_id, repo, c.hash);
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
        BackToSetupSoft => app.soft_back_to_setup(),
        SetupReset => app.hard_reset(),
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
