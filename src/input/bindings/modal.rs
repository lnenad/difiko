use super::KeyAction;
use crate::app::{App, Modal};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn modal_key(app: &App, key: KeyEvent) -> Option<KeyAction> {
    let is_overlay = matches!(
        app.modal,
        Some(Modal::HelpOverlay) | Some(Modal::Error { .. })
    );
    match key.code {
        KeyCode::Esc => Some(KeyAction::ModalClose),
        KeyCode::Char('q') if is_overlay => Some(KeyAction::ModalClose),
        KeyCode::Char('?') if is_overlay => Some(KeyAction::ModalClose),
        KeyCode::Enter if !is_overlay => Some(KeyAction::ModalAccept),
        KeyCode::Up => Some(KeyAction::ModalMoveUp),
        KeyCode::Down => Some(KeyAction::ModalMoveDown),
        KeyCode::Backspace if !is_overlay => Some(KeyAction::ModalInputBackspace),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(KeyAction::Quit)
        }
        // Only bare/Shifted chars type into the picker query. Ctrl/Alt
        // combos are ignored here so they don't insert stray letters
        // (e.g. Ctrl+T should NOT type 't' into the filter).
        KeyCode::Char(c)
            if !is_overlay
                && (key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT) =>
        {
            Some(KeyAction::ModalInputChar(c))
        }
        _ => None,
    }
}
