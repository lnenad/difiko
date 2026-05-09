use crate::app::{App, ToastKind};
use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let mut y = area.y + area.height.saturating_sub(2);
    for toast in app.toasts.iter().rev() {
        let width = (toast.message.chars().count() as u16 + 4).min(area.width.saturating_sub(2));
        let height = 3u16;
        if y < height {
            break;
        }
        let x = area.x + area.width.saturating_sub(width + 1);
        let rect = Rect { x, y: y.saturating_sub(height), width, height };
        f.render_widget(Clear, rect);
        let color = match toast.kind {
            ToastKind::Info => theme::ACCENT,
            ToastKind::Error => theme::DEL,
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color));
        let inner = block.inner(rect);
        f.render_widget(block, rect);
        let p = Paragraph::new(toast.message.clone())
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD));
        f.render_widget(p, inner);
        y = y.saturating_sub(height + 1);
    }
}
