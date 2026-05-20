use super::loaders::ensure_blame_for_visible;
use super::setup::head_branch;
use super::{AppEvent, GitResult, GitResultKind};
use crate::app::{App, CommitDiffState, Screen, ToastKind};
use tokio::sync::mpsc::UnboundedSender;

pub(super) fn handle_git_result(app: &mut App, result: GitResult, tx: &UnboundedSender<AppEvent>) {
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
            if !app.pending.commit_diff.is_loading(&hash)
                || result.req_id != app.req_ids.commit_diff
            {
                return;
            }
            app.pending.commit_diff = CommitDiffState::Idle;
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
