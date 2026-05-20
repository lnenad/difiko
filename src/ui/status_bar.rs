use crate::app::App;
use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let repo = app
        .repo_path
        .as_ref()
        .map(|p| {
            p.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string())
        })
        .unwrap_or_else(|| "<none>".to_string());

    let base = app.base_branch.clone().unwrap_or_else(|| "—".to_string());
    let compare = app
        .compare_branch
        .clone()
        .unwrap_or_else(|| "—".to_string());

    let mut spans = vec![
        Span::styled(
            " difiko ",
            Style::default()
                .fg(theme::accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(theme::dim())),
        Span::styled("repo: ", theme::label_style()),
        Span::raw(repo),
        Span::styled(" │ ", Style::default().fg(theme::dim())),
        Span::styled("base: ", theme::label_style()),
        Span::raw(base),
        Span::styled(" → ", Style::default().fg(theme::dim())),
        Span::raw(compare),
    ];

    if !app.files.is_empty() {
        spans.push(Span::styled(" │ ", Style::default().fg(theme::dim())));
        spans.push(Span::styled("files: ", theme::label_style()));
        spans.push(Span::raw(app.files.len().to_string()));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("+{}", app.total_additions()),
            Style::default().fg(theme::add()),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("-{}", app.total_deletions()),
            Style::default().fg(theme::del()),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled("reviewed: ", theme::label_style()));
        let reviewed_count = app
            .files
            .iter()
            .filter(|f| app.reviewed.contains(&f.path))
            .count();
        spans.push(Span::raw(format!("{}/{}", reviewed_count, app.files.len())));
        if let Some(hash) = &app.selected_commit {
            spans.push(Span::styled(" │ ", Style::default().fg(theme::dim())));
            spans.push(Span::styled(
                format!("filtered by {}", &hash[..hash.len().min(8)]),
                Style::default().fg(theme::accent()),
            ));
        }
    }

    if app.pending.diff
        || app.pending.commits
        || app.pending.branches
        || app.pending.commit_diff.is_active()
    {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame = frames[(app.spinner_tick as usize) % frames.len()];
        spans.push(Span::styled(
            format!("  {frame} loading"),
            Style::default().fg(theme::accent()),
        ));
    }

    let p = Paragraph::new(Line::from(spans)).block(Block::default());
    f.render_widget(p, area);
}
