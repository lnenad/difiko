use crate::app::{App, DiffMode, DiffSearch, FocusedPane};
use crate::git::Blame;
use crate::model::{DiffLine, FileChange};
use crate::ui::{syntax, theme, word_diff};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::collections::HashMap;

const BLAME_HASH_W: usize = 7;
const BLAME_AUTHOR_W: usize = 12;
const BLAME_TOTAL_W: usize = BLAME_HASH_W + 1 + BLAME_AUTHOR_W + 3; // "hash author │ "

/// Word ranges keyed by `diff_lines` index. Both old- and new-side ranges
/// land here — each is byte-offset into its own line's text.
type WordPairings = HashMap<usize, Vec<(usize, usize)>>;

pub struct DiffRenderCtx<'a> {
    pub blame: Option<&'a Blame>,
    pub search: Option<&'a DiffSearch>,
    pub syntax: Option<&'a [crate::ui::syntax::LineHighlights]>,
    pub word_pairings: Option<&'a WordPairings>,
}

impl DiffRenderCtx<'_> {
    fn syntax_for(&self, idx: usize) -> Option<&[(Style, String)]> {
        self.syntax.and_then(|s| s.get(idx).map(|v| v.as_slice()))
    }
    fn word_for(&self, idx: usize) -> Option<&[(usize, usize)]> {
        self.word_pairings
            .and_then(|m| m.get(&idx).map(|v| v.as_slice()))
    }
}

pub fn blame_gutter_span(blame: Option<&Blame>, line_no: u32) -> Option<Span<'static>> {
    let blame = blame?;
    let style = Style::default().fg(theme::DIM);
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

/// Make sure `app.syntax_cache` has an entry for `file.path` when syntax
/// highlighting is enabled. Cheap no-op when already cached or disabled.
pub fn ensure_syntax_cache(app: &App, file: &FileChange) {
    if !app.config.syntax_highlight {
        return;
    }
    if app.syntax_cache.borrow().contains_key(&file.path) {
        return;
    }
    let highlights = syntax::highlight_file(file);
    app.syntax_cache
        .borrow_mut()
        .insert(file.path.clone(), highlights);
}

