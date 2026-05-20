use crate::app::{App, DiffMode};
use crate::ui::{diff_view, hint_bar, theme};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Length(1), // breathing room
            Constraint::Min(0),    // body
            Constraint::Length(1), // hint bar
        ])
        .split(f.area());

    let total = app.files.len();
    let idx = app.fullscreen_idx;
    let Some(file) = app.files.get(idx) else {
        let p = Paragraph::new("No files to review.").style(Style::default().fg(theme::dim()));
        f.render_widget(p, outer[2]);
        hint_bar::render(f, app, outer[3]);
        return;
    };
    let reviewed = app.reviewed.contains(&file.path);

    let mut header_spans = vec![Span::raw(" ")];
    header_spans.extend(diff_view::file_header_spans(file, reviewed));
    header_spans.push(Span::styled(
        format!("  [{}/{}]", idx + 1, total),
        Style::default().fg(theme::dim()),
    ));
    if matches!(app.diff_mode, DiffMode::Split) {
        header_spans.push(Span::styled(
            "  split",
            Style::default().fg(theme::accent()),
        ));
    } else {
        header_spans.push(Span::styled(
            "  unified",
            Style::default()
                .fg(theme::dim())
                .add_modifier(Modifier::DIM),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(header_spans)), outer[0]);

    let body_full = outer[2];
    let (body_area, search_area) = if app.diff_search.is_some() && body_full.height >= 2 {
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(body_full);
        (parts[0], Some(parts[1]))
    } else {
        (body_full, None)
    };
    app.diff_view_height.set(body_area.height);
    let scroll = *app.diff_scroll.get(&file.path).unwrap_or(&0);
    let scroll_h = *app.diff_scroll_h.get(&file.path).unwrap_or(&0);
    match app.diff_mode {
        DiffMode::Unified => render_unified_inline(f, app, file, scroll, scroll_h, body_area),
        DiffMode::Split => {
            render_split_inline(f, app, file, scroll, scroll_h, body_area);
        }
    }
    if let (Some(area), Some(s)) = (search_area, app.diff_search.as_ref()) {
        diff_view::render_search_bar(f, area, s);
    }

    hint_bar::render(f, app, outer[3]);
}

fn render_unified_inline(
    f: &mut Frame,
    app: &App,
    file: &crate::model::FileChange,
    scroll: u16,
    scroll_h: u16,
    area: ratatui::layout::Rect,
) {
    diff_view::ensure_syntax_cache(app, file);
    let cache = app.syntax_cache.borrow();
    let syntax_data = if app.config.syntax_highlight {
        cache.get(&file.path).map(|v| v.as_slice())
    } else {
        None
    };
    let word_pairings = if app.config.word_diff {
        Some(diff_view::compute_word_pairings(&file.diff_lines))
    } else {
        None
    };
    let ctx = diff_view::DiffRenderCtx {
        blame: diff_view::blame_for(app, file),
        search: app.diff_search.as_ref(),
        syntax: syntax_data,
        word_pairings: word_pairings.as_ref(),
    };
    let lines = diff_view::build_unified_lines(file, &ctx);
    let p = Paragraph::new(lines).scroll((scroll, scroll_h));
    f.render_widget(p, area);
}

fn render_split_inline(
    f: &mut Frame,
    app: &App,
    file: &crate::model::FileChange,
    scroll: u16,
    scroll_h: u16,
    area: ratatui::layout::Rect,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    diff_view::ensure_syntax_cache(app, file);
    let cache = app.syntax_cache.borrow();
    let syntax_data = if app.config.syntax_highlight {
        cache.get(&file.path).map(|v| v.as_slice())
    } else {
        None
    };
    let word_pairings = if app.config.word_diff {
        Some(diff_view::compute_word_pairings(&file.diff_lines))
    } else {
        None
    };
    let ctx = diff_view::DiffRenderCtx {
        blame: diff_view::blame_for(app, file),
        search: app.diff_search.as_ref(),
        syntax: syntax_data,
        word_pairings: word_pairings.as_ref(),
    };
    let (left, right) = diff_view::build_split_lines(file, &ctx);
    let lp = Paragraph::new(left).scroll((scroll, scroll_h));
    let rp = Paragraph::new(right).scroll((scroll, scroll_h));
    f.render_widget(lp, cols[0]);
    f.render_widget(rp, cols[1]);
}
