//! Color and style helpers for the TUI. Defaults live in the accessor
//! functions; users can override any color via `<config_dir>/theme.json`
//! (see `config.rs`). The override is loaded once at startup via
//! `init` and parked in a `OnceLock` — accessors are zero-overhead reads
//! after that.

mod config;

pub use config::{ensure_default_file, load, theme_path, Theme, ThemeLoad};

use ratatui::style::{Color, Modifier, Style};
use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

/// Install the user's theme overrides. Subsequent calls are ignored
/// (first writer wins) — call once at startup.
pub fn init(theme: Theme) {
    let _ = THEME.set(theme);
}

fn t() -> &'static Theme {
    THEME.get_or_init(Theme::default)
}

pub fn bg() -> Color {
    t().bg.unwrap_or(Color::Reset)
}
pub fn fg() -> Color {
    t().fg.unwrap_or(Color::Reset)
}
pub fn dim() -> Color {
    t().dim.unwrap_or(Color::DarkGray)
}
pub fn accent() -> Color {
    t().accent.unwrap_or(Color::Cyan)
}
pub fn accent_dim() -> Color {
    t().accent_dim.unwrap_or(Color::Blue)
}
pub fn add() -> Color {
    t().add.unwrap_or(Color::Green)
}
pub fn del() -> Color {
    t().del.unwrap_or(Color::Red)
}
pub fn hunk() -> Color {
    t().hunk.unwrap_or(Color::Magenta)
}
/// Subtle dark tints for add/del rows when syntax highlighting is on.
/// 24-bit truecolor — macOS Terminal.app users won't see the bg (it
/// doesn't support truecolor) but the green/red prefix and line numbers
/// still distinguish rows. Indexed colors weren't dark enough on
/// Windows Terminal palettes; tuning by exact Rgb is the only way to
/// stay subtle across terminals that do render truecolor.
pub fn add_bg() -> Color {
    t().add_bg.unwrap_or(Color::Rgb(15, 40, 15))
}
pub fn del_bg() -> Color {
    t().del_bg.unwrap_or(Color::Rgb(55, 15, 15))
}
/// Stronger tints painted over the changed bytes within an add/del line
/// (word-diff overlay). Same two-tier scheme delta and most modern diff
/// viewers use: subtle for the whole line, stronger for the precise edit.
pub fn add_bg_strong() -> Color {
    t().add_bg_strong.unwrap_or(Color::Rgb(30, 95, 30))
}
pub fn del_bg_strong() -> Color {
    t().del_bg_strong.unwrap_or(Color::Rgb(140, 30, 30))
}
pub fn status_add() -> Color {
    t().status_add.unwrap_or(Color::Green)
}
pub fn status_mod() -> Color {
    t().status_mod.unwrap_or(Color::Yellow)
}
pub fn status_del() -> Color {
    t().status_del.unwrap_or(Color::Red)
}
pub fn status_ren() -> Color {
    t().status_ren.unwrap_or(Color::Magenta)
}
/// Background used by `highlight_style()` / `dim_highlight_style()` and
/// the modal-picker row highlight.
pub fn highlight_bg() -> Color {
    t().highlight_bg.unwrap_or(Color::Rgb(40, 50, 70))
}
pub fn highlight_bg_dim() -> Color {
    t().highlight_bg_dim.unwrap_or(Color::Rgb(30, 35, 50))
}
/// Diff-search highlight: the *current* match overrides all other
/// styling with a strong magenta-on-white block.
pub fn search_current_bg() -> Color {
    t().search_current_bg.unwrap_or(Color::Magenta)
}
pub fn search_current_fg() -> Color {
    t().search_current_fg.unwrap_or(Color::White)
}
/// Other (non-current) matches use a softer yellow-on-black so they're
/// visible but don't compete with the active match.
pub fn search_other_bg() -> Color {
    t().search_other_bg.unwrap_or(Color::Yellow)
}
pub fn search_other_fg() -> Color {
    t().search_other_fg.unwrap_or(Color::Black)
}

pub fn focused_border(focused: bool) -> Style {
    if focused {
        Style::default().fg(accent()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(dim())
    }
}

pub fn label_style() -> Style {
    Style::default().fg(dim())
}

pub fn highlight_style() -> Style {
    Style::default()
        .bg(highlight_bg())
        .add_modifier(Modifier::BOLD)
}

pub fn dim_highlight_style() -> Style {
    Style::default().bg(highlight_bg_dim())
}

pub fn status_color(status: crate::model::FileStatus) -> Color {
    use crate::model::FileStatus::*;
    match status {
        Added => status_add(),
        Modified => status_mod(),
        Deleted => status_del(),
        Renamed | Copied => status_ren(),
        Other => dim(),
    }
}
