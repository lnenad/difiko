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
