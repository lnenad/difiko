use crate::app::{App, BranchSlot, FocusedPane, Modal, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
pub enum KeyAction {
    Quit,
    ToggleHelp,
    OpenCommandPalette,

    FocusNextPane,
    FocusPrevPane,
    FocusSidebar,
    FocusDiff,
    FocusCommits,

    ToggleSidebarMode,
    ToggleCommitsPanel,
    ToggleDiffMode,
    ToggleReviewed,
    ToggleBlame,
    ClearReviewed,

    EnterFullscreen,
    ExitFullscreen,
    FullscreenNext,
    FullscreenPrev,

    OpenFileFilter,
    OpenBranchPicker(BranchSlot),

    ReloadDiff,

    ScrollDiff(i32),
    ScrollDiffH(i32),
    ScrollDiffTop,
    ScrollDiffBottom,

    SidebarMove(i32),
    SidebarTop,
    SidebarBottom,
    SidebarOpenFile,
    SidebarToggleFolder,

    NextFile,
    PrevFile,

    CommitsMove(i32),
    CommitsToggleSelect,
    CommitsToggleExpand,

    SetupNextField,
    SetupPrevField,
    SetupToggleRemote,
    SetupSubmit,
    SetupReset,
    /// Return to the Setup screen from Review without dropping branches /
    /// files / reviewed state. Lands focus on Compare for a one-keystroke
    /// change. A second Esc on Setup performs the full reset.
    BackToSetupSoft,
    SetupTextInput(char),
    SetupBackspace,
    SetupCursorLeft,
    SetupCursorRight,
    RepoCompleteAccept,
    RepoCompleteCyclePrev,
    RepoCompleteCycleNext,

    ModalClose,
    ModalAccept,
    ModalMoveUp,
    ModalMoveDown,
    ModalInputChar(char),
    ModalInputBackspace,

    DiffSearchOpen,
    DiffSearchClose,
    DiffSearchInput(char),
    DiffSearchBackspace,
    DiffSearchNext,
    DiffSearchPrev,
    DiffSearchToggleCase,
}

pub fn dispatch_key(app: &App, key: KeyEvent) -> Option<KeyAction> {
    if app.modal.is_some() {
        return modal_key(app, key);
    }
    // Diff search bar — active in Review (when Diff is focused) and Fullscreen.
    // Captures most input so typing edits the query rather than triggering
    // scroll/file shortcuts. Ctrl+C still quits.
    if app.diff_search.is_some()
        && !matches!(app.screen, Screen::Setup)
        && (matches!(app.screen, Screen::Fullscreen)
            || matches!(app.focused, crate::app::FocusedPane::Diff))
    {
        if let Some(action) = diff_search_key(key) {
            return Some(action);
        }
    }
    match app.screen {
        Screen::Setup => setup_key(app, key),
        Screen::Review => review_key(app, key),
        Screen::Fullscreen => fullscreen_key(key),
    }
}

fn diff_search_key(key: KeyEvent) -> Option<KeyAction> {
    if ctrl(&key, 'c') {
        return Some(KeyAction::Quit);
    }
    // Alt+C toggles case sensitivity (matches VS Code's find binding).
    if key.modifiers.contains(KeyModifiers::ALT) && key.code == KeyCode::Char('c') {
        return Some(KeyAction::DiffSearchToggleCase);
    }
    // Ctrl+F again jumps to next match (cycling within the open bar).
    if ctrl(&key, 'f') || ctrl(&key, 'g') || ctrl(&key, 'n') {
        return Some(KeyAction::DiffSearchNext);
    }
    if ctrl(&key, 'p') {
        return Some(KeyAction::DiffSearchPrev);
    }
    match key.code {
        KeyCode::Esc => Some(KeyAction::DiffSearchClose),
        KeyCode::Enter => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(KeyAction::DiffSearchPrev)
            } else {
                Some(KeyAction::DiffSearchNext)
            }
        }
        KeyCode::F(3) => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(KeyAction::DiffSearchPrev)
            } else {
                Some(KeyAction::DiffSearchNext)
            }
        }
        KeyCode::Backspace => Some(KeyAction::DiffSearchBackspace),
        KeyCode::Char(c) => Some(KeyAction::DiffSearchInput(c)),
        _ => None,
    }
}

