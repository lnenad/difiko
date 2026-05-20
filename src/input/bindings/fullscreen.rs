use super::{ctrl, global_key, KeyAction, DIFF_SCROLL_HALF, DIFF_SCROLL_H_STEP, DIFF_SCROLL_PAGE};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn fullscreen_key(key: KeyEvent) -> Option<KeyAction> {
    if ctrl(&key, 'c') {
        return Some(KeyAction::Quit);
    }
    // Handle q/Esc before falling through to global_key (which would treat q as Quit).
    if matches!(key.code, KeyCode::Esc)
        || matches!(key.code, KeyCode::Char('q') if key.modifiers.is_empty())
    {
        return Some(KeyAction::ExitFullscreen);
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
        return Some(KeyAction::DiffSearchOpen);
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
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(KeyAction::ScrollDiff(1)),
        KeyCode::Char('k') | KeyCode::Up => Some(KeyAction::ScrollDiff(-1)),
        KeyCode::Char('h') | KeyCode::Left => Some(KeyAction::ScrollDiffH(-DIFF_SCROLL_H_STEP)),
        KeyCode::Char('l') | KeyCode::Right => Some(KeyAction::ScrollDiffH(DIFF_SCROLL_H_STEP)),
        KeyCode::PageDown => Some(KeyAction::ScrollDiff(DIFF_SCROLL_PAGE)),
        KeyCode::PageUp => Some(KeyAction::ScrollDiff(-DIFF_SCROLL_PAGE)),
        KeyCode::Char('g') => Some(KeyAction::ScrollDiffTop),
        KeyCode::Char('G') => Some(KeyAction::ScrollDiffBottom),
        KeyCode::Char('J') | KeyCode::Char(' ') => Some(KeyAction::FullscreenNext),
        KeyCode::Char('K') => Some(KeyAction::FullscreenPrev),
        KeyCode::Char('m') => Some(KeyAction::ToggleReviewed),
        KeyCode::Char('u') => Some(KeyAction::ToggleDiffMode),
        KeyCode::Char('b') => Some(KeyAction::ToggleBlame),
        KeyCode::Char('W') => Some(KeyAction::ToggleWordDiff),
        KeyCode::Char('S') => Some(KeyAction::ToggleSyntaxHighlight),
        _ => None,
    }
}
