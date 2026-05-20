use super::AppEvent;
use crate::app::{App, BranchSlot, Screen};
use crate::tasks;
use tokio::sync::mpsc::UnboundedSender;

/// Items shown in the branch fuzzy-picker for a given slot. The Compare slot
/// gets a `[working tree]` pseudo-ref pinned at the top so users can review
/// uncommitted changes against any base.
pub(super) fn picker_items_for(app: &App, slot: BranchSlot) -> Vec<String> {
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

pub(super) fn kick_off_load_branches(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let Some(repo) = app.repo_path.clone() else {
        return;
    };
    let include_remote = app.include_remote_branches;
    let req_id = app.start_branches_load();
    tasks::spawn_load_branches(tx.clone(), req_id, repo, include_remote);
}

pub(super) fn kick_off_load_diff(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let (Some(repo), Some(base), Some(compare)) = (
        app.repo_path.clone(),
        app.base_branch.clone(),
        app.compare_branch.clone(),
    ) else {
        return;
    };
    let req_id = app.start_diff_load();
    tasks::spawn_load_diff(tx.clone(), req_id, repo, base, compare);
}

pub(super) fn kick_off_load_commits(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let (Some(repo), Some(base), Some(compare)) = (
        app.repo_path.clone(),
        app.base_branch.clone(),
        app.compare_branch.clone(),
    ) else {
        return;
    };
    let req_id = app.start_commits_load();
    tasks::spawn_load_commits(tx.clone(), req_id, repo, base, compare);
}

pub(super) fn ensure_blame_for_visible(app: &mut App, tx: &UnboundedSender<AppEvent>) {
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