fn ctrl(key: &KeyEvent, c: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(c)
}

fn modal_key(app: &App, key: KeyEvent) -> Option<KeyAction> {
    let is_overlay = matches!(app.modal, Some(Modal::HelpOverlay) | Some(Modal::Error { .. }));
    match key.code {
        KeyCode::Esc => Some(KeyAction::ModalClose),
        KeyCode::Char('q') if is_overlay => Some(KeyAction::ModalClose),
        KeyCode::Char('?') if is_overlay => Some(KeyAction::ModalClose),
        KeyCode::Enter if !is_overlay => Some(KeyAction::ModalAccept),
        KeyCode::Up => Some(KeyAction::ModalMoveUp),
        KeyCode::Down => Some(KeyAction::ModalMoveDown),
        KeyCode::Backspace if !is_overlay => Some(KeyAction::ModalInputBackspace),
        KeyCode::Char(c) if !is_overlay => {
            if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(KeyAction::Quit)
            } else {
                Some(KeyAction::ModalInputChar(c))
            }
        }
        _ => None,
    }
}

fn setup_key(app: &App, key: KeyEvent) -> Option<KeyAction> {
    use crate::app::SetupField::*;
    if ctrl(&key, 'c') {
        return Some(KeyAction::Quit);
    }
    let field = app.setup_field;
    let editing_repo = matches!(field, Repo);

    // Help, navigation, and reset always available.
    match key.code {
        KeyCode::F(1) => return Some(KeyAction::ToggleHelp),
        KeyCode::Esc => return Some(KeyAction::SetupReset),
        KeyCode::Tab => return Some(KeyAction::SetupNextField),
        KeyCode::BackTab => return Some(KeyAction::SetupPrevField),
        _ => {}
    }
    if !editing_repo {
        if let KeyCode::Char('?') = key.code {
            return Some(KeyAction::ToggleHelp);
        }
        if let KeyCode::Char('q') = key.code {
            if key.modifiers.is_empty() {
                return Some(KeyAction::Quit);
            }
        }
    }

    match field {
        Repo => match key.code {
            KeyCode::Enter => {
                if !app.repo_dropdown_hidden && !app.repo_completions.is_empty() {
                    Some(KeyAction::RepoCompleteAccept)
                } else {
                    Some(KeyAction::SetupSubmit)
                }
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(KeyAction::RepoCompleteAccept)
            }
            KeyCode::Up => Some(KeyAction::RepoCompleteCyclePrev),
            KeyCode::Down => Some(KeyAction::RepoCompleteCycleNext),
            KeyCode::Left => Some(KeyAction::SetupCursorLeft),
            KeyCode::Right => Some(KeyAction::SetupCursorRight),
            KeyCode::Backspace => Some(KeyAction::SetupBackspace),
            KeyCode::Char(c) => Some(KeyAction::SetupTextInput(c)),
            _ => None,
        },
        Base | Compare => {
            let slot = if matches!(field, Base) {
                crate::app::BranchSlot::Base
            } else {
                crate::app::BranchSlot::Compare
            };
            match key.code {
                KeyCode::Enter
                | KeyCode::Char(' ')
                | KeyCode::Char('j')
                | KeyCode::Char('k')
                | KeyCode::Down
                | KeyCode::Up => Some(KeyAction::OpenBranchPicker(slot)),
                KeyCode::Char(c) if c.is_ascii_graphic() => Some(KeyAction::OpenBranchPicker(slot)),
                _ => None,
            }
        }
        Remote => match key.code {
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('r') => {
                Some(KeyAction::SetupToggleRemote)
            }
            _ => None,
        },
        Submit => match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => Some(KeyAction::SetupSubmit),
            _ => None,
        },
    }
}

