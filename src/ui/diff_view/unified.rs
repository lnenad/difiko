use super::blame::{blame_gutter_span, blame_pad_span};
use super::layered::{add_base_style, del_base_style, line_search_matches, push_layered};
use super::DiffRenderCtx;
use crate::model::{DiffLine, FileChange};
use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub(super) fn render_unified(
    f: &mut Frame,
    file: &FileChange,
    scroll: u16,
    scroll_h: u16,
    area: Rect,
    ctx: &DiffRenderCtx<'_>,
) {
    if file.binary {
        let p = Paragraph::new("Binary file — no diff to display.")
            .style(Style::default().fg(theme::dim()));
        f.render_widget(p, area);
        return;
    }
    if file.diff_lines.is_empty() {
        let p = Paragraph::new("No diff content.").style(Style::default().fg(theme::dim()));
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
    let blame_pad = blame_pad_span(blame);
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
                        .fg(theme::hunk())
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
                    Style::default().fg(theme::add()),
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
                    Style::default().fg(theme::del()),
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
                    Style::default().fg(theme::dim()),
                )));
            }
            DiffLine::Binary(text) => {
                lines.push(Line::from(Span::styled(
                    text.clone(),
                    Style::default().fg(theme::dim()),
                )));
            }
            _ => {}
        }
    }
    lines
}
