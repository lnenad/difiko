use ratatui::style::{Color, Modifier, Style};

pub const BG: Color = Color::Reset;
pub const FG: Color = Color::Reset;
pub const DIM: Color = Color::DarkGray;
pub const ACCENT: Color = Color::Cyan;
pub const ACCENT_DIM: Color = Color::Blue;
pub const ADD: Color = Color::Green;
pub const DEL: Color = Color::Red;
pub const HUNK: Color = Color::Magenta;
pub const STATUS_ADD: Color = Color::Green;
pub const STATUS_MOD: Color = Color::Yellow;
pub const STATUS_DEL: Color = Color::Red;
pub const STATUS_REN: Color = Color::Magenta;

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
        .bg(Color::Rgb(40, 50, 70))
        .add_modifier(Modifier::BOLD)
}

pub fn dim_highlight_style() -> Style {
    Style::default().bg(Color::Rgb(30, 35, 50))
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