fn review_key(app: &App, key: KeyEvent) -> Option<KeyAction> {
    if ctrl(&key, 'c') {
        return Some(KeyAction::Quit);
    }
    if let Some(common) = global_key(key) {
        return Some(common);
    }
    if ctrl(&key, 'd') {
        return Some(KeyAction::ScrollDiff(10));
    }
    if ctrl(&key, 'u') {
        return Some(KeyAction::ScrollDiff(-10));
    }
    if ctrl(&key, 'f') {
        // Ctrl+F opens diff search when the diff is focused; elsewhere it
        // keeps its prior page-down meaning so users in the sidebar/commits
        // don't lose a familiar shortcut.
        if matches!(app.focused, FocusedPane::Diff) {
            return Some(KeyAction::DiffSearchOpen);
        }
        return Some(KeyAction::ScrollDiff(20));
    }
    if ctrl(&key, 'b') {
        return Some(KeyAction::ScrollDiff(-20));
    }
    if ctrl(&key, 'j') {
        return Some(KeyAction::ScrollDiff(20));
    }
    if ctrl(&key, 'k') {
        return Some(KeyAction::ScrollDiff(-20));
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Down => return Some(KeyAction::ScrollDiff(20)),
            KeyCode::Up => return Some(KeyAction::ScrollDiff(-20)),
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
        (_, KeyCode::Char('F'), _) => Some(KeyAction::EnterFullscreen),
        (_, KeyCode::Char('/'), m) if m.is_empty() => Some(KeyAction::OpenFileFilter),
        (_, KeyCode::Char('J'), _) => Some(KeyAction::NextFile),
        (_, KeyCode::Char('K'), _) => Some(KeyAction::PrevFile),

        // Sidebar
        (FocusedPane::Sidebar, KeyCode::Char('j'), _) | (FocusedPane::Sidebar, KeyCode::Down, _) => {
            Some(KeyAction::SidebarMove(1))
        }
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
            Some(KeyAction::ScrollDiffH(-8))
        }
        (FocusedPane::Diff, KeyCode::Char('l'), _) | (FocusedPane::Diff, KeyCode::Right, _) => {
            Some(KeyAction::ScrollDiffH(8))
        }
        (FocusedPane::Diff, KeyCode::PageDown, _) => Some(KeyAction::ScrollDiff(20)),
        (FocusedPane::Diff, KeyCode::PageUp, _) => Some(KeyAction::ScrollDiff(-20)),
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

fn fullscreen_key(key: KeyEvent) -> Option<KeyAction> {
    if ctrl(&key, 'c') {
        return Some(KeyAction::Quit);
    }
    // Handle q/Esc before falling through to global_key (which would treat q as Quit).
    if matches!(key.code, KeyCode::Esc) || matches!(key.code, KeyCode::Char('q') if key.modifiers.is_empty()) {
        return Some(KeyAction::ExitFullscreen);
    }
    if let Some(common) = global_key(key) {
        return Some(common);
    }
    if ctrl(&key, 'd') {
        return Some(KeyAction::ScrollDiff(10));
    }
    if ctrl(&key, 'u') {
        return Some(KeyAction::ScrollDiff(-10));
    }
    if ctrl(&key, 'f') {
        return Some(KeyAction::DiffSearchOpen);
    }
    if ctrl(&key, 'j') {
        return Some(KeyAction::ScrollDiff(20));
    }
    if ctrl(&key, 'k') {
        return Some(KeyAction::ScrollDiff(-20));
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Down => return Some(KeyAction::ScrollDiff(20)),
            KeyCode::Up => return Some(KeyAction::ScrollDiff(-20)),
            _ => {}
        }
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(KeyAction::ScrollDiff(1)),
        KeyCode::Char('k') | KeyCode::Up => Some(KeyAction::ScrollDiff(-1)),
        KeyCode::Char('h') | KeyCode::Left => Some(KeyAction::ScrollDiffH(-8)),
        KeyCode::Char('l') | KeyCode::Right => Some(KeyAction::ScrollDiffH(8)),
        KeyCode::PageDown => Some(KeyAction::ScrollDiff(20)),
        KeyCode::PageUp => Some(KeyAction::ScrollDiff(-20)),
        KeyCode::Char('g') => Some(KeyAction::ScrollDiffTop),
        KeyCode::Char('G') => Some(KeyAction::ScrollDiffBottom),
        KeyCode::Char('J') | KeyCode::Char(' ') => Some(KeyAction::FullscreenNext),
        KeyCode::Char('K') => Some(KeyAction::FullscreenPrev),
        KeyCode::Char('m') => Some(KeyAction::ToggleReviewed),
        KeyCode::Char('u') => Some(KeyAction::ToggleDiffMode),
        KeyCode::Char('b') => Some(KeyAction::ToggleBlame),
        _ => None,
    }
}

