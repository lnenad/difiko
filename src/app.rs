use crate::cli::Cli;
use crate::config::Config;
use crate::model::{Commit, FileChange};
use crate::persistence::Store;
use crate::tree::{DirNode, TreeRow};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

/// Map a match's `diff_lines` index to the actual rendered-row index for the
/// current diff mode. The diff vector includes header entries (GitHeader,
/// IndexHeader, OldFile, NewFile) that produce no rendered row, and in split
/// mode adjacent dels/adds are paired into shared rows. Without this
/// translation, scroll math drifts and the match lands off-screen.
pub(crate) fn rendered_row_for_match(
    file: &crate::model::FileChange,
    mode: DiffMode,
    target: usize,
) -> usize {
    use crate::model::DiffLine;
    let mut rendered = 0usize;
    match mode {
        DiffMode::Unified => {
            for (i, dl) in file.diff_lines.iter().enumerate() {
                if i == target {
                    return rendered;
                }
                match dl {
                    DiffLine::GitHeader(_)
                    | DiffLine::IndexHeader(_)
                    | DiffLine::OldFile(_)
                    | DiffLine::NewFile(_) => continue,
                    _ => rendered += 1,
                }
            }
            rendered
        }
        DiffMode::Split => {
            let mut pending_del = 0usize;
            let mut pending_add = 0usize;
            for (i, dl) in file.diff_lines.iter().enumerate() {
                let on_target = i == target;
                match dl {
                    DiffLine::Hunk { .. } | DiffLine::Context(_) => {
                        // Pending del/add rows render before this row.
                        rendered += pending_del.max(pending_add);
                        pending_del = 0;
                        pending_add = 0;
                        if on_target {
                            return rendered;
                        }
                        rendered += 1;
                    }
                    DiffLine::Del(_) => {
                        if on_target {
                            return rendered + pending_del;
                        }
                        pending_del += 1;
                    }
                    DiffLine::Add(_) => {
                        if on_target {
                            return rendered + pending_add;
                        }
                        pending_add += 1;
                    }
                    _ => {
                        if on_target {
                            return rendered;
                        }
                    }
                }
            }
            rendered + pending_del.max(pending_add)
        }
    }
}

/// Count of rendered rows for the file in the given mode. Mirrors the
/// renderer's filtering (skips header pseudo-lines) and the split-mode
/// del/add pairing. Used to clamp scroll so we don't store a position past
/// the end of the content.
pub(crate) fn total_rendered_rows(file: &crate::model::FileChange, mode: DiffMode) -> usize {
    use crate::model::DiffLine;
    match mode {
        DiffMode::Unified => file
            .diff_lines
            .iter()
            .filter(|dl| {
                !matches!(
                    dl,
                    DiffLine::GitHeader(_)
                        | DiffLine::IndexHeader(_)
                        | DiffLine::OldFile(_)
                        | DiffLine::NewFile(_)
                )
            })
            .count(),
        DiffMode::Split => {
            let mut total = 0usize;
            let mut pending_del = 0usize;
            let mut pending_add = 0usize;
            for dl in &file.diff_lines {
                match dl {
                    DiffLine::Hunk { .. } | DiffLine::Context(_) => {
                        total += pending_del.max(pending_add);
                        pending_del = 0;
                        pending_add = 0;
                        total += 1;
                    }
                    DiffLine::Del(_) => pending_del += 1,
                    DiffLine::Add(_) => pending_add += 1,
                    _ => {}
                }
            }
            total + pending_del.max(pending_add)
        }
    }
}

/// Substring search returning byte ranges. In case-insensitive mode the
/// comparison is ASCII-only — non-ASCII bytes are compared byte-for-byte
/// (so "é" only matches "é"). Returned offsets are guaranteed to fall on
/// UTF-8 char boundaries so they can be used directly to slice `hay`.
fn find_all_substr(hay: &str, needle: &[u8], case_sensitive: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if needle.is_empty() || needle.len() > hay.len() {
        return out;
    }
    let h = hay.as_bytes();
    let mut i = 0;
    while i + needle.len() <= h.len() {
        let bytes_match = h[i..i + needle.len()].iter().zip(needle).all(|(a, b)| {
            if case_sensitive {
                a == b
            } else {
                a.eq_ignore_ascii_case(b)
            }
        });
        let on_boundary = hay.is_char_boundary(i) && hay.is_char_boundary(i + needle.len());
        if bytes_match && on_boundary {
            out.push((i, i + needle.len()));
        }
        i += 1;
    }
    out
}