/// Pair adjacent runs of Del/Add lines and compute word-level changed
/// byte ranges within each line. Position-paired: the k-th Del in a run
/// pairs with the k-th Add. Unpaired lines (excess on either side) get
/// no entry — the renderer treats them as fully changed.
pub fn compute_word_pairings(diff_lines: &[DiffLine]) -> WordPairings {
    let mut out: WordPairings = HashMap::new();
    let mut i = 0;
    while i < diff_lines.len() {
        let mut dels: Vec<(usize, String)> = Vec::new();
        let mut adds: Vec<(usize, String)> = Vec::new();
        let start = i;
        while i < diff_lines.len() {
            match &diff_lines[i] {
                DiffLine::Del(t) => dels.push((i, t.clone())),
                DiffLine::Add(t) => adds.push((i, t.clone())),
                _ => break,
            }
            i += 1;
        }
        if i == start {
            i += 1;
            continue;
        }
        let n = dels.len().min(adds.len());
        for k in 0..n {
            let (di, dt) = &dels[k];
            let (ai, at) = &adds[k];
            let (or, nr) = word_diff::word_ranges(dt, at);
            out.insert(*di, or);
            out.insert(*ai, nr);
        }
    }
    out
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
        DiffMode::Unified => render_unified(f, file, scroll, scroll_h, diff_area, &ctx),
        DiffMode::Split => render_split(f, file, scroll, scroll_h, diff_area, &ctx),
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
        Span::raw(file.display_name()),
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

fn render_unified(
    f: &mut Frame,
    file: &FileChange,
    scroll: u16,
    scroll_h: u16,
    area: Rect,
    ctx: &DiffRenderCtx<'_>,
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
    let lines = build_unified_lines(file, ctx);
    let total = lines.len() as u16;
    let max_scroll = total.saturating_sub(area.height);
    let effective = scroll.min(max_scroll);
    let p = Paragraph::new(lines).scroll((effective, scroll_h));
    f.render_widget(p, area);
}

pub fn build_unified_lines(file: &FileChange, ctx: &DiffRenderCtx<'_>) -> Vec<Line<'static>> {
    let blame = ctx.blame;
    let syntax_on = ctx.syntax.is_some();
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
            DiffLine::Hunk {
                header,
                old_start,
                new_start,
                ..
            } => {
                old_no = *old_start;
                new_no = *new_start;
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(p) = &blame_pad {
                    spans.push(p.clone());
                }
                push_layered(
                    &mut spans,
                    header,
                    Style::default()
                        .fg(theme::HUNK)
                        .add_modifier(Modifier::BOLD),
                    None,
                    None,
                    line_search_matches(ctx.search, i),
                );
                lines.push(Line::from(spans));
            }
            DiffLine::Add(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    spans.push(s);
                }
                let base = add_base_style(syntax_on);
                spans.push(Span::styled(
                    format!("{:>5} {:>5} + ", "", new_no),
                    Style::default().fg(theme::ADD),
                ));
                push_layered(
                    &mut spans,
                    text,
                    base,
                    ctx.syntax_for(i),
                    ctx.word_for(i),
                    line_search_matches(ctx.search, i),
                );
                new_no += 1;
                lines.push(Line::from(spans));
            }
            DiffLine::Del(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(p) = &blame_pad {
                    spans.push(p.clone());
                }
                let base = del_base_style(syntax_on);
                spans.push(Span::styled(
                    format!("{:>5} {:>5} - ", old_no, ""),
                    Style::default().fg(theme::DEL),
                ));
                push_layered(
                    &mut spans,
                    text,
                    base,
                    ctx.syntax_for(i),
                    ctx.word_for(i),
                    line_search_matches(ctx.search, i),
                );
                old_no += 1;
                lines.push(Line::from(spans));
            }
            DiffLine::Context(text) => {
                let mut spans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    spans.push(s);
                }
                spans.push(Span::raw(format!("{:>5} {:>5}   ", old_no, new_no)));
                push_layered(
                    &mut spans,
                    text,
                    Style::default(),
                    ctx.syntax_for(i),
                    None,
                    line_search_matches(ctx.search, i),
                );
                old_no += 1;
                new_no += 1;
                lines.push(Line::from(spans));
            }
            DiffLine::NoNewline(text) => {
                lines.push(Line::from(Span::styled(
                    text.clone(),
                    Style::default().fg(theme::DIM),
                )));
            }
            DiffLine::Binary(text) => {
                lines.push(Line::from(Span::styled(
                    text.clone(),
                    Style::default().fg(theme::DIM),
                )));
            }
            _ => {}
        }
    }
    lines
}

fn add_base_style(syntax_on: bool) -> Style {
    if syntax_on {
        // Syntax owns the fg; ADD_BG is tuned subtle (very dark green) so
        // it differentiates the row without fighting syntax colors.
        Style::default().bg(theme::ADD_BG)
    } else {
        Style::default().fg(theme::ADD)
    }
}

fn del_base_style(syntax_on: bool) -> Style {
    if syntax_on {
        Style::default().bg(theme::DEL_BG)
    } else {
        Style::default().fg(theme::DEL)
    }
}

fn line_search_matches(search: Option<&DiffSearch>, line_idx: usize) -> Vec<(usize, usize, bool)> {
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
            .bg(Color::Magenta)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }
}

/// Emit styled spans for one diff line's content, layering syntax, word
/// diff, and search highlighting over `base_style`. All inputs are byte
/// offsets into `text`; non-char-boundary offsets are skipped silently.
fn push_layered(
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

fn render_split(
    f: &mut Frame,
    file: &FileChange,
    scroll: u16,
    scroll_h: u16,
    area: Rect,
    ctx: &DiffRenderCtx<'_>,
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

    let (left, right) = build_split_lines(file, ctx);
    let total = left.len().max(right.len()) as u16;
    let max_scroll = total.saturating_sub(area.height);
    let effective = scroll.min(max_scroll);

    let left_p = Paragraph::new(left).scroll((effective, scroll_h)).block(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme::DIM))
            .title(Span::styled(" old ", Style::default().fg(theme::DEL))),
    );
    let right_p = Paragraph::new(right).scroll((effective, scroll_h)).block(
        Block::default()
            .borders(Borders::NONE)
            .title(Span::styled(" new ", Style::default().fg(theme::ADD))),
    );
    f.render_widget(left_p, chunks[0]);
    f.render_widget(right_p, chunks[1]);
}

