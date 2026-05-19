use crate::app::{App, SetupField};
use crate::ui::{hint_bar, status_bar, theme};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    status_bar::render(f, app, outer[0]);

    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(outer[1]);

    let _spacer = body[0];
    render_repo_input(f, app, body[1]);
    let _spacer2 = body[2];
    render_branch_pickers(f, app, body[3]);
    render_options(f, app, body[4]);

    hint_bar::render(f, app, outer[2]);

    // Draw completion dropdown LAST so it overlays everything else.
    if matches!(app.setup_field, SetupField::Repo)
        && !app.repo_dropdown_hidden
        && !app.repo_completions.is_empty()
    {
        render_completion_dropdown(f, app, body[1]);
    }
}

fn render_repo_input(f: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.setup_field, SetupField::Repo);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Repository ")
        .border_style(theme::focused_border(focused));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let mut spans = vec![Span::raw(app.repo_input.buffer.clone())];
    if focused {
        spans.push(Span::styled("█", Style::default().fg(theme::ACCENT)));
    }
    let p = Paragraph::new(Line::from(spans));
    f.render_widget(p, inner);
}

fn render_completion_dropdown(f: &mut Frame, app: &App, repo_area: Rect) {
    let max_visible: u16 = 6;
    let visible = (app.repo_completions.len() as u16).min(max_visible);
    let height = visible + 2;
    let frame_area = f.area();
    if repo_area.y + repo_area.height + height > frame_area.y + frame_area.height {
        return;
    }
    let widest = app
        .repo_completions
        .iter()
        .map(|s| s.chars().count() as u16)
        .max()
        .unwrap_or(0)
        + 4;
    let width = widest
        .max(20)
        .min(frame_area.width.saturating_sub(repo_area.x + 4));
    let drop_rect = Rect {
        x: repo_area.x + 2,
        y: repo_area.y + repo_area.height,
        width,
        height,
    };
    f.render_widget(Clear, drop_rect);

    let title = format!(
        " {} match{} — Shift-→ accept, ↑/↓ cycle ",
        app.repo_completions.len(),
        if app.repo_completions.len() == 1 {
            ""
        } else {
            "es"
        }
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIM))
        .title(Span::styled(title, Style::default().fg(theme::DIM)));
    let inner = block.inner(drop_rect);
    f.render_widget(block, drop_rect);

    let n = app.repo_completions.len();
    let cap = max_visible as usize;
    let start = if n <= cap || app.repo_completion_index < cap / 2 {
        0
    } else if app.repo_completion_index + cap / 2 >= n {
        n - cap
    } else {
        app.repo_completion_index - cap / 2
    };
    let end = (start + cap).min(n);

    let lines: Vec<Line> = app.repo_completions[start..end]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let abs_idx = start + i;
            let selected = abs_idx == app.repo_completion_index;
            let marker = if selected { "▶ " } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::styled(marker.to_string(), Style::default().fg(theme::ACCENT)),
                Span::styled(format!("{name}/"), style),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_branch_pickers(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    render_branch_list(
        f,
        app,
        cols[0],
        "Base branch",
        &app.base_branch,
        matches!(app.setup_field, SetupField::Base),
    );
    render_branch_list(
        f,
        app,
        cols[1],
        "Compare branch",
        &app.compare_branch,
        matches!(app.setup_field, SetupField::Compare),
    );
}

fn render_branch_list(
    f: &mut Frame,
    app: &App,
    area: Rect,
    label: &str,
    selected: &Option<String>,
    focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {label} "))
        .border_style(theme::focused_border(focused));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.branches.is_empty() {
        let msg = if app.pending.branches {
            "Loading branches…"
        } else {
            "No branches loaded. Tab to repo and press Enter."
        };
        let p = Paragraph::new(msg).style(Style::default().fg(theme::DIM));
        f.render_widget(p, inner);
        return;
    }

    let lines: Vec<Line> = app
        .branches
        .iter()
        .take(inner.height as usize)
        .map(|b| {
            let is_selected = selected.as_deref() == Some(b.as_str());
            let marker = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::raw(marker.to_string()),
                Span::styled(b.clone(), style),
            ])
        })
        .collect();
    let p = Paragraph::new(lines);
    f.render_widget(p, inner);

    if focused {
        let hint =
            Paragraph::new("Enter / Space / j / k to pick").style(Style::default().fg(theme::DIM));
        let h = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        f.render_widget(hint, h);
    }
}

fn render_options(f: &mut Frame, app: &App, area: Rect) {
    let on = if app.include_remote_branches {
        "ON"
    } else {
        "OFF"
    };
    let focused_remote = matches!(app.setup_field, SetupField::Remote);
    let focused_submit = matches!(app.setup_field, SetupField::Submit);
    let label_style = |focused| {
        if focused {
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::DIM)
        }
    };
    let line = Line::from(vec![
        Span::styled("include remote branches: ", label_style(focused_remote)),
        Span::raw(on),
        Span::raw("    "),
        Span::styled("load diff", label_style(focused_submit)),
        Span::styled(
            "    (Tab to a field, then Enter)",
            Style::default().fg(theme::DIM),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}