fn split_path_for_completion(buf: &str) -> Option<(PathBuf, String)> {
    if buf.is_empty() {
        let cwd = std::env::current_dir().ok()?;
        return Some((cwd, String::new()));
    }
    if buf == "~" {
        let home = std::env::var_os("HOME")?;
        return Some((PathBuf::from(home), String::new()));
    }
    let (prefix, frag) = match buf.rfind('/') {
        Some(i) => (&buf[..=i], &buf[i + 1..]),
        None => ("", buf),
    };
    let parent: PathBuf = if prefix.is_empty() {
        std::env::current_dir().ok()?
    } else if prefix == "/" {
        PathBuf::from("/")
    } else if prefix == "~/" {
        PathBuf::from(std::env::var_os("HOME")?)
    } else if let Some(rest) = prefix.strip_prefix("~/") {
        let mut p = PathBuf::from(std::env::var_os("HOME")?);
        p.push(rest.trim_end_matches('/'));
        p
    } else {
        PathBuf::from(prefix.trim_end_matches('/'))
    };
    Some((parent, frag.to_string()))
}

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

#[derive(Debug, Clone, Default)]
pub struct TextInput {
    pub buffer: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(initial: impl Into<String>) -> Self {
        let buffer: String = initial.into();
        let cursor = buffer.chars().count();
        Self { buffer, cursor }
    }

