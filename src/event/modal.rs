use super::loaders::{kick_off_load_commits, kick_off_load_diff, picker_items_for};
use super::AppEvent;
use crate::app::{App, BranchSlot, FocusedPane, Modal, Picker, Screen, ToastKind};
use tokio::sync::mpsc::UnboundedSender;

pub(super) enum ModalInputOp {
    Insert(char),
    Backspace,
}

pub(super) fn modal_input(app: &mut App, op: ModalInputOp) {
    let Some(modal) = app.modal.as_mut() else {
        return;
    };
    let picker = match modal {
        Modal::BranchPicker { picker, .. }
        | Modal::FileFilter { picker }
        | Modal::CommandPalette { picker } => picker,
        _ => return,
    };
    match op {
        ModalInputOp::Insert(c) => picker.query.insert(c),
        ModalInputOp::Backspace => picker.query.backspace(),
    }
    picker.refilter();
}

pub(super) fn modal_move(app: &mut App, delta: i32) {
    let Some(modal) = app.modal.as_mut() else {
        return;
    };
    let picker = match modal {
        Modal::BranchPicker { picker, .. }
        | Modal::FileFilter { picker }
        | Modal::CommandPalette { picker } => picker,
        _ => return,
    };
    if delta < 0 {
        picker.move_up();
    } else {
        picker.move_down();
    }
}

pub(super) fn handle_modal_accept(app: &mut App, modal: Modal, tx: &UnboundedSender<AppEvent>) {
    match modal {
        Modal::BranchPicker { which, picker } => {
            if let Some(b) = picker.current().cloned() {
                match which {
                    BranchSlot::Base => app.base_branch = Some(b),
                    BranchSlot::Compare => app.compare_branch = Some(b),
                }
                let just_picked_base = matches!(which, BranchSlot::Base);
                if matches!(app.screen, Screen::Setup) && just_picked_base {
                    // Force the user through the compare step before loading.
                    app.setup_field = crate::app::SetupField::Compare;
                    let items = picker_items_for(app, BranchSlot::Compare);
                    app.modal = Some(Modal::BranchPicker {
                        which: BranchSlot::Compare,
                        picker: Picker::with_selected(items, app.compare_branch.as_deref()),
                    });
                } else if app.base_branch.is_some() && app.compare_branch.is_some() {
                    kick_off_load_diff(app, tx);
                    kick_off_load_commits(app, tx);
                }
            }
        }
        Modal::FileFilter { picker } => {
            if let Some(p) = picker.current().cloned() {
                app.select_file_by_path(&p);
                app.focused = FocusedPane::Diff;
            }
        }
        Modal::CommandPalette { picker } => {
            if let Some(cmd) = picker.current().cloned() {
                run_command(app, &cmd, tx);
            }
        }
        _ => {}
    }
}

pub(super) fn run_command(app: &mut App, cmd: &str, tx: &UnboundedSender<AppEvent>) {
    match cmd {
        "branches" => {
            let items = picker_items_for(app, BranchSlot::Compare);
            app.modal = Some(Modal::BranchPicker {
                which: BranchSlot::Compare,
                picker: Picker::with_selected(items, app.compare_branch.as_deref()),
            });
        }
        "reload" => {
            kick_off_load_diff(app, tx);
            kick_off_load_commits(app, tx);
        }
        "tree" => {
            app.sidebar_mode = crate::app::SidebarMode::Tree;
            app.sidebar_selected = 0;
            app.rebuild_tree();
        }
        "flat" => {
            app.sidebar_mode = crate::app::SidebarMode::Flat;
            app.sidebar_selected = 0;
        }
        "fullscreen" => {
            if let Some(idx) = app.current_file_index() {
                app.fullscreen_idx = idx;
                app.screen = Screen::Fullscreen;
            }
        }
        "clear-reviewed" => {
            app.reviewed.clear();
            app.save_review();
            app.toast("Cleared reviewed set", ToastKind::Info);
        }
        "toggle-commits" => app.commits_panel_visible = !app.commits_panel_visible,
        "split-diff" => app.diff_mode = crate::app::DiffMode::Split,
        "unified-diff" => app.diff_mode = crate::app::DiffMode::Unified,
        "toggle-word-diff" => {
            app.toggle_word_diff();
            let on = app.config.word_diff;
            app.toast(
                format!("Word diff: {}", if on { "on" } else { "off" }),
                ToastKind::Info,
            );
        }
        "toggle-syntax" => {
            app.toggle_syntax_highlight();
            let on = app.config.syntax_highlight;
            app.toast(
                format!("Syntax highlight: {}", if on { "on" } else { "off" }),
                ToastKind::Info,
            );
        }
        "quit" => app.should_quit = true,
        _ => {}
    }
}
