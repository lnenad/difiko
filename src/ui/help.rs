use crate::app::App;
use crate::ui::{modal, theme};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

pub fn render(f: &mut Frame, _app: &App) {
    let area = modal::centered_rect(70, 75, f.area());
    let groups: Vec<(&str, Vec<(&str, &str)>)> = vec![
        (
            "Global",
            vec![
                ("?  /  F1", "toggle this help"),
                (":", "command palette  (Review / Fullscreen)"),
                ("Ctrl-c", "quit from any screen"),
                (
                    "q",
                    "quit  (Review only — typeable in Repo field, exits fullscreen otherwise)",
                ),
                ("Esc", "close modal / exit fullscreen / back to Setup"),
            ],
        ),
        (
            "Setup screen",
            vec![
                ("Tab / Shift-Tab", "cycle fields"),
                ("type / Backspace", "edit Repository path"),
                ("↑ / ↓", "cycle directory matches under cursor"),
                ("Shift-→", "accept highlighted directory"),
                ("Enter (Repo)", "load branches at this path"),
                (
                    "Enter (Base / Compare)",
                    "open branch picker  (lands on current value)",
                ),
                ("Enter (Remote)", "toggle include-remote-branches"),
                ("Enter (Submit)", "load diff (after both branches set)"),
                ("Esc", "clear branches & selections, refocus Repo"),
                ("?", "help  (any field except Repo, where it's typeable)"),
            ],
        ),
        (
            "Review screen",
            vec![
                ("Tab / Shift-Tab", "cycle pane focus"),
                ("1 / 2 / 3", "jump to Sidebar / Diff / Commits"),
                ("j / k", "scroll diff or move list selection"),
                ("h / l", "scroll diff horizontally  (Diff pane)"),
                ("J / K", "next / prev file"),
                ("g / G", "top / bottom"),
                ("Ctrl-d / Ctrl-u", "half-page scroll diff"),
                ("Ctrl-f / Ctrl-b", "page scroll diff"),
                ("Ctrl-j / Ctrl-k", "page scroll diff"),
                ("Ctrl-↓ / Ctrl-↑", "page scroll diff"),
                ("m", "mark file reviewed"),
                ("/", "fuzzy filter files"),
                ("c", "toggle commits panel"),
                ("t", "sidebar tree / flat"),
                ("u", "toggle Unified / Split diff"),
                ("b", "toggle git blame gutter"),
                ("W", "toggle word diff highlighting"),
                ("S", "toggle syntax highlighting"),
                ("F", "fullscreen current file"),
                ("r", "reload diff"),
                ("B", "branch picker"),
                (
                    "R",
                    "clear reviewed set  (press twice within 2s to confirm)",
                ),
                ("PgDn / PgUp", "page scroll diff  (Diff pane)"),
                (
                    "Esc",
                    "back to Setup, focus Compare  (state preserved; Esc again resets)",
                ),
                ("q", "quit"),
            ],
        ),
        (
            "Commits panel",
            vec![
                ("Enter", "filter files by commit"),
                ("Space", "expand / collapse body"),
            ],
        ),
        (
            "Fullscreen",
            vec![
                ("j / k", "scroll one line"),
                ("h / l / ← / →", "scroll horizontally"),
                ("Ctrl-d / Ctrl-u", "half-page scroll"),
                ("Ctrl-j / Ctrl-k", "page scroll"),
                ("Ctrl-↓ / Ctrl-↑", "page scroll"),
                ("g / G", "top / bottom"),
                ("J / Space", "next file"),
                ("K", "prev file"),
                ("m", "toggle reviewed"),
                ("u", "toggle Unified / Split"),
                ("b", "toggle git blame gutter"),
                ("W", "toggle word diff highlighting"),
                ("S", "toggle syntax highlighting"),
                ("q / Esc", "exit fullscreen"),
                ("Ctrl-c", "quit"),
            ],
        ),
        (
            "Modals",
            vec![
                ("Esc", "close"),
                ("Enter", "accept"),
                ("↑ / ↓", "move selection"),
                ("type / Backspace", "fuzzy filter"),
            ],
        ),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (i, (title, rows)) in groups.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            (*title).to_string(),
            Style::default()
                .fg(theme::accent())
                .add_modifier(Modifier::BOLD),
        )));
        for (k, label) in rows {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<22}", k),
                    Style::default().fg(theme::accent_dim()),
                ),
                Span::styled((*label).to_string(), Style::default().fg(theme::dim())),
            ]));
        }
    }
    modal::render_text_modal(f, area, " Keybindings ", lines);
}