    pub fn insert(&mut self, c: char) {
        let byte_pos = self.byte_cursor();
        self.buffer.insert(byte_pos, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte_end = self.byte_cursor();
        let prev = self.buffer[..byte_end]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        let byte_start = byte_end - prev;
        self.buffer.replace_range(byte_start..byte_end, "");
        self.cursor -= 1;
    }

    pub fn delete(&mut self) {
        let byte_pos = self.byte_cursor();
        if byte_pos >= self.buffer.len() {
            return;
        }
        let next = self.buffer[byte_pos..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        self.buffer.replace_range(byte_pos..byte_pos + next, "");
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        let len = self.buffer.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.buffer.chars().count();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }

    fn byte_cursor(&self) -> usize {
        self.buffer
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.buffer.len())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Picker {
    pub query: TextInput,
    pub items: Vec<String>,
    pub filtered: Vec<usize>,
    pub selected: usize,
}

impl Picker {
    pub fn new(items: Vec<String>) -> Self {
        let mut p = Picker {
            query: TextInput::new(""),
            items,
            filtered: Vec::new(),
            selected: 0,
        };
        p.refilter();
        p
    }

    /// Build a picker with the cursor positioned on `current`, when present.
    /// Used for branch pickers so re-opening them lands on the current value
    /// instead of the top of the list.
    pub fn with_selected(items: Vec<String>, current: Option<&str>) -> Self {
        let mut p = Self::new(items);
        if let Some(c) = current {
            if let Some(item_idx) = p.items.iter().position(|s| s == c) {
                if let Some(pos) = p.filtered.iter().position(|&i| i == item_idx) {
                    p.selected = pos;
                }
            }
        }
        p
    }

    pub fn refilter(&mut self) {
        let q = self.query.buffer.as_str();
        if q.is_empty() {
            self.filtered = (0..self.items.len()).collect();
        } else {
            use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
            use nucleo_matcher::{Matcher, Utf32Str};
            let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
            let pattern = Pattern::parse(q, CaseMatching::Smart, Normalization::Smart);
            let mut buf: Vec<char> = Vec::new();
            let mut scored: Vec<(u32, usize)> = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    let haystack = Utf32Str::new(s, &mut buf);
                    pattern
                        .score(haystack, &mut matcher)
                        .map(|score| (score, i))
                })
                .collect();
            scored.sort_by_key(|b| std::cmp::Reverse(b.0));
            self.filtered = scored.into_iter().map(|(_, i)| i).collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn current(&self) -> Option<&String> {
        self.filtered
            .get(self.selected)
            .and_then(|i| self.items.get(*i))
    }
}

#[derive(Debug, Clone)]
pub enum Modal {
    BranchPicker { which: BranchSlot, picker: Picker },
    FileFilter { picker: Picker },
    CommandPalette { picker: Picker },
    HelpOverlay,
    Error { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffMatch {
    /// Index into the current file's `diff_lines`.
    pub line: usize,
    /// Byte offsets into the diff line's text content (excluding gutters).
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DiffSearch {
    pub query: TextInput,
    pub matches: Vec<DiffMatch>,
    pub current: usize,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub created: Instant,
    pub kind: ToastKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Error,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ReqIds {
    pub branches: u64,
    pub diff: u64,
    pub commits: u64,
    pub commit_diff: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PendingOps {
    pub branches: bool,
    pub diff: bool,
    pub commits: bool,
    pub commit_diff: Option<String>,
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

    pub pending: PendingOps,
    pub req_ids: ReqIds,
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
        let Some((parent, frag)) = split_path_for_completion(&buf) else {
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
        if self.toasts.len() > 5 {
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
        let _ = self.config.save();
    }

    pub fn toggle_syntax_highlight(&mut self) {
        self.config.syntax_highlight = !self.config.syntax_highlight;
        self.syntax_cache.borrow_mut().clear();
        let _ = self.config.save();
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
                    for (s, e) in find_all_substr(text, needle, case_sensitive) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_input_insert_and_cursor() {
        let mut t = TextInput::new("");
        t.insert('a');
        t.insert('b');
        assert_eq!(t.buffer, "ab");
        assert_eq!(t.cursor, 2);
        t.move_left();
        t.insert('X');
        assert_eq!(t.buffer, "aXb");
        assert_eq!(t.cursor, 2);
    }

    #[test]
    fn text_input_backspace_multibyte() {
        let mut t = TextInput::new("héllo");
        t.move_end();
        t.backspace();
        assert_eq!(t.buffer, "héll");
        // Move so the multibyte 'é' is the char *before* the cursor, then
        // backspace it. Must not panic on the UTF-8 boundary.
        // Layout after backspace above: h é l l (cursor=4). To delete 'é',
        // place cursor at index 2.
        t.move_left();
        t.move_left();
        assert_eq!(t.cursor, 2);
        t.backspace();
        assert_eq!(t.buffer, "hll");
        assert_eq!(t.cursor, 1);
    }

    #[test]
    fn find_all_substr_ci_finds_overlapping_and_case_insensitive() {
        let hits = find_all_substr("Foo foo FOO", b"foo", false);
        assert_eq!(hits, vec![(0, 3), (4, 7), (8, 11)]);
    }

    #[test]
    fn find_all_substr_case_sensitive_filters() {
        let hits = find_all_substr("Foo foo FOO", b"foo", true);
        assert_eq!(hits, vec![(4, 7)]);
    }

    #[test]
    fn find_all_substr_returns_char_boundary_offsets() {
        // Non-ASCII bytes in haystack — needle is ASCII so it can only match
        // inside the ASCII region and must land on char boundaries.
        // "café" is 5 bytes (é is 2 bytes), then " bar" — "bar" starts at byte 6.
        let hits = find_all_substr("café bar", b"bar", false);
        assert_eq!(hits, vec![(6, 9)]);
        // Ensure slicing at returned offsets does not panic.
        assert_eq!(&"café bar"[6..9], "bar");
    }

    #[test]
    fn rendered_row_skips_unrendered_headers_in_unified() {
        use crate::model::{DiffLine, FileChange, FileStatus};
        let file = FileChange {
            path: "a".into(),
            old_path: None,
            status: FileStatus::Modified,
            // Two header lines that don't render, then Hunk + Context — the
            // match on diff_line index 3 should map to rendered row 1.
            diff_lines: vec![
                DiffLine::GitHeader("diff --git ...".into()),
                DiffLine::IndexHeader("index ...".into()),
                DiffLine::Hunk {
                    header: "@@".into(),
                    old_start: 1,
                    old_count: 1,
                    new_start: 1,
                    new_count: 1,
                },
                DiffLine::Context("x".into()),
            ],
            additions: 0,
            deletions: 0,
            binary: false,
        };
        assert_eq!(rendered_row_for_match(&file, DiffMode::Unified, 2), 0);
        assert_eq!(rendered_row_for_match(&file, DiffMode::Unified, 3), 1);
    }

    #[test]
    fn rendered_row_pairs_del_and_add_in_split() {
        use crate::model::{DiffLine, FileChange, FileStatus};
        let file = FileChange {
            path: "a".into(),
            old_path: None,
            status: FileStatus::Modified,
            // Hunk row, then Del,Del,Add — pairs into 2 rows. A match on the
            // second Add (diff index 4) is on the right pane row index 1
            // counting from after the hunk row, i.e. rendered row 2.
            diff_lines: vec![
                DiffLine::Hunk {
                    header: "@@".into(),
                    old_start: 1,
                    old_count: 1,
                    new_start: 1,
                    new_count: 1,
                },
                DiffLine::Del("a".into()),
                DiffLine::Del("b".into()),
                DiffLine::Add("c".into()),
                DiffLine::Add("d".into()),
                DiffLine::Context("e".into()),
            ],
            additions: 2,
            deletions: 2,
            binary: false,
        };
        // Hunk renders at row 0. First Del row 1, second Del row 2.
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 1), 1);
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 2), 2);
        // First Add row 1 (right pane), second Add row 2.
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 3), 1);
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 4), 2);
        // Context is after the 2-row del/add block — row 3.
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 5), 3);
    }

