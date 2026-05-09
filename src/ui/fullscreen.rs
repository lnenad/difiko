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
        let p = Paragraph::new("No files to review.").style(Style::default().fg(theme::DIM));
        f.render_widget(p, outer[2]);
        hint_bar::render(f, app, outer[3]);
        return;
    };
    let reviewed = app.reviewed.contains(&file.path);

    let mut header_spans = vec![Span::raw(" ")];
    header_spans.extend(diff_view::file_header_spans(file, reviewed));
    header_spans.push(Span::styled(
        format!("  [{}/{}]", idx + 1, total),
        Style::default().fg(theme::DIM),
    ));
    if matches!(app.diff_mode, DiffMode::Split) {
        header_spans.push(Span::styled("  split", Style::default().fg(theme::ACCENT)));
    } else {
        header_spans.push(Span::styled(
            "  unified",
            Style::default().fg(theme::DIM).add_modifier(Modifier::DIM),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(header_spans)), outer[0]);

    let body_area = outer[2];
    let scroll = *app.diff_scroll.get(&file.path).unwrap_or(&0);
    let scroll_h = *app.diff_scroll_h.get(&file.path).unwrap_or(&0);
    match app.diff_mode {
        DiffMode::Unified => render_unified_inline(f, app, file, scroll, scroll_h, body_area),
        DiffMode::Split => {
            render_split_inline(f, app, file, scroll, scroll_h, body_area);
        }
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
    let blame = diff_view::blame_for(app, file);
    let lines = diff_view::build_unified_lines(file, blame);
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

    let blame = diff_view::blame_for(app, file);
    let (left, right) = diff_view::build_split_lines(file, blame);
    let lp = Paragraph::new(left).scroll((scroll, scroll_h));
    let rp = Paragraph::new(right).scroll((scroll, scroll_h));
    f.render_widget(lp, cols[0]);
    f.render_widget(rp, cols[1]);
}
