use super::{BranchSlot, Picker};

#[derive(Debug, Clone)]
pub enum Modal {
    BranchPicker { which: BranchSlot, picker: Picker },
    FileFilter { picker: Picker },
    CommandPalette { picker: Picker },
    HelpOverlay,
    Error { message: String },
}
