use ratatui::style::{Color, Modifier, Style};

pub const BG: Color = Color::Reset;
pub const FG: Color = Color::Reset;
pub const DIM: Color = Color::DarkGray;
pub const ACCENT: Color = Color::Cyan;
pub const ACCENT_DIM: Color = Color::Blue;
pub const ADD: Color = Color::Green;
pub const DEL: Color = Color::Red;
pub const HUNK: Color = Color::Magenta;
/// Subtle dark tints for add/del rows when syntax highlighting is on.
/// 24-bit truecolor — macOS Terminal.app users won't see the bg (it
/// doesn't support truecolor) but the green/red prefix and line numbers
/// still distinguish rows. Indexed colors weren't dark enough on
/// Windows Terminal palettes; tuning by exact Rgb is the only way to
/// stay subtle across terminals that do render truecolor.
pub const ADD_BG: Color = Color::Rgb(15, 40, 15);
pub const DEL_BG: Color = Color::Rgb(55, 15, 15);
/// Stronger tints painted over the changed bytes within an add/del line
/// (word-diff overlay). Same two-tier scheme Claude Code, delta, and
/// most modern diff viewers use: subtle for the whole line, stronger
/// for the precise edit.
pub const ADD_BG_STRONG: Color = Color::Rgb(30, 95, 30);
pub const DEL_BG_STRONG: Color = Color::Rgb(140, 30, 30);
pub const STATUS_ADD: Color = Color::Green;
pub const STATUS_MOD: Color = Color::Yellow;
pub const STATUS_DEL: Color = Color::Red;
pub const STATUS_REN: Color = Color::Magenta;

/// Background used by `highlight_style()` / `dim_highlight_style()` and the
/// modal-picker row highlight. Same blueish tint reused in multiple places.
pub const HIGHLIGHT_BG: Color = Color::Rgb(40, 50, 70);
pub const HIGHLIGHT_BG_DIM: Color = Color::Rgb(30, 35, 50);

/// Diff-search highlight: the *current* match overrides all other styling
/// (syntax / word-diff / base) with a strong magenta-on-white block.
pub const SEARCH_CURRENT_BG: Color = Color::Magenta;
pub const SEARCH_CURRENT_FG: Color = Color::White;
/// Other (non-current) matches use a softer yellow-on-black so they're
/// visible but don't compete with the active match.
pub const SEARCH_OTHER_BG: Color = Color::Yellow;
pub const SEARCH_OTHER_FG: Color = Color::Black;

pub fn focused_border(focused: bool) -> Style {
    if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM)
    }
}

pub fn label_style() -> Style {
    Style::default().fg(DIM)
}

pub fn highlight_style() -> Style {
    Style::default()
        .bg(HIGHLIGHT_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn dim_highlight_style() -> Style {
    Style::default().bg(HIGHLIGHT_BG_DIM)
}

pub fn status_color(status: crate::model::FileStatus) -> Color {
    use crate::model::FileStatus::*;
    match status {
        Added => STATUS_ADD,
        Modified => STATUS_MOD,
        Deleted => STATUS_DEL,
        Renamed | Copied => STATUS_REN,
        Other => DIM,
    }
}
