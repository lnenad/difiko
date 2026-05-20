use super::{DiffMode, TextInput};
use crate::model::{DiffLine, FileChange};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffMatch {
    /// Index into the current file's `diff_lines`.
    pub line: usize,
    /// Byte offsets into the diff line's text content (excluding gutters).
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DiffSearch {
    pub query: TextInput,
    pub matches: Vec<DiffMatch>,
    pub current: usize,
    pub case_sensitive: bool,
}

/// Substring search returning byte ranges. In case-insensitive mode the
/// comparison is ASCII-only — non-ASCII bytes are compared byte-for-byte
/// (so "é" only matches "é"). Returned offsets are guaranteed to fall on
/// UTF-8 char boundaries so they can be used directly to slice `hay`.
pub(super) fn find_all_substr(
    hay: &str,
    needle: &[u8],
    case_sensitive: bool,
) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if needle.is_empty() || needle.len() > hay.len() {
        return out;
    }
    let h = hay.as_bytes();
    let mut i = 0;
    while i + needle.len() <= h.len() {
        let bytes_match = h[i..i + needle.len()].iter().zip(needle).all(|(a, b)| {
            if case_sensitive {
                a == b
            } else {
                a.eq_ignore_ascii_case(b)
            }
        });
        let on_boundary = hay.is_char_boundary(i) && hay.is_char_boundary(i + needle.len());
        if bytes_match && on_boundary {
            out.push((i, i + needle.len()));
        }
        i += 1;
    }
    out
}

/// Map a match's `diff_lines` index to the actual rendered-row index for the
/// current diff mode. The diff vector includes header entries (GitHeader,
/// IndexHeader, OldFile, NewFile) that produce no rendered row, and in split
/// mode adjacent dels/adds are paired into shared rows. Without this
/// translation, scroll math drifts and the match lands off-screen.
pub(crate) fn rendered_row_for_match(file: &FileChange, mode: DiffMode, target: usize) -> usize {
    let mut rendered = 0usize;
    match mode {
        DiffMode::Unified => {
            for (i, dl) in file.diff_lines.iter().enumerate() {
                if i == target {
                    return rendered;
                }
                match dl {
                    DiffLine::GitHeader(_)
                    | DiffLine::IndexHeader(_)
                    | DiffLine::OldFile(_)
                    | DiffLine::NewFile(_) => continue,
                    _ => rendered += 1,
                }
            }
            rendered
        }
        DiffMode::Split => {
            let mut pending_del = 0usize;
            let mut pending_add = 0usize;
            for (i, dl) in file.diff_lines.iter().enumerate() {
                let on_target = i == target;
                match dl {
                    DiffLine::Hunk { .. } | DiffLine::Context(_) => {
                        // Pending del/add rows render before this row.
                        rendered += pending_del.max(pending_add);
                        pending_del = 0;
                        pending_add = 0;
                        if on_target {
                            return rendered;
                        }
                        rendered += 1;
                    }
                    DiffLine::Del(_) => {
                        if on_target {
                            return rendered + pending_del;
                        }
                        pending_del += 1;
                    }
                    DiffLine::Add(_) => {
                        if on_target {
                            return rendered + pending_add;
                        }
                        pending_add += 1;
                    }
                    _ => {
                        if on_target {
                            return rendered;
                        }
                    }
                }
            }
            rendered + pending_del.max(pending_add)
        }
    }
}

/// Count of rendered rows for the file in the given mode. Mirrors the
/// renderer's filtering (skips header pseudo-lines) and the split-mode
/// del/add pairing. Used to clamp scroll so we don't store a position past
/// the end of the content.
pub(crate) fn total_rendered_rows(file: &FileChange, mode: DiffMode) -> usize {
    match mode {
        DiffMode::Unified => file
            .diff_lines
            .iter()
            .filter(|dl| {
                !matches!(
                    dl,
                    DiffLine::GitHeader(_)
                        | DiffLine::IndexHeader(_)
                        | DiffLine::OldFile(_)
                        | DiffLine::NewFile(_)
                )
            })
            .count(),
        DiffMode::Split => {
            let mut total = 0usize;
            let mut pending_del = 0usize;
            let mut pending_add = 0usize;
            for dl in &file.diff_lines {
                match dl {
                    DiffLine::Hunk { .. } | DiffLine::Context(_) => {
                        total += pending_del.max(pending_add);
                        pending_del = 0;
                        pending_add = 0;
                        total += 1;
                    }
                    DiffLine::Del(_) => pending_del += 1,
                    DiffLine::Add(_) => pending_add += 1,
                    _ => {}
                }
            }
            total + pending_del.max(pending_add)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FileStatus;

    #[test]
    fn find_all_substr_ci_finds_overlapping_and_case_insensitive() {
        let hits = find_all_substr("Foo foo FOO", b"foo", false);
        assert_eq!(hits, vec![(0, 3), (4, 7), (8, 11)]);
    }

    #[test]
    fn find_all_substr_case_sensitive_filters() {
        let hits = find_all_substr("Foo foo FOO", b"foo", true);
        assert_eq!(hits, vec![(4, 7)]);
    }

    #[test]
    fn find_all_substr_returns_char_boundary_offsets() {
        let hits = find_all_substr("café bar", b"bar", false);
        assert_eq!(hits, vec![(6, 9)]);
        assert_eq!(&"café bar"[6..9], "bar");
    }

    #[test]
    fn find_all_substr_empty_needle_returns_nothing() {
        let hits = find_all_substr("anything", b"", false);
        assert!(hits.is_empty());
    }

    #[test]
    fn rendered_row_skips_unrendered_headers_in_unified() {
        let file = FileChange {
            path: "a".into(),
            old_path: None,
            status: FileStatus::Modified,
            diff_lines: vec![
                DiffLine::GitHeader("diff --git ...".into()),
                DiffLine::IndexHeader("index ...".into()),
                DiffLine::Hunk {
                    header: "@@".into(),
                    old_start: 1,
                    old_count: 1,
                    new_start: 1,
                    new_count: 1,
                },
                DiffLine::Context("x".into()),
            ],
            additions: 0,
            deletions: 0,
            binary: false,
        };
        assert_eq!(rendered_row_for_match(&file, DiffMode::Unified, 2), 0);
        assert_eq!(rendered_row_for_match(&file, DiffMode::Unified, 3), 1);
    }

    #[test]
    fn rendered_row_pairs_del_and_add_in_split() {
        let file = FileChange {
            path: "a".into(),
            old_path: None,
            status: FileStatus::Modified,
            diff_lines: vec![
                DiffLine::Hunk {
                    header: "@@".into(),
                    old_start: 1,
                    old_count: 1,
                    new_start: 1,
                    new_count: 1,
                },
                DiffLine::Del("a".into()),
                DiffLine::Del("b".into()),
                DiffLine::Add("c".into()),
                DiffLine::Add("d".into()),
                DiffLine::Context("e".into()),
            ],
            additions: 2,
            deletions: 2,
            binary: false,
        };
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 1), 1);
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 2), 2);
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 3), 1);
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 4), 2);
        assert_eq!(rendered_row_for_match(&file, DiffMode::Split, 5), 3);
    }
}
