use crate::app::{App, FocusedPane, Screen, SetupField};
use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let hints: Vec<(&str, &str)> = match app.screen {
        Screen::Setup => match app.setup_field {
            SetupField::Repo => {
                let dropdown_visible =
                    !app.repo_dropdown_hidden && !app.repo_completions.is_empty();
                if dropdown_visible {
                    vec![
                        ("↑/↓", "cycle dirs"),
                        ("Enter", "select dir"),
                        ("Shift-→", "select dir"),
                        ("type", "filter"),
                        ("Tab", "next field"),
                        ("Esc", "reset"),
                        ("Ctrl-c", "quit"),
                    ]
                } else {
                    vec![
                        ("type", "edit path"),
                        ("↑/↓", "show matches"),
                        ("Enter", "load branches"),
                        ("Tab", "next field"),
                        ("Esc", "reset"),
                        ("Ctrl-c", "quit"),
                    ]
                }
            }
            _ => vec![
                ("Tab", "next field"),
                ("Enter", "open picker"),
                ("Esc", "reset"),
                ("?", "help"),
                ("Ctrl-c", "quit"),
            ],
        },
        Screen::Fullscreen => vec![
            ("j/k", "scroll"),
            ("h/l/←/→", "h-scroll"),
            ("J/K", "next/prev file"),
            ("Ctrl-f", "find"),
            ("m", "mark reviewed"),
            ("b", "blame"),
            ("u", "split/unified"),
            ("W", "word diff"),
            ("S", "syntax"),
            ("q/Esc", "exit fullscreen"),
            ("?", "help"),
            ("Ctrl-c", "quit"),
        ],
        Screen::Review => match app.focused {
            FocusedPane::Sidebar => vec![
                ("j/k", "move"),
                ("Enter", "open"),
                ("Space", "fold"),
                ("t", "tree/flat"),
                ("/", "filter"),
                ("Tab", "next pane"),
                ("F", "fullscreen"),
                ("Esc", "tweak compare"),
                ("?", "help"),
                ("q", "quit"),
            ],
            FocusedPane::Diff => vec![
                ("j/k", "scroll"),
                ("h/l", "h-scroll"),
                ("J/K", "next/prev file"),
                ("Ctrl-f", "find"),
                ("m", "mark reviewed"),
                ("b", "blame"),
                ("u", "split/unified"),
                ("W", "word diff"),
                ("S", "syntax"),
                ("F", "fullscreen"),
                ("c", "toggle commits"),
                ("Tab", "next pane"),
                ("Esc", "tweak compare"),
                ("q", "quit"),
            ],
            FocusedPane::Commits => vec![
                ("j/k", "move"),
                ("Enter", "filter by commit"),
                ("Space", "expand body"),
                ("c", "close panel"),
                ("Tab", "next pane"),
                ("Esc", "tweak compare"),
                ("q", "quit"),
            ],
        },
    };

    let mut spans = vec![Span::raw(" ")];
    for (i, (k, label)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default().fg(theme::DIM)));
        }
        spans.push(Span::styled(*k, Style::default().fg(theme::ACCENT)));
        spans.push(Span::raw(":"));
        spans.push(Span::styled(*label, Style::default().fg(theme::DIM)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
