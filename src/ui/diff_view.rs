use crate::app::{App, DiffMode, DiffSearch, FocusedPane};
use crate::git::Blame;
use crate::model::{DiffLine, FileChange};
use crate::ui::theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
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
    let search = app.diff_search.as_ref();
    let (diff_area, search_area) = split_for_search(inner, search.is_some());
    app.diff_view_height.set(diff_area.height);
    match app.diff_mode {
        DiffMode::Unified => render_unified(f, file, scroll, scroll_h, diff_area, blame, search),
        DiffMode::Split => render_split(f, file, scroll, scroll_h, diff_area, blame, search),
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
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(
        "/".to_string(),
        Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw(search.query.buffer.clone()));
    spans.push(Span::styled("▏", Style::default().fg(theme::ACCENT)));
    spans.push(Span::styled(counter, Style::default().fg(theme::DIM)));
    // Case-sensitivity indicator: bright when active, dim when off.
    let case_label = " Aa";
    let case_style = if search.case_sensitive {
        Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED)
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
    search: Option<&DiffSearch>,
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
    let lines = build_unified_lines(file, blame, search);
    let total = lines.len() as u16;
    let max_scroll = total.saturating_sub(area.height);
    let effective = scroll.min(max_scroll);
    // No wrap — long lines are scrolled horizontally with h/l (or arrow keys).
    let p = Paragraph::new(lines).scroll((effective, scroll_h));
    f.render_widget(p, area);
}

pub fn build_unified_lines(
    file: &FileChange,
    blame: Option<&Blame>,
    search: Option<&DiffSearch>,
) -> Vec<Line<'static>> {
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
    for (i, dl) in file.diff_lines.iter().enumerate() {
        match dl {
            DiffLine::Hunk { header, old_start, new_start, .. } => {
                old_no = *old_start;
                new_no = *new_start;
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(p) = &blame_pad {
                    spans.push(p.clone());
                }
                push_content_spans(
                    &mut spans,
                    header,
                    Style::default().fg(theme::HUNK).add_modifier(Modifier::BOLD),
                    i,
                    search,
                );
                lines.push(Line::from(spans));
            }
            DiffLine::Add(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    spans.push(s);
                }
                let style = Style::default().fg(theme::ADD);
                spans.push(Span::styled(format!("{:>5} {:>5} + ", "", new_no), style));
                push_content_spans(&mut spans, text, style, i, search);
                new_no += 1;
                lines.push(Line::from(spans));
            }
            DiffLine::Del(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(p) = &blame_pad {
                    spans.push(p.clone());
                }
                let style = Style::default().fg(theme::DEL);
                spans.push(Span::styled(format!("{:>5} {:>5} - ", old_no, ""), style));
                push_content_spans(&mut spans, text, style, i, search);
                old_no += 1;
                lines.push(Line::from(spans));
            }
            DiffLine::Context(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    spans.push(s);
                }
                spans.push(Span::raw(format!("{:>5} {:>5}   ", old_no, new_no)));
                push_content_spans(&mut spans, text, Style::default(), i, search);
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

fn search_match_style(is_current: bool) -> Style {
    if is_current {
        // High-contrast hot magenta for the active match — pops against the
        // green/red/default diff text plus the other (yellow) matches.
        Style::default()
            .bg(Color::Rgb(220, 0, 160))
            .fg(Color::Rgb(255, 255, 255))
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }
}

/// Push the content text for one diff line, splitting around any active
/// search matches so they render with a highlight background. Without search,
/// it's a single styled span — same shape as before.
fn push_content_spans(
    spans: &mut Vec<Span<'static>>,
    text: &str,
    base_style: Style,
    line_idx: usize,
    search: Option<&DiffSearch>,
) {
    let Some(search) = search else {
        spans.push(Span::styled(text.to_string(), base_style));
        return;
    };
    let current_idx = search.current;
    let line_matches: Vec<(usize, usize, bool)> = search
        .matches
        .iter()
        .enumerate()
        .filter(|(_, m)| m.line == line_idx)
        .map(|(i, m)| (m.start, m.end, i == current_idx))
        .collect();
    if line_matches.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
        return;
    }
    let mut cursor = 0;
    for (s, e, is_current) in line_matches {
        let s = s.min(text.len());
        let e = e.min(text.len()).max(s);
        if !text.is_char_boundary(s) || !text.is_char_boundary(e) {
            continue;
        }
        if s > cursor {
            spans.push(Span::styled(text[cursor..s].to_string(), base_style));
        }
        spans.push(Span::styled(
            text[s..e].to_string(),
            search_match_style(is_current),
        ));
        cursor = e;
    }
    if cursor < text.len() {
        spans.push(Span::styled(text[cursor..].to_string(), base_style));
    }
}

