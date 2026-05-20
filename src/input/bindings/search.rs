use super::{ctrl, KeyAction};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn diff_search_key(key: KeyEvent) -> Option<KeyAction> {
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