    #[test]
    fn find_all_substr_empty_needle_returns_nothing() {
        let hits = find_all_substr("anything", b"", false);
        assert!(hits.is_empty());
    }

    #[test]
    fn text_input_delete_at_end_is_noop() {
        let mut t = TextInput::new("ab");
        t.move_end();
        t.delete();
        assert_eq!(t.buffer, "ab");
        assert_eq!(t.cursor, 2);
    }

    #[test]
    fn picker_filters_and_selects() {
        let mut p = Picker::new(vec!["foo.rs".into(), "bar.rs".into(), "baz.txt".into()]);
        assert_eq!(p.filtered.len(), 3);
        for c in "ba".chars() {
            p.query.insert(c);
        }
        p.refilter();
        // "bar.rs" and "baz.txt" both match "ba"; "foo.rs" should not.
        assert_eq!(p.filtered.len(), 2);
        let cur = p.current().unwrap().clone();
        assert!(cur.starts_with("ba"));
        p.move_down();
        let cur2 = p.current().unwrap().clone();
        assert!(cur2.starts_with("ba"));
        assert_ne!(cur, cur2);
    }

    #[test]
    fn picker_with_selected_lands_on_current() {
        let p = Picker::with_selected(
            vec!["main".into(), "develop".into(), "feat/x".into()],
            Some("feat/x"),
        );
        assert_eq!(p.current().map(String::as_str), Some("feat/x"));
    }

    #[test]
    fn picker_with_selected_falls_back_when_missing() {
        let p = Picker::with_selected(vec!["main".into(), "develop".into()], Some("not-in-list"));
        // Falls back to first item, doesn't panic.
        assert_eq!(p.current().map(String::as_str), Some("main"));
    }

    #[test]
    fn picker_no_match_clears_selection() {
        let mut p = Picker::new(vec!["foo.rs".into()]);
        for c in "zzzz".chars() {
            p.query.insert(c);
        }
        p.refilter();
        assert!(p.filtered.is_empty());
        assert!(p.current().is_none());
    }
}
