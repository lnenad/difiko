use super::blame::{blame_gutter_span, blame_pad_span};
use super::layered::{add_base_style, del_base_style, line_search_matches, push_layered};
use super::DiffRenderCtx;
use crate::git::Blame;
use crate::model::{DiffLine, FileChange};
use crate::ui::theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub(super) fn render_split(
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

    let blame_pad = blame_pad_span(blame);

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
