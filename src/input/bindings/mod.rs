use crate::app::{App, BranchSlot, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod fullscreen;
mod modal;
mod review;
mod search;
mod setup;

/// Half-page diff scroll for Ctrl+D / Ctrl+U.
pub(super) const DIFF_SCROLL_HALF: i32 = 10;
/// Full page diff scroll for Ctrl+F/B/J/K and PgUp/PgDn.
pub(super) const DIFF_SCROLL_PAGE: i32 = 20;
/// Horizontal scroll step for h/l / arrow keys in the diff pane.
pub(super) const DIFF_SCROLL_H_STEP: i32 = 8;

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
    ToggleWordDiff,
    ToggleSyntaxHighlight,
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
        return modal::modal_key(app, key);
    }
    // Diff search bar — active in Review (when Diff is focused) and Fullscreen.
    // Captures most input so typing edits the query rather than triggering
    // scroll/file shortcuts. Ctrl+C still quits.
    if app.diff_search.is_some()
        && !matches!(app.screen, Screen::Setup)
        && (matches!(app.screen, Screen::Fullscreen)
            || matches!(app.focused, crate::app::FocusedPane::Diff))
    {
        if let Some(action) = search::diff_search_key(key) {
            return Some(action);
        }
    }
    match app.screen {
        Screen::Setup => setup::setup_key(app, key),
        Screen::Review => review::review_key(app, key),
        Screen::Fullscreen => fullscreen::fullscreen_key(key),
    }
}

/// True when `key` is `Ctrl + c` (case-insensitive on the char). Used by
/// every per-screen handler since Ctrl+C must always quit.
pub(super) fn ctrl(key: &KeyEvent, c: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(c)
}

/// Keys shared across Review and Fullscreen (and never typed into a field):
/// `q` quits, `?`/F1 toggles help, `:` opens the command palette.
pub(super) fn global_key(key: KeyEvent) -> Option<KeyAction> {
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
            no_word_diff: false,
            no_syntax: false,
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
