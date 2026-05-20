use crate::app::App;
use crate::git::Blame;
use crate::model::FileChange;
use crate::ui::theme;
use ratatui::style::Style;
use ratatui::text::Span;

/// Visible width of the abbreviated commit hash in the blame gutter.
pub(super) const BLAME_HASH_W: usize = 7;
/// Visible width of the author column in the blame gutter.
pub(super) const BLAME_AUTHOR_W: usize = 12;
/// Total visible width of the blame gutter including the " │ " separator.
pub(super) const BLAME_TOTAL_W: usize = BLAME_HASH_W + 1 + BLAME_AUTHOR_W + 3;

pub fn blame_gutter_span(blame: Option<&Blame>, line_no: u32) -> Option<Span<'static>> {
    let blame = blame?;
    let style = Style::default().fg(theme::dim());
    if let Some(b) = blame.by_line.get(&line_no) {
        let author = truncate_pad(&b.author, BLAME_AUTHOR_W);
        let hash = truncate_pad(&b.short_hash, BLAME_HASH_W);
        Some(Span::styled(format!("{} {} │ ", hash, author), style))
    } else {
        Some(Span::styled(
            format!(
                "{:width$} │ ",
                "",
                width = BLAME_HASH_W + 1 + BLAME_AUTHOR_W
            ),
            style,
        ))
    }
}

fn truncate_pad(s: &str, n: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut out = String::new();
    let mut width = 0usize;
    for c in s.chars() {
        let w = UnicodeWidthChar::width(c).unwrap_or(0);
        if width + w > n {
            break;
        }
        out.push(c);
        width += w;
    }
    if width < n {
        out.push_str(&" ".repeat(n - width));
    }
    out
}

pub fn blame_for<'a>(app: &'a App, file: &FileChange) -> Option<&'a Blame> {
    if !app.blame_enabled {
        return None;
    }
    let target = app.blame_target_for(file)?;
    app.blame_cache.get(&target)
}

/// Padding span the width of the blame gutter, for rows that don't have a
/// blame entry (e.g. Del rows in unified mode). Cached at builder time so
/// callers don't reformat the empty string per row.
pub(super) fn blame_pad_span(blame: Option<&Blame>) -> Option<Span<'static>> {
    blame.map(|_| {
        Span::styled(
            format!("{:width$}", "", width = BLAME_TOTAL_W),
            Style::default().fg(theme::dim()),
        )
    })
}
