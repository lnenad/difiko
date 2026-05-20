use super::loaders::{kick_off_load_branches, kick_off_load_commits, kick_off_load_diff};
use super::AppEvent;
use crate::app::{App, SetupField, ToastKind};
use tokio::sync::mpsc::UnboundedSender;

/// Kick off initial loads if a repo path was provided on the command line.
/// Decides which Setup field to focus based on what was pre-filled.
pub(super) fn bootstrap(app: &mut App, tx: &UnboundedSender<AppEvent>) {
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
                app.setup_field = SetupField::Compare;
            } else {
                // Default: skip the path field, land on Base.
                app.setup_field = SetupField::Base;
            }
        }
    }
}

/// Read the active HEAD branch from `.git/HEAD`, transparently handling
/// the linked-worktree case (`.git` is a file pointing at a gitdir).
/// Returns `None` for detached HEAD or anything we can't parse.
pub(super) fn head_branch(app: &App) -> Option<String> {
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

pub(super) fn handle_setup_submit(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    use SetupField::*;
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

pub(super) fn next_field(f: SetupField) -> SetupField {
    use SetupField::*;
    match f {
        Repo => Base,
        Base => Compare,
        Compare => Remote,
        Remote => Submit,
        Submit => Repo,
    }
}

pub(super) fn prev_field(f: SetupField) -> SetupField {
    use SetupField::*;
    match f {
        Repo => Submit,
        Base => Repo,
        Compare => Base,
        Remote => Compare,
        Submit => Remote,
    }
}
