//! Intra-line word diff using the `similar` crate.
//!
//! Given a `-` line and the adjacent `+` line, returns byte ranges within
//! each marking the segments that *differ*. The renderer dims the
//! unchanged ranges and bolds the changed ones, so the eye is drawn to the
//! actual edit instead of the whole line.
//!
//! We diff at *character* granularity rather than `from_words`.
//! `from_words` tokenises quoted strings / dotted numbers as a single
//! token, so a version bump like `"0.1.3" → "0.1.4"` highlights the
//! whole quoted literal instead of just `3 → 4`. `from_chars` matches
//! what `git diff --word-diff` does in practice for small edits, and
//! adjacent changed characters coalesce into one range so the output
//! reads cleanly.

use similar::{ChangeTag, TextDiff};

pub type ByteRanges = Vec<(usize, usize)>;

/// `(changed_ranges_in_old, changed_ranges_in_new)`, each a list of
/// `(start_byte, end_byte)` half-open intervals.
pub fn word_ranges(old: &str, new: &str) -> (ByteRanges, ByteRanges) {
    let diff = TextDiff::from_chars(old, new);
    let mut old_ranges: Vec<(usize, usize)> = Vec::new();
    let mut new_ranges: Vec<(usize, usize)> = Vec::new();
    let mut old_cur = 0usize;
    let mut new_cur = 0usize;
    for change in diff.iter_all_changes() {
        let text = change.value();
        let len = text.len();
        match change.tag() {
            ChangeTag::Equal => {
                old_cur += len;
                new_cur += len;
            }
            ChangeTag::Delete => {
                merge_or_push(&mut old_ranges, old_cur, old_cur + len);
                old_cur += len;
            }
            ChangeTag::Insert => {
                merge_or_push(&mut new_ranges, new_cur, new_cur + len);
                new_cur += len;
            }
        }
    }
    (old_ranges, new_ranges)
}

fn merge_or_push(out: &mut Vec<(usize, usize)>, s: usize, e: usize) {
    if let Some(last) = out.last_mut() {
        if last.1 == s {
            last.1 = e;
            return;
        }
    }
    out.push((s, e));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_word_change_middle() {
        let (o, n) = word_ranges("foo bar baz", "foo qux baz");
        // "bar" is at bytes 4..7 in old; "qux" at 4..7 in new.
        assert_eq!(o, vec![(4, 7)]);
        assert_eq!(n, vec![(4, 7)]);
    }

    #[test]
    fn identical_lines_no_ranges() {
        let (o, n) = word_ranges("foo bar", "foo bar");
        assert!(o.is_empty());
        assert!(n.is_empty());
    }

    #[test]
    fn version_bump_only_changes_digit() {
        let (o, n) = word_ranges("version = \"0.1.3\"", "version = \"0.1.4\"");
        eprintln!("old ranges: {:?}", o);
        eprintln!("new ranges: {:?}", n);
        // Sanity: ranges shouldn't cover the whole line.
        let line_len = "version = \"0.1.3\"".len();
        let total_old: usize = o.iter().map(|(s, e)| e - s).sum();
        let total_new: usize = n.iter().map(|(s, e)| e - s).sum();
        assert!(
            total_old < line_len / 2,
            "old change-span {total_old} too large vs line {line_len}"
        );
        assert!(
            total_new < line_len / 2,
            "new change-span {total_new} too large vs line {line_len}"
        );
    }

    #[test]
    fn lines_with_no_common_chars_flag_everything() {
        // No characters in common → every byte ends up in a changed range.
        let (o, n) = word_ranges("abc", "xyz");
        let total_old: usize = o.iter().map(|(s, e)| e - s).sum();
        let total_new: usize = n.iter().map(|(s, e)| e - s).sum();
        assert_eq!(total_old, 3);
        assert_eq!(total_new, 3);
    }
}
