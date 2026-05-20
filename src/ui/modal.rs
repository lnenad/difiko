use crate::app::Picker;
use crate::ui::theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

/// Width/height (as percent of viewport) of the centered fuzzy-picker modal.
pub const PICKER_MODAL_PCT: (u16, u16) = (60, 60);
/// Width/height of the centered error modal — wider than tall.
pub const ERROR_MODAL_PCT: (u16, u16) = (60, 25);

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let pop = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(pop[1])[1]
}

pub fn render_picker(f: &mut Frame, title: &str, picker: &Picker) {
    let area = centered_rect(PICKER_MODAL_PCT.0, PICKER_MODAL_PCT.1, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(theme::ACCENT));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner);

    let mut input_spans = vec![
        Span::styled("> ", Style::default().fg(theme::ACCENT)),
        Span::raw(picker.query.buffer.clone()),
    ];
    input_spans.push(Span::styled("█", Style::default().fg(theme::ACCENT)));
    f.render_widget(Paragraph::new(Line::from(input_spans)), chunks[0]);

    let items: Vec<ListItem> = picker
        .filtered
        .iter()
        .filter_map(|i| picker.items.get(*i))
        .map(|s| ListItem::new(Line::from(Span::raw(s.clone()))))
        .collect();
    // The "▶" marker keeps the selected row visible even on terminals where
    // a subtle background tint is imperceptible.
    let list = List::new(items).highlight_symbol("▶ ").highlight_style(
        Style::default()
            .fg(theme::ACCENT)
            .bg(theme::HIGHLIGHT_BG)
            .add_modifier(Modifier::BOLD),
    );
    let mut state = ListState::default();
    if !picker.filtered.is_empty() {
        state.select(Some(picker.selected));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);
}

pub fn render_error(f: &mut Frame, message: &str) {
    let area = centered_rect(ERROR_MODAL_PCT.0, ERROR_MODAL_PCT.1, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Error ")
        .border_style(Style::default().fg(theme::DEL));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let p = Paragraph::new(message.to_string()).style(Style::default().fg(theme::DEL));
    f.render_widget(p, inner);
}

pub fn render_text_modal(f: &mut Frame, area: Rect, title: &str, lines: Vec<Line<'_>>) {
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(theme::ACCENT));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines), inner);
}
