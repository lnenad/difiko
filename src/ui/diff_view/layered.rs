use crate::app::DiffSearch;
use crate::ui::theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

pub(super) fn add_base_style(syntax_on: bool) -> Style {
    if syntax_on {
        // Syntax owns the fg; ADD_BG is tuned subtle (very dark green) so
        // it differentiates the row without fighting syntax colors.
        Style::default().bg(theme::ADD_BG)
    } else {
        Style::default().fg(theme::ADD)
    }
}

pub(super) fn del_base_style(syntax_on: bool) -> Style {
    if syntax_on {
        Style::default().bg(theme::DEL_BG)
    } else {
        Style::default().fg(theme::DEL)
    }
}

pub(super) fn line_search_matches(
    search: Option<&DiffSearch>,
    line_idx: usize,
) -> Vec<(usize, usize, bool)> {
    let Some(s) = search else {
        return Vec::new();
    };
    let current = s.current;
    s.matches
        .iter()
        .enumerate()
        .filter(|(_, m)| m.line == line_idx)
        .map(|(i, m)| (m.start, m.end, i == current))
        .collect()
}

fn search_match_style(is_current: bool) -> Style {
    if is_current {
        Style::default()
            .bg(theme::SEARCH_CURRENT_BG)
            .fg(theme::SEARCH_CURRENT_FG)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default()
            .bg(theme::SEARCH_OTHER_BG)
            .fg(theme::SEARCH_OTHER_FG)
            .add_modifier(Modifier::BOLD)
    }
}

/// Emit styled spans for one diff line's content, layering syntax, word
/// diff, and search highlighting over `base_style`. All inputs are byte
/// offsets into `text`; non-char-boundary offsets are skipped silently.
///
/// Layer precedence (lowest → highest priority):
///   1. `base_style`             — add/del row bg or context fg
///   2. `syntax_segments`        — patched fg/modifiers from syntect
///   3. `word_changed`           — stronger bg over precise changed bytes
///   4. `search_matches`         — overrides everything when fully inside
///
/// Order is load-bearing: search must win over syntax+word-diff so the
/// active match is unmistakable, and word-diff must paint over syntax so
/// the user can see *what* changed even on themed code.
pub(super) fn push_layered(
    spans: &mut Vec<Span<'static>>,
    text: &str,
    base_style: Style,
    syntax_segments: Option<&[(Style, String)]>,
    word_changed: Option<&[(usize, usize)]>,
    search_matches: Vec<(usize, usize, bool)>,
) {
    if text.is_empty() {
        return;
    }
    let word_has_changes = word_changed.map(|r| !r.is_empty()).unwrap_or(false);

    // Pre-resolve the syntax segment byte range table.
    let syntax_byte_styles: Vec<(usize, usize, Style)> = match syntax_segments {
        Some(segs) => {
            let mut out = Vec::with_capacity(segs.len());
            let mut acc = 0usize;
            for (st, s) in segs {
                let end = (acc + s.len()).min(text.len());
                if acc < end {
                    out.push((acc, end, *st));
                }
                acc += s.len();
                if acc >= text.len() {
                    break;
                }
            }
            out
        }
        None => Vec::new(),
    };

    // Boundary points.
    let mut boundaries: Vec<usize> = vec![0, text.len()];
    for (_, e, _) in &syntax_byte_styles {
        boundaries.push(*e);
    }
    if let Some(ranges) = word_changed {
        for (s, e) in ranges {
            boundaries.push(*s);
            boundaries.push(*e);
        }
    }
    for (s, e, _) in &search_matches {
        boundaries.push(*s);
        boundaries.push(*e);
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries.retain(|b| *b <= text.len() && text.is_char_boundary(*b));

    for w in boundaries.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a == b {
            continue;
        }
        let slice = &text[a..b];
        let style = resolve_style(
            a,
            b,
            base_style,
            &syntax_byte_styles,
            word_changed,
            word_has_changes,
            &search_matches,
        );
        spans.push(Span::styled(slice.to_string(), style));
    }
}

fn resolve_style(
    a: usize,
    b: usize,
    base: Style,
    syntax_byte_styles: &[(usize, usize, Style)],
    word_changed: Option<&[(usize, usize)]>,
    word_has_changes: bool,
    search_matches: &[(usize, usize, bool)],
) -> Style {
    // Search overrides everything else when the segment is fully inside a match.
    for (s, e, is_current) in search_matches {
        if a >= *s && b <= *e {
            return search_match_style(*is_current);
        }
    }
    let mut style = base;
    for (s, e, syn) in syntax_byte_styles {
        if a >= *s && b <= *e {
            style = style.patch(*syn);
            break;
        }
    }
    if word_has_changes {
        let changed_ranges = word_changed.unwrap();
        let in_changed = changed_ranges.iter().any(|(s, e)| a >= *s && b <= *e);
        if in_changed {
            // Paint a stronger bg over the precise changed bytes. Mirrors
            // the two-tier look used by delta / Claude Code / GitHub:
            // subtle line tint + bright word tint. Bold helps when the
            // terminal doesn't render the stronger bg (Terminal.app).
            let stronger = if base.bg == Some(theme::ADD_BG) {
                Some(theme::ADD_BG_STRONG)
            } else if base.bg == Some(theme::DEL_BG) {
                Some(theme::DEL_BG_STRONG)
            } else {
                None
            };
            if let Some(bg) = stronger {
                style = style.bg(bg);
            }
            style = style.add_modifier(Modifier::BOLD);
        }
    }
    style
}
