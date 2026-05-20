//! Application state. `App` is intentionally flat — fields that span
//! multiple screens (reviewed set, diff_scroll cache, blame_cache,
//! fullscreen_idx, ...) live here side by side. Small self-contained
//! pieces (TextInput, Picker, Modal, search, toasts) get their own
//! submodule; re-exported below so `crate::app::X` paths are unchanged.

mod completion;
mod modal;
mod pending;
mod picker;
mod search;
mod text_input;
mod toast;

pub use modal::Modal;
pub use pending::CommitDiffState;
pub use picker::Picker;
pub use search::{DiffMatch, DiffSearch};
pub use text_input::TextInput;
pub use toast::{Toast, ToastKind};

pub(crate) use pending::{PendingOps, ReqIds};
pub(crate) use search::{rendered_row_for_match, total_rendered_rows};

use crate::cli::Cli;
use crate::config::Config;
use crate::model::{Commit, FileChange};
use crate::persistence::Store;
use crate::tree::{DirNode, TreeRow};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

/// Maximum number of in-flight toasts. Older ones are evicted FIFO so a
/// burst of errors during git failures can't fill the screen.
const MAX_TOASTS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Setup,
    Review,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Sidebar,
    Diff,
    Commits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    Flat,
    Tree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    Unified,
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupField {
    Repo,
    Base,
    Compare,
    Remote,
    Submit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchSlot {
    Base,
    Compare,
}

pub struct App {
    pub screen: Screen,
    pub focused: FocusedPane,
    pub modal: Option<Modal>,

    pub repo_input: TextInput,
    pub repo_path: Option<PathBuf>,
    pub setup_field: SetupField,
    pub repo_completions: Vec<String>,
    pub repo_completion_index: usize,
    pub repo_dropdown_hidden: bool,

    pub base_branch: Option<String>,
    pub compare_branch: Option<String>,
    pub include_remote_branches: bool,
    pub branches: Vec<String>,

    pub files: Vec<FileChange>,
    pub commits: Vec<Commit>,
    pub selected_commit: Option<String>,
    pub commit_diff_cache: HashMap<String, Vec<FileChange>>,
    pub all_files_backup: Option<Vec<FileChange>>,

    pub reviewed: HashSet<String>,
    pub sidebar_mode: SidebarMode,
    pub sidebar_selected: usize,
    pub sidebar_collapsed: HashSet<String>,
    pub tree_rows: Vec<TreeRow>,

    pub diff_scroll: HashMap<String, u16>,
    pub diff_scroll_h: HashMap<String, u16>,
    pub diff_mode: DiffMode,
    pub commits_collapsed: bool,
    pub commits_selected: usize,
    pub expanded_commit: Option<String>,
    pub commits_panel_visible: bool,

    pub pending_clear_reviewed: Option<Instant>,

    pub blame_enabled: bool,
    pub blame_cache: HashMap<(String, String), crate::git::Blame>,
    pub blame_pending: std::collections::HashSet<(String, String)>,

    pub fullscreen_idx: usize,

    /// Last rendered diff body height in rows. Updated each frame by the diff
    /// renderer; used by search-scroll to keep the active match on-screen.
    /// Interior-mutable so renderers (which see `&App`) can update it.
    pub diff_view_height: std::cell::Cell<u16>,

    pub diff_search: Option<DiffSearch>,

    pub(crate) pending: PendingOps,
    pub(crate) req_ids: ReqIds,
    /// One-shot guard so a failing config save doesn't toast on every
    /// keystroke. Toggled true on first failure and never reset.
    pub(crate) config_save_warned: bool,
    pub spinner_tick: u64,
    pub toasts: VecDeque<Toast>,
    pub store: Option<Store>,

    pub initial_file: Option<String>,
    pub initial_fullscreen: bool,

    pub config: Config,

    /// Per-file cache of syntect highlight output. Lazily populated by the
    /// renderer; cleared when a new diff loads. Interior-mutable so renderers
    /// holding `&App` can populate it.
    pub syntax_cache: RefCell<crate::ui::syntax::FileHighlights>,

    pub should_quit: bool,
}

impl App {
    pub fn new(cli: &Cli) -> Self {
        let cwd = std::env::current_dir().ok();
        let repo_path = cli.repo.clone().or(cwd);
        let repo_input = TextInput::new(
            repo_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );

        let (store, store_warning) = match Store::open() {
            Ok(r) => (
                Some(r.store),
                r.recovered_backup.map(|p| {
                    format!(
                        "State file was unreadable; backed up to {} and started fresh",
                        p.display()
                    )
                }),
            ),
            Err(_) => (None, None),
        };

        let mut app = Self {
            screen: Screen::Setup,
            focused: FocusedPane::Sidebar,
            modal: None,

            repo_input,
            repo_path,
            setup_field: SetupField::Repo,
            repo_completions: Vec::new(),
            repo_completion_index: 0,
            repo_dropdown_hidden: false,

            base_branch: cli.base.clone(),
            compare_branch: cli.compare.clone(),
            include_remote_branches: !cli.no_remote_branches,
            branches: Vec::new(),

            files: Vec::new(),
            commits: Vec::new(),
            selected_commit: None,
            commit_diff_cache: HashMap::new(),
            all_files_backup: None,

            reviewed: HashSet::new(),
            sidebar_mode: SidebarMode::Flat,
            sidebar_selected: 0,
            sidebar_collapsed: HashSet::new(),
            tree_rows: Vec::new(),

            diff_scroll: HashMap::new(),
            diff_scroll_h: HashMap::new(),
            diff_mode: DiffMode::Unified,
            commits_collapsed: false,
            commits_selected: 0,
            expanded_commit: None,
            commits_panel_visible: true,

            pending_clear_reviewed: None,

            blame_enabled: false,
            blame_cache: HashMap::new(),
            blame_pending: std::collections::HashSet::new(),

            fullscreen_idx: 0,

            diff_view_height: std::cell::Cell::new(0),

            diff_search: None,

            pending: PendingOps::default(),
            req_ids: ReqIds::default(),
            config_save_warned: false,
            spinner_tick: 0,
            toasts: VecDeque::new(),
            store,

            initial_file: cli.file.clone(),
            initial_fullscreen: cli.fullscreen,

            config: {
                let mut c = Config::load();
                if cli.no_word_diff {
                    c.word_diff = false;
                }
                if cli.no_syntax {
                    c.syntax_highlight = false;
                }
                c
            },
            syntax_cache: RefCell::new(HashMap::new()),

            should_quit: false,
        };
        if let Some(msg) = store_warning {
            app.toast(msg, ToastKind::Error);
        }
        app.update_repo_completions();
        app
    }

    pub fn update_repo_completions(&mut self) {
        let buf = self.repo_input.buffer.clone();
        let Some((parent, frag)) = completion::split_path_for_completion(&buf) else {
            self.repo_completions.clear();
            self.repo_completion_index = 0;
            return;
        };
        let frag_lower = frag.to_lowercase();
        let mut matches: Vec<String> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&parent) {
            for e in entries.flatten() {
                let Ok(ft) = e.file_type() else { continue };
                if !ft.is_dir() {
                    continue;
                }
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') && !frag.starts_with('.') {
                    continue;
                }
                if !frag.is_empty() && !name.to_lowercase().contains(&frag_lower) {
                    continue;
                }
                matches.push(name);
            }
        }
        matches.sort_by(|a, b| {
            let a_prefix = a.to_lowercase().starts_with(&frag_lower);
            let b_prefix = b.to_lowercase().starts_with(&frag_lower);
            b_prefix
                .cmp(&a_prefix)
                .then_with(|| a.to_lowercase().cmp(&b.to_lowercase()))
        });
        self.repo_completions = matches;
        if self.repo_completion_index >= self.repo_completions.len() {
            self.repo_completion_index = 0;
        }
    }

    pub fn accept_repo_completion(&mut self) {
        if self.repo_completions.is_empty() {
            return;
        }
        let candidate = self.repo_completions[self.repo_completion_index].clone();
        let buf = &self.repo_input.buffer;
        let new = if let Some(i) = buf.rfind('/') {
            format!("{}{}/", &buf[..=i], candidate)
        } else if buf == "~" {
            format!("~/{}/", candidate)
        } else {
            format!("{}/", candidate)
        };
        self.repo_input.buffer = new;
        self.repo_input.move_end();
        self.repo_completion_index = 0;
        self.update_repo_completions();
        self.repo_dropdown_hidden = true;
    }

    pub fn cycle_repo_completion(&mut self, delta: i32) {
        if self.repo_completions.is_empty() {
            return;
        }
        let n = self.repo_completions.len() as i32;
        let cur = self.repo_completion_index as i32;
        self.repo_completion_index = (((cur + delta) % n + n) % n) as usize;
    }

    pub fn rebuild_tree(&mut self) {
        let paths: Vec<&str> = self.files.iter().map(|f| f.path.as_str()).collect();
        let root = DirNode::from_paths(paths);
        self.tree_rows = crate::tree::flatten(&root, &self.sidebar_collapsed);
    }

    pub fn toast(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.toasts.push_back(Toast {
            message: message.into(),
            created: Instant::now(),
            kind,
        });
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.pop_front();
        }
    }

    pub fn current_file_index(&self) -> Option<usize> {
        match self.sidebar_mode {
            SidebarMode::Flat => {
                if self.sidebar_selected < self.files.len() {
                    Some(self.sidebar_selected)
                } else {
                    None
                }
            }
            SidebarMode::Tree => {
                let row = self.tree_rows.get(self.sidebar_selected)?;
                let path = match row {
                    TreeRow::File { path, .. } => path.as_str(),
                    TreeRow::Dir { .. } => return None,
                };
                self.files.iter().position(|f| f.path == path)
            }
        }
    }

    pub fn current_file(&self) -> Option<&FileChange> {
        self.current_file_index().and_then(|i| self.files.get(i))
    }

    pub fn select_file_by_path(&mut self, target: &str) {
        match self.sidebar_mode {
            SidebarMode::Flat => {
                if let Some(idx) = self.files.iter().position(|f| f.path == target) {
                    self.sidebar_selected = idx;
                }
            }
            SidebarMode::Tree => {
                if let Some(idx) = self
                    .tree_rows
                    .iter()
                    .position(|r| matches!(r, TreeRow::File { path, .. } if path == target))
                {
                    self.sidebar_selected = idx;
                }
            }
        }
    }

    pub fn focus_next_pane(&mut self) {
        self.focused = match self.focused {
            FocusedPane::Sidebar => FocusedPane::Diff,
            FocusedPane::Diff => {
                if self.commits_panel_visible {
                    FocusedPane::Commits
                } else {
                    FocusedPane::Sidebar
                }
            }
            FocusedPane::Commits => FocusedPane::Sidebar,
        };
    }

    pub fn focus_prev_pane(&mut self) {
        self.focused = match self.focused {
            FocusedPane::Sidebar => {
                if self.commits_panel_visible {
                    FocusedPane::Commits
                } else {
                    FocusedPane::Diff
                }
            }
            FocusedPane::Diff => FocusedPane::Sidebar,
            FocusedPane::Commits => FocusedPane::Diff,
        };
    }

    pub fn save_review(&mut self) {
        // Don't trample persisted state when no diff has loaded yet — the file
        // list would be empty and the snapshot would mismatch on next load.
        if self.all_files_backup.is_none() && self.files.is_empty() {
            return;
        }
        if let (Some(repo), Some(base), Some(compare)) =
            (&self.repo_path, &self.base_branch, &self.compare_branch)
        {
            if let Some(store) = self.store.as_mut() {
                let _ = store.save_reviewed(
                    &repo.display().to_string(),
                    base,
                    compare,
                    self.all_files_backup.as_deref().unwrap_or(&self.files),
                    &self.reviewed,
                );
            }
        }
    }

    pub fn load_review(&mut self) {
        if let (Some(repo), Some(base), Some(compare), Some(store)) = (
            &self.repo_path,
            &self.base_branch,
            &self.compare_branch,
            self.store.as_ref(),
        ) {
            self.reviewed =
                store.load_reviewed(&repo.display().to_string(), base, compare, &self.files);
        }
    }

    pub fn toggle_word_diff(&mut self) {
        self.config.word_diff = !self.config.word_diff;
        self.save_config();
    }

    pub fn toggle_syntax_highlight(&mut self) {
        self.config.syntax_highlight = !self.config.syntax_highlight;
        self.syntax_cache.borrow_mut().clear();
        self.save_config();
    }

    pub fn total_additions(&self) -> u32 {
        self.files.iter().map(|f| f.additions).sum()
    }
    pub fn total_deletions(&self) -> u32 {
        self.files.iter().map(|f| f.deletions).sum()
    }

    /// File currently visible in the diff panel (respects Review vs Fullscreen).
    pub fn diff_visible_file(&self) -> Option<&crate::model::FileChange> {
        match self.screen {
            Screen::Fullscreen => self.files.get(self.fullscreen_idx),
            _ => self.current_file(),
        }
    }

    /// Path of the file currently visible in the diff panel. Same logic as
    /// `diff_visible_file()` but returns an owned `String` so callers can
    /// use it as a HashMap key without holding a borrow on `App`.
    pub fn diff_visible_path(&self) -> Option<String> {
        self.diff_visible_file().map(|f| f.path.clone())
    }

    /// Start a branches load: bumps req_id, flips the pending flag, returns
    /// the id to hand to the spawner. Paired so callers can't forget either
    /// half of the invariant (req_id ordering = freshness).
    pub(crate) fn start_branches_load(&mut self) -> u64 {
        self.req_ids.branches += 1;
        self.pending.branches = true;
        self.req_ids.branches
    }

    pub(crate) fn start_diff_load(&mut self) -> u64 {
        self.req_ids.diff += 1;
        self.pending.diff = true;
        self.req_ids.diff
    }

    pub(crate) fn start_commits_load(&mut self) -> u64 {
        self.req_ids.commits += 1;
        self.pending.commits = true;
        self.req_ids.commits
    }

    /// Start a commit-diff load for `hash`. Returns the req_id.
    pub(crate) fn start_commit_diff_load(&mut self, hash: String) -> u64 {
        self.req_ids.commit_diff += 1;
        self.pending.commit_diff = CommitDiffState::Loading(hash);
        self.req_ids.commit_diff
    }

    /// Cancel any in-flight diff/commits/commit-diff loads by bumping their
    /// req_ids so arriving results are dropped. Used by soft-back so a
    /// late diff response can't auto-flip the user back to Review.
    pub(crate) fn cancel_inflight_loads(&mut self) {
        if self.pending.diff {
            self.req_ids.diff += 1;
            self.pending.diff = false;
        }
        if self.pending.commits {
            self.req_ids.commits += 1;
            self.pending.commits = false;
        }
        if self.pending.commit_diff.is_active() {
            self.req_ids.commit_diff += 1;
            self.pending.commit_diff = CommitDiffState::Idle;
        }
    }

    /// Esc from Review → back to Setup with state preserved. Lands focus on
    /// Compare for a one-keystroke change of compare branch. A second Esc
    /// on Setup invokes `hard_reset` for a full restart.
    pub fn soft_back_to_setup(&mut self) {
        self.cancel_inflight_loads();
        self.screen = Screen::Setup;
        self.setup_field = SetupField::Compare;
        self.focused = FocusedPane::Sidebar;
        self.modal = None;
    }

    /// Full reset back to Setup: drops branches, files, commits, reviewed
    /// set, and any cached commit-diffs. Saves the current reviewed set
    /// first so it survives the reset. Used by SetupReset (second Esc).
    pub fn hard_reset(&mut self) {
        self.save_review();
        self.base_branch = None;
        self.compare_branch = None;
        self.branches.clear();
        self.files.clear();
        self.commits.clear();
        self.commit_diff_cache.clear();
        self.all_files_backup = None;
        self.selected_commit = None;
        self.reviewed.clear();
        self.diff_scroll.clear();
        self.tree_rows.clear();
        self.sidebar_selected = 0;
        self.fullscreen_idx = 0;
        self.pending = PendingOps::default();
        // Bump req IDs so any in-flight git results from the previous session are dropped.
        self.req_ids.branches += 1;
        self.req_ids.diff += 1;
        self.req_ids.commits += 1;
        self.req_ids.commit_diff += 1;
        self.screen = Screen::Setup;
        self.setup_field = SetupField::Repo;
        self.focused = FocusedPane::Sidebar;
        self.modal = None;
        self.update_repo_completions();
    }

    /// Persist the config. Surfaces failures once per session as an error
    /// toast (e.g. `~/.config` is read-only) so the user knows their flag
    /// toggle won't survive a restart.
    fn save_config(&mut self) {
        if self.config.save().is_err() && !self.config_save_warned {
            self.config_save_warned = true;
            self.toast(
                "Could not save config; preferences won't persist this session",
                ToastKind::Error,
            );
        }
    }

    /// Recompute search matches against the visible file's diff_lines. ASCII
    /// case-insensitive — non-ASCII byte sequences only match exactly. Keeps
    /// `current` valid (clamped to the new match count, or 0 if empty).
    pub fn recompute_diff_search(&mut self) {
        let Some((query, case_sensitive)) = self
            .diff_search
            .as_ref()
            .map(|s| (s.query.buffer.clone(), s.case_sensitive))
        else {
            return;
        };
        let mut matches: Vec<DiffMatch> = Vec::new();
        if !query.is_empty() {
            let file_idx = match self.screen {
                Screen::Fullscreen => Some(self.fullscreen_idx),
                _ => self.current_file_index(),
            };
            if let Some(file) = file_idx.and_then(|i| self.files.get(i)) {
                let needle = query.as_bytes();
                for (i, dl) in file.diff_lines.iter().enumerate() {
                    let text: &str = match dl {
                        crate::model::DiffLine::Add(t)
                        | crate::model::DiffLine::Del(t)
                        | crate::model::DiffLine::Context(t) => t,
                        crate::model::DiffLine::Hunk { header, .. } => header,
                        _ => continue,
                    };
                    for (s, e) in search::find_all_substr(text, needle, case_sensitive) {
                        matches.push(DiffMatch {
                            line: i,
                            start: s,
                            end: e,
                        });
                    }
                }
            }
        }
        let search = self.diff_search.as_mut().unwrap();
        let prev_count = search.matches.len();
        search.matches = matches;
        if search.matches.is_empty() || prev_count == 0 || search.current >= search.matches.len() {
            search.current = 0;
        }
    }

    /// Scroll the visible diff so the current match sits in the middle of
    /// the panel. Near the top/bottom of the diff we let natural clamping
    /// (against 0 and `total - height`) keep the diff from scrolling past
    /// its content, so matches there stay near the edge instead of leaving
    /// empty space.
    pub fn scroll_to_current_match(&mut self) {
        let Some(search) = self.diff_search.as_ref() else {
            return;
        };
        let Some(m) = search.matches.get(search.current).copied() else {
            return;
        };
        let Some(file) = self.diff_visible_file() else {
            return;
        };
        let path = file.path.clone();
        let rendered = rendered_row_for_match(file, self.diff_mode, m.line) as i32;
        let height = self.diff_view_height.get().max(4) as i32;
        let total = total_rendered_rows(file, self.diff_mode) as i32;
        let max_scroll = (total - height).max(0);
        let centered = rendered - height / 2;
        let new = centered.max(0).min(max_scroll);
        self.diff_scroll.insert(path, new as u16);
    }

    /// Returns the (ref, file_path) pair to look up blame for the given file, if any.
    /// For deletions we'd need base ref blame — skipped in MVP, returns None.
    pub fn blame_target_for(&self, file: &crate::model::FileChange) -> Option<(String, String)> {
        if matches!(file.status, crate::model::FileStatus::Deleted) {
            return None;
        }
        let r = self.compare_branch.as_ref()?.clone();
        // The working-tree pseudo-ref isn't a real git ref. Blame the HEAD
        // contents instead so users still see committed authorship; lines
        // added since HEAD simply won't have blame data, which is fine.
        let r = if crate::git::is_working_tree(&r) {
            "HEAD".to_string()
        } else {
            r
        };
        Some((r, file.path.clone()))
    }
}
