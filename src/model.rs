use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Other,
}

impl FileStatus {
    pub fn from_letter(c: char) -> Self {
        match c.to_ascii_uppercase() {
            'A' => FileStatus::Added,
            'M' => FileStatus::Modified,
            'D' => FileStatus::Deleted,
            'R' => FileStatus::Renamed,
            'C' => FileStatus::Copied,
            _ => FileStatus::Other,
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            FileStatus::Added => "A",
            FileStatus::Modified => "M",
            FileStatus::Deleted => "D",
            FileStatus::Renamed => "R",
            FileStatus::Copied => "C",
            FileStatus::Other => "?",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FileStatus::Added => "Added",
            FileStatus::Modified => "Modified",
            FileStatus::Deleted => "Deleted",
            FileStatus::Renamed => "Renamed",
            FileStatus::Copied => "Copied",
            FileStatus::Other => "Changed",
        }
    }
}

#[derive(Debug, Clone)]
pub enum DiffLine {
    GitHeader(String),
    IndexHeader(String),
    OldFile(String),
    NewFile(String),
    Hunk {
        header: String,
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
    },
    Add(String),
    Del(String),
    Context(String),
    NoNewline(String),
    Binary(String),
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub diff_lines: Vec<DiffLine>,
    pub additions: u32,
    pub deletions: u32,
    pub binary: bool,
}

impl FileChange {
    /// Header label for the file. Returns a borrow of `self.path` in the
    /// common (non-rename) case so callers don't allocate per render.
    pub fn display_name(&self) -> Cow<'_, str> {
        match (&self.old_path, self.status) {
            (Some(old), FileStatus::Renamed | FileStatus::Copied) if old != &self.path => {
                Cow::Owned(format!("{} → {}", old, self.path))
            }
            _ => Cow::Borrowed(self.path.as_str()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub body: String,
}

pub fn sort_files(files: &mut [FileChange]) {
    files.sort_by(|a, b| {
        let a_dir = a.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let b_dir = b.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        a_dir
            .cmp(b_dir)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.status.short().cmp(b.status.short()))
    });
}
