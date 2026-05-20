use crate::app::App;
use crate::ui::{commits_panel, diff_view, hint_bar, sidebar, status_bar};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

/// Sidebar column width in the Review layout. Wide enough to fit typical
/// path segments without truncation; narrow enough to leave most of the
/// frame for the diff body.
const SIDEBAR_WIDTH: u16 = 32;
/// Maximum commit rows shown before scrolling kicks in.
const COMMITS_MAX_VISIBLE: usize = 6;
/// Extra rows added when a commit is expanded (body preview).
const COMMITS_EXPANDED_EXTRA: u16 = 4;
/// Minimum diff body height before the commits panel.
const DIFF_BODY_MIN_HEIGHT: u16 = 8;

pub fn render(f: &mut Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status bar
            Constraint::Length(1), // breathing room
            Constraint::Min(0),    // body
            Constraint::Length(1), // hint bar
        ])
        .split(f.area());

    status_bar::render(f, app, outer[0]);

    if app.commits_panel_visible {
        let middle = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(DIFF_BODY_MIN_HEIGHT),
                Constraint::Length(commits_height(app)),
            ])
            .split(outer[2]);

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)])
            .split(middle[0]);

        sidebar::render(f, app, columns[0]);
        diff_view::render(f, app, columns[1]);
        commits_panel::render(f, app, middle[1]);
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)])
            .split(outer[2]);
        sidebar::render(f, app, columns[0]);
        diff_view::render(f, app, columns[1]);
    }

    hint_bar::render(f, app, outer[3]);
}

fn commits_height(app: &App) -> u16 {
    let n = app.commits.len();
    let base = (n.min(COMMITS_MAX_VISIBLE) + 2) as u16;
    if app.expanded_commit.is_some() {
        base + COMMITS_EXPANDED_EXTRA
    } else {
        base
    }
}
