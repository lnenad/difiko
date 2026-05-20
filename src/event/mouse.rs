use crate::app::{App, FocusedPane, SidebarMode};
use crossterm::event::{MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

/// How long a toast stays on screen before it expires.
const TOAST_LIFETIME_SECS: u64 = 4;
/// Mouse-wheel scroll step for the diff body.
const MOUSE_SCROLL_DIFF: i32 = 3;
/// Mouse-wheel scroll step for the sidebar.
const MOUSE_SCROLL_SIDEBAR: i32 = 3;
/// Mouse-wheel scroll step for the commits panel (rows are short, one at a time).
const MOUSE_SCROLL_COMMITS: i32 = 1;

pub(super) fn handle_mouse(app: &mut App, ev: MouseEvent) {
    match ev.kind {
        MouseEventKind::ScrollDown => match app.focused {
            FocusedPane::Diff => scroll_diff(app, MOUSE_SCROLL_DIFF),
            FocusedPane::Sidebar => move_sidebar(app, MOUSE_SCROLL_SIDEBAR),
            FocusedPane::Commits => move_commits(app, MOUSE_SCROLL_COMMITS),
        },
        MouseEventKind::ScrollUp => match app.focused {
            FocusedPane::Diff => scroll_diff(app, -MOUSE_SCROLL_DIFF),
            FocusedPane::Sidebar => move_sidebar(app, -MOUSE_SCROLL_SIDEBAR),
            FocusedPane::Commits => move_commits(app, -MOUSE_SCROLL_COMMITS),
        },
        _ => {}
    }
}

pub(super) fn move_sidebar(app: &mut App, by: i32) {
    let len = match app.sidebar_mode {
        SidebarMode::Flat => app.files.len(),
        SidebarMode::Tree => app.tree_rows.len(),
    };
    if len == 0 {
        return;
    }
    let new = (app.sidebar_selected as i32 + by).clamp(0, len as i32 - 1);
    app.sidebar_selected = new as usize;
}

pub(super) fn move_commits(app: &mut App, by: i32) {
    if app.commits.is_empty() {
        return;
    }
    let new = (app.commits_selected as i32 + by).clamp(0, app.commits.len() as i32 - 1);
    app.commits_selected = new as usize;
}

pub(super) fn scroll_diff(app: &mut App, by: i32) {
    let Some(key) = app.diff_visible_path() else {
        return;
    };
    let cur = *app.diff_scroll.get(&key).unwrap_or(&0u16);
    let new = (cur as i32 + by).max(0) as u16;
    app.diff_scroll.insert(key, new);
}

pub(super) fn scroll_diff_h(app: &mut App, by: i32) {
    let Some(key) = app.diff_visible_path() else {
        return;
    };
    let cur = *app.diff_scroll_h.get(&key).unwrap_or(&0u16);
    let new = (cur as i32 + by).max(0) as u16;
    app.diff_scroll_h.insert(key, new);
}

pub(super) fn expire_toasts(app: &mut App) {
    let now = Instant::now();
    while let Some(t) = app.toasts.front() {
        if now.duration_since(t.created) > Duration::from_secs(TOAST_LIFETIME_SECS) {
            app.toasts.pop_front();
        } else {
            break;
        }
    }
}
