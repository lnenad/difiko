/// Monotonic request-id counters per git op. Bumped before spawn; the
/// result handler drops anything that doesn't match the current value,
/// so rapid navigation never lets a stale response stomp current state.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ReqIds {
    pub branches: u64,
    pub diff: u64,
    pub commits: u64,
    pub commit_diff: u64,
}

/// Loading-state mirrors the in-flight async git operations so the status
/// bar can show a spinner. Always paired with `ReqIds` — see App's
/// `start_*_load` methods.
#[derive(Debug, Clone, Default)]
pub(crate) struct PendingOps {
    pub branches: bool,
    pub diff: bool,
    pub commits: bool,
    pub commit_diff: CommitDiffState,
}

/// Loading state for the per-commit diff. The hash carried in `Loading` is
/// what `handle_git_result::CommitDiff` matches against to drop stale
/// results (e.g. user moved off the commit before its diff arrived).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CommitDiffState {
    #[default]
    Idle,
    Loading(String),
}

impl CommitDiffState {
    pub fn is_loading(&self, hash: &str) -> bool {
        matches!(self, CommitDiffState::Loading(h) if h == hash)
    }
    pub fn is_active(&self) -> bool {
        matches!(self, CommitDiffState::Loading(_))
    }
}
