use crate::app::{App, DiffMode, FocusedPane};
use crate::git::Blame;
use crate::model::{DiffLine, FileChange};
use crate::ui::theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const BLAME_HASH_W: usize = 7;
const BLAME_AUTHOR_W: usize = 12;
const BLAME_TOTAL_W: usize = BLAME_HASH_W + 1 + BLAME_AUTHOR_W + 3; // "hash author │ "

pub fn blame_gutter_span(blame: Option<&Blame>, line_no: u32) -> Option<Span<'static>> {
    let blame = blame?;
    let style = Style::default().fg(theme::DIM);
    if let Some(b) = blame.by_line.get(&line_no) {
        let author = truncate_pad(&b.author, BLAME_AUTHOR_W);
        let hash = truncate_pad(&b.short_hash, BLAME_HASH_W);
        Some(Span::styled(format!("{} {} │ ", hash, author), style))
    } else {
        Some(Span::styled(
            format!("{:width$} │ ", "", width = BLAME_HASH_W + 1 + BLAME_AUTHOR_W),
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

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focused, FocusedPane::Diff);
    let Some(file) = app.current_file() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Diff ")
            .border_style(theme::focused_border(focused));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let p = Paragraph::new("Select a file in the sidebar.").style(Style::default().fg(theme::DIM));
        f.render_widget(p, inner);
        return;
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::focused_border(focused))
        .title(Line::from(file_header_spans(file, app.reviewed.contains(&file.path))));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let scroll = *app.diff_scroll.get(&file.path).unwrap_or(&0);
    let scroll_h = *app.diff_scroll_h.get(&file.path).unwrap_or(&0);
    let blame = blame_for(app, file);
    match app.diff_mode {
        DiffMode::Unified => render_unified(f, file, scroll, scroll_h, inner, blame),
        DiffMode::Split => render_split(f, file, scroll, scroll_h, inner, blame),
    }
}

pub fn file_header_spans(file: &FileChange, reviewed: bool) -> Vec<Span<'static>> {
    let status_color = theme::status_color(file.status);
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(
            format!("[{}] ", file.status.label()),
            Style::default().fg(status_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(file.display_name()),
        Span::raw("  "),
        Span::styled(format!("+{}", file.additions), Style::default().fg(theme::ADD)),
        Span::raw(" "),
        Span::styled(format!("-{}", file.deletions), Style::default().fg(theme::DEL)),
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

fn render_unified(
    f: &mut Frame,
    file: &FileChange,
    scroll: u16,
    scroll_h: u16,
    area: Rect,
    blame: Option<&Blame>,
) {
    if file.binary {
        let p = Paragraph::new("Binary file — no diff to display.")
            .style(Style::default().fg(theme::DIM));
        f.render_widget(p, area);
        return;
    }
    if file.diff_lines.is_empty() {
        let p = Paragraph::new("No diff content.").style(Style::default().fg(theme::DIM));
        f.render_widget(p, area);
        return;
    }
    let lines = build_unified_lines(file, blame);
    let total = lines.len() as u16;
    let max_scroll = total.saturating_sub(area.height);
    let effective = scroll.min(max_scroll);
    // No wrap — long lines are scrolled horizontally with h/l (or arrow keys).
    let p = Paragraph::new(lines).scroll((effective, scroll_h));
    f.render_widget(p, area);
}

pub fn build_unified_lines(file: &FileChange, blame: Option<&Blame>) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(file.diff_lines.len());
    let mut old_no: u32 = 0;
    let mut new_no: u32 = 0;
    let blame_pad: Option<Span<'static>> = if blame.is_some() {
        Some(Span::styled(
            format!("{:width$}", "", width = BLAME_TOTAL_W),
            Style::default().fg(theme::DIM),
        ))
    } else {
        None
    };
    for dl in &file.diff_lines {
        match dl {
            DiffLine::Hunk { header, old_start, new_start, .. } => {
                old_no = *old_start;
                new_no = *new_start;
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(p) = &blame_pad {
                    spans.push(p.clone());
                }
                spans.push(Span::styled(
                    header.clone(),
                    Style::default().fg(theme::HUNK).add_modifier(Modifier::BOLD),
                ));
                lines.push(Line::from(spans));
            }
            DiffLine::Add(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    spans.push(s);
                }
                spans.push(Span::styled(
                    format!("{:>5} {:>5} + {}", "", new_no, text),
                    Style::default().fg(theme::ADD),
                ));
                new_no += 1;
                lines.push(Line::from(spans));
            }
            DiffLine::Del(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(p) = &blame_pad {
                    spans.push(p.clone());
                }
                spans.push(Span::styled(
                    format!("{:>5} {:>5} - {}", old_no, "", text),
                    Style::default().fg(theme::DEL),
                ));
                old_no += 1;
                lines.push(Line::from(spans));
            }
            DiffLine::Context(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    spans.push(s);
                }
                spans.push(Span::raw(format!("{:>5} {:>5}   {}", old_no, new_no, text)));
                old_no += 1;
                new_no += 1;
                lines.push(Line::from(spans));
            }
            DiffLine::NoNewline(text) => {
                lines.push(Line::from(Span::styled(text.clone(), Style::default().fg(theme::DIM))));
            }
            DiffLine::Binary(text) => {
                lines.push(Line::from(Span::styled(text.clone(), Style::default().fg(theme::DIM))));
            }
            _ => {}
        }
    }
    lines
}

