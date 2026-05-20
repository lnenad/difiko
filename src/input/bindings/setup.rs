use super::{ctrl, KeyAction};
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn setup_key(app: &App, key: KeyEvent) -> Option<KeyAction> {
    use crate::app::SetupField::*;
    if ctrl(&key, 'c') {
        return Some(KeyAction::Quit);
    }
    // Bare-letter shortcuts (open picker, toggle remote, type into the repo
    // field) must not fire on Ctrl/Alt+letter. Shift is allowed so uppercase
    // letters still type into the repo field.
    let typing_modifiers = key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT;
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
            KeyCode::Char(c) if typing_modifiers => Some(KeyAction::SetupTextInput(c)),
            _ => None,
        },
        Base | Compare if typing_modifiers => {
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
        Base | Compare => None,
        Remote if typing_modifiers => match key.code {
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('r') => {
                Some(KeyAction::SetupToggleRemote)
            }
            _ => None,
        },
        Remote => None,
        Submit if typing_modifiers => match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => Some(KeyAction::SetupSubmit),
            _ => None,
        },
        Submit => None,
    }
}
