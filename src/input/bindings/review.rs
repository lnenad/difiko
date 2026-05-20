use super::{ctrl, global_key, KeyAction, DIFF_SCROLL_HALF, DIFF_SCROLL_H_STEP, DIFF_SCROLL_PAGE};
use crate::app::{App, BranchSlot, FocusedPane};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn review_key(app: &App, key: KeyEvent) -> Option<KeyAction> {
    if ctrl(&key, 'c') {
        return Some(KeyAction::Quit);
    }
    if let Some(common) = global_key(key) {
        return Some(common);
    }
    if ctrl(&key, 'd') {
        return Some(KeyAction::ScrollDiff(DIFF_SCROLL_HALF));
    }
    if ctrl(&key, 'u') {
        return Some(KeyAction::ScrollDiff(-DIFF_SCROLL_HALF));
    }
    if ctrl(&key, 'f') {
        // Ctrl+F opens diff search when the diff is focused; elsewhere it
        // keeps its prior page-down meaning so users in the sidebar/commits
        // don't lose a familiar shortcut.
        if matches!(app.focused, FocusedPane::Diff) {
            return Some(KeyAction::DiffSearchOpen);
        }
        return Some(KeyAction::ScrollDiff(DIFF_SCROLL_PAGE));
    }
    if ctrl(&key, 'b') {
        return Some(KeyAction::ScrollDiff(-DIFF_SCROLL_PAGE));
    }
    if ctrl(&key, 'j') {
        return Some(KeyAction::ScrollDiff(DIFF_SCROLL_PAGE));
    }
    if ctrl(&key, 'k') {
        return Some(KeyAction::ScrollDiff(-DIFF_SCROLL_PAGE));
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Down => return Some(KeyAction::ScrollDiff(DIFF_SCROLL_PAGE)),
            KeyCode::Up => return Some(KeyAction::ScrollDiff(-DIFF_SCROLL_PAGE)),
            _ => {}
        }
    }
    match (app.focused, key.code, key.modifiers) {
        // Esc returns to the Setup screen but keeps branches/files loaded so
        // the user can quickly tweak the compare branch. A second Esc on
        // Setup performs the full reset.
        (_, KeyCode::Esc, _) => Some(KeyAction::BackToSetupSoft),
        // Pane switching
        (_, KeyCode::Tab, _) => Some(KeyAction::FocusNextPane),
        (_, KeyCode::BackTab, _) => Some(KeyAction::FocusPrevPane),
        (_, KeyCode::Char('1'), m) if m.is_empty() => Some(KeyAction::FocusSidebar),
        (_, KeyCode::Char('2'), m) if m.is_empty() => Some(KeyAction::FocusDiff),
        (_, KeyCode::Char('3'), m) if m.is_empty() => Some(KeyAction::FocusCommits),

        // Modeless
        (_, KeyCode::Char('m'), m) if m.is_empty() => Some(KeyAction::ToggleReviewed),
        (_, KeyCode::Char('R'), _) => Some(KeyAction::ClearReviewed),
        (_, KeyCode::Char('r'), m) if m.is_empty() => Some(KeyAction::ReloadDiff),
        (_, KeyCode::Char('B'), _) => Some(KeyAction::OpenBranchPicker(BranchSlot::Compare)),
        (_, KeyCode::Char('t'), m) if m.is_empty() => Some(KeyAction::ToggleSidebarMode),
        (_, KeyCode::Char('c'), m) if m.is_empty() => Some(KeyAction::ToggleCommitsPanel),
        (_, KeyCode::Char('u'), m) if m.is_empty() => Some(KeyAction::ToggleDiffMode),
        (_, KeyCode::Char('b'), m) if m.is_empty() => Some(KeyAction::ToggleBlame),
        (_, KeyCode::Char('W'), _) => Some(KeyAction::ToggleWordDiff),
        (_, KeyCode::Char('S'), _) => Some(KeyAction::ToggleSyntaxHighlight),
        (_, KeyCode::Char('F'), _) => Some(KeyAction::EnterFullscreen),
        (_, KeyCode::Char('/'), m) if m.is_empty() => Some(KeyAction::OpenFileFilter),
        (_, KeyCode::Char('J'), _) => Some(KeyAction::NextFile),
        (_, KeyCode::Char('K'), _) => Some(KeyAction::PrevFile),

        // Sidebar
        (FocusedPane::Sidebar, KeyCode::Char('j'), _)
        | (FocusedPane::Sidebar, KeyCode::Down, _) => Some(KeyAction::SidebarMove(1)),
        (FocusedPane::Sidebar, KeyCode::Char('k'), _) | (FocusedPane::Sidebar, KeyCode::Up, _) => {
            Some(KeyAction::SidebarMove(-1))
        }
        (FocusedPane::Sidebar, KeyCode::Char('g'), _) => Some(KeyAction::SidebarTop),
        (FocusedPane::Sidebar, KeyCode::Char('G'), _) => Some(KeyAction::SidebarBottom),
        (FocusedPane::Sidebar, KeyCode::Enter, _)
        | (FocusedPane::Sidebar, KeyCode::Char('l'), _) => Some(KeyAction::SidebarOpenFile),
        (FocusedPane::Sidebar, KeyCode::Char(' '), _) => Some(KeyAction::SidebarToggleFolder),

        // Diff
        (FocusedPane::Diff, KeyCode::Char('j'), _) | (FocusedPane::Diff, KeyCode::Down, _) => {
            Some(KeyAction::ScrollDiff(1))
        }
        (FocusedPane::Diff, KeyCode::Char('k'), _) | (FocusedPane::Diff, KeyCode::Up, _) => {
            Some(KeyAction::ScrollDiff(-1))
        }
        (FocusedPane::Diff, KeyCode::Char('h'), _) | (FocusedPane::Diff, KeyCode::Left, _) => {
            Some(KeyAction::ScrollDiffH(-DIFF_SCROLL_H_STEP))
        }
        (FocusedPane::Diff, KeyCode::Char('l'), _) | (FocusedPane::Diff, KeyCode::Right, _) => {
            Some(KeyAction::ScrollDiffH(DIFF_SCROLL_H_STEP))
        }
        (FocusedPane::Diff, KeyCode::PageDown, _) => Some(KeyAction::ScrollDiff(DIFF_SCROLL_PAGE)),
        (FocusedPane::Diff, KeyCode::PageUp, _) => Some(KeyAction::ScrollDiff(-DIFF_SCROLL_PAGE)),
        (FocusedPane::Diff, KeyCode::Char('g'), _) => Some(KeyAction::ScrollDiffTop),
        (FocusedPane::Diff, KeyCode::Char('G'), _) => Some(KeyAction::ScrollDiffBottom),

        // Commits
        (FocusedPane::Commits, KeyCode::Char('j'), _)
        | (FocusedPane::Commits, KeyCode::Down, _) => Some(KeyAction::CommitsMove(1)),
        (FocusedPane::Commits, KeyCode::Char('k'), _) | (FocusedPane::Commits, KeyCode::Up, _) => {
            Some(KeyAction::CommitsMove(-1))
        }
        (FocusedPane::Commits, KeyCode::Enter, _) => Some(KeyAction::CommitsToggleSelect),
        (FocusedPane::Commits, KeyCode::Char(' '), _) => Some(KeyAction::CommitsToggleExpand),

        _ => None,
    }
}