fn global_key(key: KeyEvent) -> Option<KeyAction> {
    match key.code {
        KeyCode::Char('q') if key.modifiers.is_empty() => Some(KeyAction::Quit),
        KeyCode::F(1) | KeyCode::Char('?') => Some(KeyAction::ToggleHelp),
        KeyCode::Char(':') => Some(KeyAction::OpenCommandPalette),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Modal, Picker, Screen};
    use crate::cli::Cli;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn key_mod(code: KeyCode, m: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, m)
    }

    fn empty_app() -> App {
        let cli = Cli {
            repo: None,
            base: None,
            compare: None,
            file: None,
            fullscreen: false,
            no_remote_branches: false,
            new_window: false,
        };
        App::new(&cli)
    }

    #[test]
    fn fullscreen_q_exits_not_quits() {
        let mut app = empty_app();
        app.screen = Screen::Fullscreen;
        let action = dispatch_key(&app, key(KeyCode::Char('q'))).expect("action");
        assert!(matches!(action, KeyAction::ExitFullscreen));
    }

    #[test]
    fn fullscreen_esc_exits() {
        let mut app = empty_app();
        app.screen = Screen::Fullscreen;
        let action = dispatch_key(&app, key(KeyCode::Esc)).expect("action");
        assert!(matches!(action, KeyAction::ExitFullscreen));
    }

    #[test]
    fn review_q_quits() {
        let mut app = empty_app();
        app.screen = Screen::Review;
        let action = dispatch_key(&app, key(KeyCode::Char('q'))).expect("action");
        assert!(matches!(action, KeyAction::Quit));
    }

    #[test]
    fn ctrl_c_quits_everywhere() {
        let mut app = empty_app();
        for screen in [Screen::Setup, Screen::Review, Screen::Fullscreen] {
            app.screen = screen;
            let action = dispatch_key(&app, key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL))
                .expect("action");
            assert!(
                matches!(action, KeyAction::Quit),
                "expected Quit on {:?}",
                screen
            );
        }
    }

    #[test]
    fn modal_traps_keys_before_screen() {
        let mut app = empty_app();
        app.screen = Screen::Review;
        app.modal = Some(Modal::FileFilter {
            picker: Picker::new(vec!["a".into()]),
        });
        let action = dispatch_key(&app, key(KeyCode::Char('q'))).expect("action");
        // Inside a picker modal, q is filter input, not Quit.
        assert!(matches!(action, KeyAction::ModalInputChar('q')));
    }

    #[test]
    fn setup_repo_field_q_is_typeable() {
        let mut app = empty_app();
        app.screen = Screen::Setup;
        app.setup_field = crate::app::SetupField::Repo;
        let action = dispatch_key(&app, key(KeyCode::Char('q'))).expect("action");
        assert!(matches!(action, KeyAction::SetupTextInput('q')));
    }

    #[test]
    fn setup_remote_field_q_quits() {
        let mut app = empty_app();
        app.screen = Screen::Setup;
        app.setup_field = crate::app::SetupField::Remote;
        let action = dispatch_key(&app, key(KeyCode::Char('q'))).expect("action");
        assert!(matches!(action, KeyAction::Quit));
    }

    #[test]
    fn modal_help_overlay_q_closes() {
        let mut app = empty_app();
        app.screen = Screen::Review;
        app.modal = Some(Modal::HelpOverlay);
        let action = dispatch_key(&app, key(KeyCode::Char('q'))).expect("action");
        assert!(matches!(action, KeyAction::ModalClose));
    }
}