fn render_split(
    f: &mut Frame,
    file: &FileChange,
    scroll: u16,
    scroll_h: u16,
    area: Rect,
    blame: Option<&Blame>,
    search: Option<&DiffSearch>,
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

    let (left, right) = build_split_lines(file, blame, search);
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
    search: Option<&DiffSearch>,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let mut left: Vec<Line<'static>> = Vec::new();
    let mut right: Vec<Line<'static>> = Vec::new();
    let mut pending_del: Vec<(String, usize)> = Vec::new();
    let mut pending_add: Vec<(String, u32, usize)> = Vec::new();
    let mut new_no: u32 = 0;

    let blame_pad: Option<Span<'static>> = blame.map(|_| {
        Span::styled(
            format!("{:width$}", "", width = BLAME_TOTAL_W),
            Style::default().fg(theme::DIM),
        )
    });

    for (i, dl) in file.diff_lines.iter().enumerate() {
        match dl {
            DiffLine::Hunk { header, new_start, .. } => {
                flush_split_pending(
                    &mut pending_del,
                    &mut pending_add,
                    &mut left,
                    &mut right,
                    blame,
                    &blame_pad,
                    search,
                );
                new_no = *new_start;
                let hunk_style = Style::default().fg(theme::HUNK).add_modifier(Modifier::BOLD);
                let mut lspans: Vec<Span<'static>> = Vec::new();
                push_content_spans(&mut lspans, header, hunk_style, i, search);
                let mut rspans: Vec<Span<'static>> = Vec::new();
                push_content_spans(&mut rspans, header, hunk_style, i, search);
                left.push(Line::from(lspans));
                right.push(Line::from(rspans));
            }
            DiffLine::Context(text) => {
                flush_split_pending(
                    &mut pending_del,
                    &mut pending_add,
                    &mut left,
                    &mut right,
                    blame,
                    &blame_pad,
                    search,
                );
                let mut lspans: Vec<Span<'static>> = Vec::new();
                lspans.push(Span::raw("  "));
                push_content_spans(&mut lspans, text, Style::default(), i, search);
                left.push(Line::from(lspans));
                let mut rspans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    rspans.push(s);
                }
                rspans.push(Span::raw("  "));
                push_content_spans(&mut rspans, text, Style::default(), i, search);
                right.push(Line::from(rspans));
                new_no += 1;
            }
            DiffLine::Del(text) => pending_del.push((text.clone(), i)),
            DiffLine::Add(text) => {
                pending_add.push((text.clone(), new_no, i));
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
        search,
    );
    (left, right)
}

fn flush_split_pending(
    pending_del: &mut Vec<(String, usize)>,
    pending_add: &mut Vec<(String, u32, usize)>,
    left: &mut Vec<Line<'static>>,
    right: &mut Vec<Line<'static>>,
    blame: Option<&Blame>,
    blame_pad: &Option<Span<'static>>,
    search: Option<&DiffSearch>,
) {
    let n = pending_del.len().max(pending_add.len());
    for i in 0..n {
        // LEFT: del row or blank — no blame, since these lines are from the
        // base ref and aren't in the compare ref.
        if let Some((d, line_idx)) = pending_del.get(i) {
            let style = Style::default().fg(theme::DEL);
            let mut spans: Vec<Span<'static>> = Vec::new();
            spans.push(Span::styled("- ".to_string(), style));
            push_content_spans(&mut spans, d, style, *line_idx, search);
            left.push(Line::from(spans));
        } else {
            left.push(Line::from(""));
        }
        // RIGHT: add row with blame, or blank pad.
        if let Some((a, line_no, line_idx)) = pending_add.get(i) {
            let mut spans: Vec<Span<'static>> = Vec::new();
            if let Some(s) = blame_gutter_span(blame, *line_no) {
                spans.push(s);
            }
            let style = Style::default().fg(theme::ADD);
            spans.push(Span::styled("+ ".to_string(), style));
            push_content_spans(&mut spans, a, style, *line_idx, search);
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
