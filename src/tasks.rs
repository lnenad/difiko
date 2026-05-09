use crate::event::{AppEvent, GitResult, GitResultKind};
use crate::git;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

pub fn spawn_load_branches(
    tx: UnboundedSender<AppEvent>,
    req_id: u64,
    repo: PathBuf,
    include_remote: bool,
) {
    tokio::spawn(async move {
        let res = match git::ensure_git_repo(&repo).await {
            Ok(()) => git::list_branches(&repo, include_remote).await,
            Err(e) => Err(e),
        };
        let _ = tx.send(AppEvent::Git(GitResult {
            req_id,
            kind: GitResultKind::Branches(res),
        }));
    });
}

pub fn spawn_load_diff(
    tx: UnboundedSender<AppEvent>,
    req_id: u64,
    repo: PathBuf,
    base: String,
    compare: String,
) {
    tokio::spawn(async move {
        let res = git::load_diff(&repo, &base, &compare).await;
        let _ = tx.send(AppEvent::Git(GitResult {
            req_id,
            kind: GitResultKind::Diff(res),
        }));
    });
}

pub fn spawn_load_commits(
    tx: UnboundedSender<AppEvent>,
    req_id: u64,
    repo: PathBuf,
    base: String,
    compare: String,
) {
    tokio::spawn(async move {
        let res = git::load_commits(&repo, &base, &compare).await;
        let _ = tx.send(AppEvent::Git(GitResult {
            req_id,
            kind: GitResultKind::Commits(res),
        }));
    });
}

pub fn spawn_load_commit_diff(
    tx: UnboundedSender<AppEvent>,
    req_id: u64,
    repo: PathBuf,
    commit: String,
) {
    tokio::spawn(async move {
        let res = git::load_commit_diff(&repo, &commit).await;
        let hash = commit.clone();
        let _ = tx.send(AppEvent::Git(GitResult {
            req_id,
            kind: GitResultKind::CommitDiff { hash, result: res },
        }));
    });
}

pub fn spawn_load_blame(
    tx: UnboundedSender<AppEvent>,
    repo: PathBuf,
    git_ref: String,
    file: String,
) {
    tokio::spawn(async move {
        let result = git::load_blame(&repo, &git_ref, &file).await;
        let _ = tx.send(AppEvent::Git(GitResult {
            req_id: 0,
            kind: GitResultKind::Blame {
                git_ref: git_ref.clone(),
                file: file.clone(),
                result,
            },
        }));
    });
}
