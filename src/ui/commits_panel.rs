use crate::app::{App, FocusedPane};
use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focused, FocusedPane::Commits);
    let title = format!(" Commits ({}) ", app.commits.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme::focused_border(focused));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.commits.is_empty() {
        let p = ratatui::widgets::Paragraph::new("No commits in range.")
            .style(Style::default().fg(theme::DIM));
        f.render_widget(p, inner);
        return;
    }

    let items: Vec<ListItem> = app
        .commits
        .iter()
        .map(|c| {
            let selected = app.selected_commit.as_deref() == Some(c.hash.as_str());
            let header_spans = vec![
                Span::styled(
                    if selected { "● " } else { "  " },
                    Style::default().fg(if selected { theme::ACCENT } else { theme::DIM }),
                ),
                Span::styled(
                    format!("{} ", c.short_hash),
                    Style::default().fg(theme::ACCENT_DIM).add_modifier(Modifier::BOLD),
                ),
                Span::raw(c.subject.clone()),
                Span::styled(
                    format!("  {}  {}", c.author, c.date),
                    Style::default().fg(theme::DIM),
                ),
            ];
            let mut lines: Vec<Line> = vec![Line::from(header_spans)];
            if app.expanded_commit.as_deref() == Some(c.hash.as_str()) && !c.body.is_empty() {
                for body_line in c.body.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("    {body_line}"),
                        Style::default().fg(theme::DIM),
                    )));
                }
            }
            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items).highlight_style(if focused {
        theme::highlight_style()
    } else {
        theme::dim_highlight_style()
    });
    let mut state = ListState::default();
    state.select(Some(app.commits_selected));
    f.render_stateful_widget(list, inner, &mut state);
}
