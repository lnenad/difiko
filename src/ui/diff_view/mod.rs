//! Diff rendering pipeline. The entry points (`render` for Review and the
//! per-mode `build_*_lines` for Fullscreen) prepare a `DiffRenderCtx` and
//! dispatch to the unified or split layout module.

mod blame;
mod layered;
mod split;
mod unified;
mod word;

use crate::app::{App, DiffMode, DiffSearch, FocusedPane};
use crate::git::Blame;
use crate::model::FileChange;
use crate::ui::theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub use blame::blame_for;
pub use blame::blame_gutter_span;
pub use split::build_split_lines;
pub use unified::build_unified_lines;
pub use word::{compute_word_pairings, ensure_syntax_cache, WordPairings};

/// Per-render bundle of optional decoration sources. Built once at the top
/// of `render()` and passed into the unified or split layout module so
/// they don't each re-derive the same lookups.
pub struct DiffRenderCtx<'a> {
    pub blame: Option<&'a Blame>,
    pub search: Option<&'a DiffSearch>,
    pub syntax: Option<&'a [crate::ui::syntax::LineHighlights]>,
    pub word_pairings: Option<&'a WordPairings>,
}

impl DiffRenderCtx<'_> {
    pub(super) fn syntax_for(&self, idx: usize) -> Option<&[(Style, String)]> {
        self.syntax.and_then(|s| s.get(idx).map(|v| v.as_slice()))
    }
    pub(super) fn word_for(&self, idx: usize) -> Option<&[(usize, usize)]> {
        self.word_pairings
            .and_then(|m| m.get(&idx).map(|v| v.as_slice()))
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focused, FocusedPane::Diff);
    let Some(file) = app.current_file() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Diff ")
            .border_style(theme::focused_border(focused));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let p =
            Paragraph::new("Select a file in the sidebar.").style(Style::default().fg(theme::DIM));
        f.render_widget(p, inner);
        return;
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::focused_border(focused))
        .title(Line::from(file_header_spans(
            file,
            app.reviewed.contains(&file.path),
        )));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let scroll = *app.diff_scroll.get(&file.path).unwrap_or(&0);
    let scroll_h = *app.diff_scroll_h.get(&file.path).unwrap_or(&0);
    let blame = blame_for(app, file);
    let search = app.diff_search.as_ref();
    let (diff_area, search_area) = split_for_search(inner, search.is_some());
    app.diff_view_height.set(diff_area.height);

    ensure_syntax_cache(app, file);
    let cache = app.syntax_cache.borrow();
    let syntax_data: Option<&[Vec<(Style, String)>]> = if app.config.syntax_highlight {
        cache.get(&file.path).map(|v| v.as_slice())
    } else {
        None
    };
    let word_pairings = if app.config.word_diff {
        Some(compute_word_pairings(&file.diff_lines))
    } else {
        None
    };
    let ctx = DiffRenderCtx {
        blame,
        search,
        syntax: syntax_data,
        word_pairings: word_pairings.as_ref(),
    };

    match app.diff_mode {
        DiffMode::Unified => unified::render_unified(f, file, scroll, scroll_h, diff_area, &ctx),
        DiffMode::Split => split::render_split(f, file, scroll, scroll_h, diff_area, &ctx),
    }
    if let (Some(area), Some(s)) = (search_area, search) {
        render_search_bar(f, area, s);
    }
}

fn split_for_search(area: Rect, active: bool) -> (Rect, Option<Rect>) {
    if !active || area.height < 2 {
        return (area, None);
    }
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    (parts[0], Some(parts[1]))
}

pub fn render_search_bar(f: &mut Frame, area: Rect, search: &DiffSearch) {
    let total = search.matches.len();
    let counter = if total == 0 {
        if search.query.buffer.is_empty() {
            String::new()
        } else {
            " [0/0]".to_string()
        }
    } else {
        format!(" [{}/{}]", search.current + 1, total)
    };
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(
            "/".to_string(),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(search.query.buffer.clone()),
        Span::styled("▏", Style::default().fg(theme::ACCENT)),
        Span::styled(counter, Style::default().fg(theme::DIM)),
    ];
    let case_label = " Aa";
    let case_style = if search.case_sensitive {
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(theme::DIM)
    };
    spans.push(Span::styled("  ".to_string(), Style::default()));
    spans.push(Span::styled(case_label.to_string(), case_style));
    spans.push(Span::styled(
        "   Enter:next  Shift-Enter:prev  Alt-c:case  Esc:close".to_string(),
        Style::default().fg(theme::DIM),
    ));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub fn file_header_spans(file: &FileChange, reviewed: bool) -> Vec<Span<'static>> {
    let status_color = theme::status_color(file.status);
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            format!("[{}] ", file.status.label()),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(file.display_name().into_owned()),
        Span::raw("  "),
        Span::styled(
            format!("+{}", file.additions),
            Style::default().fg(theme::ADD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("-{}", file.deletions),
            Style::default().fg(theme::DEL),
        ),
    ];
    if reviewed {
        spans.push(Span::styled(
            "  ✓ reviewed",
            Style::default().fg(theme::ADD).add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::raw(" "));
    spans
}
