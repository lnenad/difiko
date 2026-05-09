use super::{command, parse};
use crate::model::{sort_files, FileChange, FileStatus};
use anyhow::Result;
use std::path::Path;

pub async fn load_diff(repo: &Path, base: &str, compare: &str) -> Result<Vec<FileChange>> {
    let range = format!("{base}...{compare}");
    load_diff_for_range(repo, &range).await
}

pub async fn load_commit_diff(repo: &Path, commit: &str) -> Result<Vec<FileChange>> {
    let range = format!("{commit}^!");
    load_diff_for_range(repo, &range).await
}

async fn load_diff_for_range(repo: &Path, range: &str) -> Result<Vec<FileChange>> {
    let name_status = command::run(repo, &["diff", "--name-status", "-z", range, "--"]).await?;
    let entries = parse_name_status_z(&name_status);
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    // Disable C-style path quoting so the diff body uses raw UTF-8 paths;
    // section content is matched to entries positionally, so we don't depend
    // on the "diff --git" header path being parseable.
    let full_diff = command::run(
        repo,
        &["-c", "core.quotePath=false", "diff", range, "--"],
    )
    .await?;
    let sections = parse::split_diff_into_sections(&full_diff);

    let mut files: Vec<FileChange> = entries
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let lines = sections.get(i).cloned().unwrap_or_default();
            let (additions, deletions, binary) = parse::count_changes(&lines);
            FileChange {
                path: e.new_path,
                old_path: e.old_path,
                status: e.status,
                diff_lines: lines,
                additions,
                deletions,
                binary,
            }
        })
        .collect();

    sort_files(&mut files);
    Ok(files)
}

#[derive(Debug)]
struct NameStatusEntry {
    status: FileStatus,
    new_path: String,
    old_path: Option<String>,
}

/// Parse `git diff --name-status -z` output.
/// Records: `STATUS\0PATH\0` for plain entries, or `R/C+score\0OLD\0NEW\0`
/// for renames/copies.
fn parse_name_status_z(output: &str) -> Vec<NameStatusEntry> {
    let mut out = Vec::new();
    let mut tokens = output.split('\0').filter(|s| !s.is_empty());
    while let Some(raw_status) = tokens.next() {
        let Some(first) = raw_status.chars().next() else {
            continue;
        };
        if !first.is_ascii_uppercase() {
            continue;
        }
        let status = FileStatus::from_letter(first);
        let entry = match status {
            FileStatus::Renamed | FileStatus::Copied => {
                let Some(old) = tokens.next() else { break };
                let Some(new) = tokens.next() else { break };
                NameStatusEntry {
                    status,
                    new_path: new.to_string(),
                    old_path: Some(old.to_string()),
                }
            }
            _ => {
                let Some(p) = tokens.next() else { break };
                NameStatusEntry {
                    status,
                    new_path: p.to_string(),
                    old_path: None,
                }
            }
        };
        out.push(entry);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modified_z() {
        // STATUS \0 PATH \0 STATUS \0 PATH \0
        let entries = parse_name_status_z("M\0src/foo.rs\0A\0src/bar.rs\0");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].new_path, "src/foo.rs");
        assert!(matches!(entries[0].status, FileStatus::Modified));
        assert!(matches!(entries[1].status, FileStatus::Added));
    }

    #[test]
    fn parses_rename_z() {
        let entries = parse_name_status_z("R100\0old.rs\0new.rs\0");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].new_path, "new.rs");
        assert_eq!(entries[0].old_path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn parses_path_with_spaces_z() {
        let entries = parse_name_status_z("M\0path with spaces.rs\0");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].new_path, "path with spaces.rs");
    }
}