fn render_split(
    f: &mut Frame,
    file: &FileChange,
    scroll: u16,
    scroll_h: u16,
    area: Rect,
    blame: Option<&Blame>,
) {
    if file.binary {
        let p = Paragraph::new("Binary file — no diff to display.")
            .style(Style::default().fg(theme::DIM));
        f.render_widget(p, area);
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let (left, right) = build_split_lines(file, blame);
    let total = left.len().max(right.len()) as u16;
    let max_scroll = total.saturating_sub(area.height);
    let effective = scroll.min(max_scroll);

    let left_p = Paragraph::new(left)
        .scroll((effective, scroll_h))
        .block(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(theme::DIM))
                .title(Span::styled(" old ", Style::default().fg(theme::DEL))),
        );
    let right_p = Paragraph::new(right)
        .scroll((effective, scroll_h))
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(Span::styled(" new ", Style::default().fg(theme::ADD))),
        );
    f.render_widget(left_p, chunks[0]);
    f.render_widget(right_p, chunks[1]);
}

/// Build a side-by-side split. Blame, when supplied, is rendered as a gutter
/// on the right (compare) column for lines that exist in the new file —
/// Context and Add lines. Del rows on the right (when no Add to pair with)
/// get a blank pad so columns stay aligned with the unified view.
pub fn build_split_lines(
    file: &FileChange,
    blame: Option<&Blame>,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let mut left: Vec<Line<'static>> = Vec::new();
    let mut right: Vec<Line<'static>> = Vec::new();
    let mut pending_del: Vec<String> = Vec::new();
    let mut pending_add: Vec<(String, u32)> = Vec::new();
    let mut new_no: u32 = 0;

    let blame_pad: Option<Span<'static>> = blame.map(|_| {
        Span::styled(
            format!("{:width$}", "", width = BLAME_TOTAL_W),
            Style::default().fg(theme::DIM),
        )
    });

    for dl in &file.diff_lines {
        match dl {
            DiffLine::Hunk { header, new_start, .. } => {
                flush_split_pending(
                    &mut pending_del,
                    &mut pending_add,
                    &mut left,
                    &mut right,
                    blame,
                    &blame_pad,
                );
                new_no = *new_start;
                let span = Span::styled(
                    header.clone(),
                    Style::default().fg(theme::HUNK).add_modifier(Modifier::BOLD),
                );
                left.push(Line::from(span.clone()));
                right.push(Line::from(span));
            }
            DiffLine::Context(text) => {
                flush_split_pending(
                    &mut pending_del,
                    &mut pending_add,
                    &mut left,
                    &mut right,
                    blame,
                    &blame_pad,
                );
                left.push(Line::from(Span::raw(format!("  {}", text))));
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    spans.push(s);
                }
                spans.push(Span::raw(format!("  {}", text)));
                right.push(Line::from(spans));
                new_no += 1;
            }
            DiffLine::Del(text) => pending_del.push(text.clone()),
            DiffLine::Add(text) => {
                pending_add.push((text.clone(), new_no));
                new_no += 1;
            }
            _ => {}
        }
    }
    flush_split_pending(
        &mut pending_del,
        &mut pending_add,
        &mut left,
        &mut right,
        blame,
        &blame_pad,
    );
    (left, right)
}

fn flush_split_pending(
    pending_del: &mut Vec<String>,
    pending_add: &mut Vec<(String, u32)>,
    left: &mut Vec<Line<'static>>,
    right: &mut Vec<Line<'static>>,
    blame: Option<&Blame>,
    blame_pad: &Option<Span<'static>>,
) {
    let n = pending_del.len().max(pending_add.len());
    for i in 0..n {
        // LEFT: del row or blank — no blame, since these lines are from the
        // base ref and aren't in the compare ref.
        if let Some(d) = pending_del.get(i) {
            left.push(Line::from(Span::styled(
                format!("- {}", d),
                Style::default().fg(theme::DEL),
            )));
        } else {
            left.push(Line::from(""));
        }
        // RIGHT: add row with blame, or blank pad.
        if let Some((a, line_no)) = pending_add.get(i) {
            let mut spans: Vec<Span<'static>> = Vec::new();
            if let Some(s) = blame_gutter_span(blame, *line_no) {
                spans.push(s);
            }
            spans.push(Span::styled(
                format!("+ {}", a),
                Style::default().fg(theme::ADD),
            ));
            right.push(Line::from(spans));
        } else if let Some(p) = blame_pad {
            right.push(Line::from(p.clone()));
        } else {
            right.push(Line::from(""));
        }
    }
    pending_del.clear();
    pending_add.clear();
}
