//! Syntect-backed syntax highlighting for diff content.
//!
//! Highlights are produced by running each file's diff content (in order)
//! through a single `HighlightLines` state so multi-line constructs (block
//! comments, strings) keep context across lines. Output is cached on
//! `App.syntax_cache` keyed by file path; the cache is invalidated when a
//! new diff loads or the user toggles the feature off.

use crate::model::{DiffLine, FileChange};
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

/// One styled-segment list per `file.diff_lines` entry. Non-textual diff
/// lines (headers, hunk markers, binary, NoNewline) get an empty Vec.
pub type LineHighlights = Vec<(Style, String)>;
pub type FileHighlights = HashMap<String, Vec<LineHighlights>>;

struct Engine {
    syntax_set: SyntaxSet,
    theme: Theme,
}

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        // "base16-eighties.dark" reads well over both default and dark
        // terminal backgrounds and ships with syntect's default theme set.
        let theme = theme_set
            .themes
            .get("base16-eighties.dark")
            .cloned()
            .unwrap_or_else(|| theme_set.themes.values().next().cloned().unwrap());
        Engine { syntax_set, theme }
    })
}

/// Returns one `Vec<(Style, String)>` per `file.diff_lines` entry. Entries
/// that aren't textual content (GitHeader, IndexHeader, OldFile, NewFile,
/// Hunk, Binary, NoNewline) get an empty segment list — the renderer falls
/// back to its default spans for them.
pub fn highlight_file(file: &FileChange) -> Vec<LineHighlights> {
    let eng = engine();
    let syntax = eng
        .syntax_set
        .find_syntax_for_file(&file.path)
        .ok()
        .flatten()
        .or_else(|| {
            std::path::Path::new(&file.path)
                .extension()
                .and_then(|e| e.to_str())
                .and_then(|e| eng.syntax_set.find_syntax_by_extension(e))
        })
        .unwrap_or_else(|| eng.syntax_set.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, &eng.theme);

    let mut out: Vec<LineHighlights> = Vec::with_capacity(file.diff_lines.len());
    for dl in &file.diff_lines {
        let text: Option<&str> = match dl {
            DiffLine::Add(t) | DiffLine::Del(t) | DiffLine::Context(t) => Some(t.as_str()),
            _ => None,
        };
        if let Some(line) = text {
            // syntect expects \n-terminated input for accurate state tracking.
            let with_newline = format!("{}\n", line);
            let segments = h
                .highlight_line(&with_newline, &eng.syntax_set)
                .map(|v| {
                    v.into_iter()
                        .map(|(sty, s)| {
                            // Strip the trailing newline we added.
                            let s = s.strip_suffix('\n').unwrap_or(s);
                            (to_ratatui(sty), s.to_string())
                        })
                        .filter(|(_, s)| !s.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|_| vec![(Style::default(), line.to_string())]);
            out.push(segments);
        } else {
            out.push(Vec::new());
        }
    }
    out
}

fn to_ratatui(s: SyntectStyle) -> Style {
    // We deliberately ignore syntect's background — it's a theme-level dark
    // background that fights with the diff add/del backgrounds we apply at
    // the line level. Foreground + font style only.
    let fg = Color::Rgb(s.foreground.r, s.foreground.g, s.foreground.b);
    let mut style = Style::default().fg(fg);
    if s.font_style.contains(FontStyle::BOLD) {
        style = style.add_modifier(Modifier::BOLD);
    }
    if s.font_style.contains(FontStyle::ITALIC) {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if s.font_style.contains(FontStyle::UNDERLINE) {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    style
}
