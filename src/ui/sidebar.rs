use crate::app::{App, FocusedPane, SidebarMode};
use crate::model::FileChange;
use crate::tree::TreeRow;
use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;
use std::collections::HashMap;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focused, FocusedPane::Sidebar);
    let title = match app.sidebar_mode {
        SidebarMode::Flat => " Files ",
        SidebarMode::Tree => " Files (Tree) ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme::focused_border(focused));

    let inner = block.inner(area);
    f.render_widget(block, area);
    if app.files.is_empty() {
        let p = ratatui::widgets::Paragraph::new("No files. Press 'r' to reload.")
            .style(Style::default().fg(theme::dim()));
        f.render_widget(p, inner);
        return;
    }

    let items: Vec<ListItem> = match app.sidebar_mode {
        SidebarMode::Flat => app
            .files
            .iter()
            .map(|f| {
                ListItem::new(Line::from(format_flat_row(
                    f,
                    app.reviewed.contains(&f.path),
                )))
            })
            .collect(),
        SidebarMode::Tree => {
            let by_path: HashMap<&str, &FileChange> =
                app.files.iter().map(|f| (f.path.as_str(), f)).collect();
            app.tree_rows
                .iter()
                .map(|row| ListItem::new(Line::from(format_tree_row(app, row, &by_path))))
                .collect()
        }
    };

    let list = List::new(items).highlight_style(if focused {
        theme::highlight_style()
    } else {
        theme::dim_highlight_style()
    });
    let mut state = ListState::default();
    state.select(Some(app.sidebar_selected));
    f.render_stateful_widget(list, inner, &mut state);
}

fn format_flat_row(file: &crate::model::FileChange, reviewed: bool) -> Vec<Span<'static>> {
    let mark = if reviewed { "✓ " } else { "  " };
    let mark_style = if reviewed {
        Style::default().fg(theme::add())
    } else {
        Style::default().fg(theme::dim())
    };
    vec![
        Span::styled(mark.to_string(), mark_style),
        Span::styled(
            format!("{} ", file.status.short()),
            Style::default()
                .fg(theme::status_color(file.status))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(file.path.clone()),
    ]
}

fn format_tree_row(
    app: &App,
    row: &TreeRow,
    by_path: &HashMap<&str, &FileChange>,
) -> Vec<Span<'static>> {
    match row {
        TreeRow::Dir {
            label,
            depth,
            collapsed,
            ..
        } => {
            let indent = "  ".repeat(*depth);
            let caret = if *collapsed { "▸" } else { "▾" };
            vec![
                Span::raw(indent),
                Span::styled(format!("{caret} "), Style::default().fg(theme::dim())),
                Span::styled(
                    format!("{label}/"),
                    Style::default()
                        .fg(theme::accent_dim())
                        .add_modifier(Modifier::BOLD),
                ),
            ]
        }
        TreeRow::File { label, depth, path } => {
            let indent = "  ".repeat(*depth + 1);
            let reviewed = app.reviewed.contains(path);
            let mark = if reviewed { "✓ " } else { "  " };
            let mark_style = if reviewed {
                Style::default().fg(theme::add())
            } else {
                Style::default().fg(theme::dim())
            };
            let file = by_path.get(path.as_str()).copied();
            let status_letter = file.map(|f| f.status.short()).unwrap_or(" ");
            let status_color = file
                .map(|f| theme::status_color(f.status))
                .unwrap_or(theme::dim());
            vec![
                Span::raw(indent),
                Span::styled(mark.to_string(), mark_style),
                Span::styled(
                    format!("{status_letter} "),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(label.clone()),
            ]
        }
    }
}
