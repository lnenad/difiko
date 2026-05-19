pub mod commits_panel;
pub mod diff_view;
pub mod fullscreen;
pub mod help;
pub mod hint_bar;
pub mod modal;
pub mod review;
pub mod setup;
pub mod sidebar;
pub mod status_bar;
pub mod syntax;
pub mod theme;
pub mod toast;
pub mod word_diff;

use crate::app::{App, Modal, Screen};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Setup => setup::render(f, app),
        Screen::Review => review::render(f, app),
        Screen::Fullscreen => fullscreen::render(f, app),
    }
    if let Some(m) = &app.modal {
        match m {
            Modal::HelpOverlay => help::render(f, app),
            Modal::Error { message } => modal::render_error(f, message),
            Modal::BranchPicker { which, picker } => {
                let title = match which {
                    crate::app::BranchSlot::Base => "Pick base branch",
                    crate::app::BranchSlot::Compare => "Pick compare branch",
                };
                modal::render_picker(f, title, picker);
            }
            Modal::FileFilter { picker } => modal::render_picker(f, "Filter files", picker),
            Modal::CommandPalette { picker } => modal::render_picker(f, "Commands", picker),
        }
    }
    toast::render(f, app);
}