pub fn build_split_lines(
    file: &FileChange,
    ctx: &DiffRenderCtx<'_>,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let blame = ctx.blame;
    let syntax_on = ctx.syntax.is_some();
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
            DiffLine::Hunk {
                header, new_start, ..
            } => {
                flush_split_pending(
                    &mut pending_del,
                    &mut pending_add,
                    &mut left,
                    &mut right,
                    blame,
                    &blame_pad,
                    ctx,
                    syntax_on,
                );
                new_no = *new_start;
                let hunk_style = Style::default()
                    .fg(theme::HUNK)
                    .add_modifier(Modifier::BOLD);
                let mut lspans: Vec<Span<'static>> = Vec::new();
                push_layered(
                    &mut lspans,
                    header,
                    hunk_style,
                    None,
                    None,
                    line_search_matches(ctx.search, i),
                );
                let mut rspans: Vec<Span<'static>> = Vec::new();
                push_layered(
                    &mut rspans,
                    header,
                    hunk_style,
                    None,
                    None,
                    line_search_matches(ctx.search, i),
                );
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
                    ctx,
                    syntax_on,
                );
                let mut lspans: Vec<Span<'static>> = Vec::new();
                lspans.push(Span::raw("  "));
                push_layered(
                    &mut lspans,
                    text,
                    Style::default(),
                    ctx.syntax_for(i),
                    None,
                    line_search_matches(ctx.search, i),
                );
                left.push(Line::from(lspans));
                let mut rspans: Vec<Span<'static>> = Vec::new();
                if let Some(s) = blame_gutter_span(blame, new_no) {
                    rspans.push(s);
                }
                rspans.push(Span::raw("  "));
                push_layered(
                    &mut rspans,
                    text,
                    Style::default(),
                    ctx.syntax_for(i),
                    None,
                    line_search_matches(ctx.search, i),
                );
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
        ctx,
        syntax_on,
    );
    (left, right)
}

#[allow(clippy::too_many_arguments)]
fn flush_split_pending(
    pending_del: &mut Vec<(String, usize)>,
    pending_add: &mut Vec<(String, u32, usize)>,
    left: &mut Vec<Line<'static>>,
    right: &mut Vec<Line<'static>>,
    blame: Option<&Blame>,
    blame_pad: &Option<Span<'static>>,
    ctx: &DiffRenderCtx<'_>,
    syntax_on: bool,
) {
    let n = pending_del.len().max(pending_add.len());
    for i in 0..n {
        if let Some((d, line_idx)) = pending_del.get(i) {
            let base = del_base_style(syntax_on);
            let mut spans: Vec<Span<'static>> = Vec::new();
            spans.push(Span::styled(
                "- ".to_string(),
                Style::default().fg(theme::DEL),
            ));
            push_layered(
                &mut spans,
                d,
                base,
                ctx.syntax_for(*line_idx),
                ctx.word_for(*line_idx),
                line_search_matches(ctx.search, *line_idx),
            );
            left.push(Line::from(spans));
        } else {
            left.push(Line::from(""));
        }
        if let Some((a, line_no, line_idx)) = pending_add.get(i) {
            let mut spans: Vec<Span<'static>> = Vec::new();
            if let Some(s) = blame_gutter_span(blame, *line_no) {
                spans.push(s);
            }
            let base = add_base_style(syntax_on);
            spans.push(Span::styled(
                "+ ".to_string(),
                Style::default().fg(theme::ADD),
            ));
            push_layered(
                &mut spans,
                a,
                base,
                ctx.syntax_for(*line_idx),
                ctx.word_for(*line_idx),
                line_search_matches(ctx.search, *line_idx),
            );
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
