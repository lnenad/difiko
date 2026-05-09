use crate::model::DiffLine;

/// Split a multi-file unified diff into per-file sections, in the order git
/// emitted them. Each returned `Vec<DiffLine>` belongs to one file. Callers
/// match sections to file paths positionally — the section's "diff --git"
/// header path is not parsed (paths with whitespace would be ambiguous).
pub fn split_diff_into_sections(diff_text: &str) -> Vec<Vec<DiffLine>> {
    let mut starts: Vec<usize> = Vec::new();
    let bytes = diff_text.as_bytes();
    for (i, _) in diff_text.match_indices("diff --git ") {
        if i == 0 || bytes[i - 1] == b'\n' {
            starts.push(i);
        }
    }
    if starts.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Vec<DiffLine>> = Vec::with_capacity(starts.len());
    for (i, &start) in starts.iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(diff_text.len());
        let section = diff_text[start..end].trim_end_matches('\n');
        out.push(parse_section(section));
    }
    out
}

fn parse_section(section: &str) -> Vec<DiffLine> {
    let mut out = Vec::new();
    let mut in_hunk = false;
    for line in section.split('\n') {
        if line.starts_with("diff --git ") {
            out.push(DiffLine::GitHeader(line.to_string()));
        } else if line.starts_with("index ") {
            out.push(DiffLine::IndexHeader(line.to_string()));
        } else if line.starts_with("--- ") {
            out.push(DiffLine::OldFile(line.to_string()));
        } else if line.starts_with("+++ ") {
            out.push(DiffLine::NewFile(line.to_string()));
        } else if line.starts_with("@@") {
            in_hunk = true;
            let (old_start, old_count, new_start, new_count) = parse_hunk_header(line);
            out.push(DiffLine::Hunk {
                header: line.to_string(),
                old_start,
                old_count,
                new_start,
                new_count,
            });
        } else if line.starts_with("Binary files ") {
            out.push(DiffLine::Binary(line.to_string()));
        } else if !in_hunk {
            // file-mode lines, similarity index, etc.
            if !line.is_empty() {
                out.push(DiffLine::IndexHeader(line.to_string()));
            }
        } else if let Some(rest) = line.strip_prefix('+') {
            out.push(DiffLine::Add(rest.to_string()));
        } else if let Some(rest) = line.strip_prefix('-') {
            out.push(DiffLine::Del(rest.to_string()));
        } else if line.starts_with('\\') {
            out.push(DiffLine::NoNewline(line.to_string()));
        } else {
            let rest = line.strip_prefix(' ').unwrap_or(line);
            out.push(DiffLine::Context(rest.to_string()));
        }
    }
    out
}

fn parse_hunk_header(line: &str) -> (u32, u32, u32, u32) {
    let mut old_start = 0u32;
    let mut old_count = 1u32;
    let mut new_start = 0u32;
    let mut new_count = 1u32;
    if let Some(rest) = line.strip_prefix("@@") {
        let core = rest.split("@@").next().unwrap_or("");
        for token in core.split_whitespace() {
            if let Some(t) = token.strip_prefix('-') {
                let (s, c) = parse_pair(t);
                old_start = s;
                old_count = c;
            } else if let Some(t) = token.strip_prefix('+') {
                let (s, c) = parse_pair(t);
                new_start = s;
                new_count = c;
            }
        }
    }
    (old_start, old_count, new_start, new_count)
}

fn parse_pair(s: &str) -> (u32, u32) {
    let mut parts = s.split(',');
    let start = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let count = parts.next().and_then(|x| x.parse().ok()).unwrap_or(1);
    (start, count)
}

pub fn count_changes(lines: &[DiffLine]) -> (u32, u32, bool) {
    let mut adds = 0u32;
    let mut dels = 0u32;
    let mut binary = false;
    for line in lines {
        match line {
            DiffLine::Add(_) => adds += 1,
            DiffLine::Del(_) => dels += 1,
            DiffLine::Binary(_) => binary = true,
            _ => {}
        }
    }
    (adds, dels, binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_multi_file_diff_in_order() {
        let diff = "diff --git a/foo.rs b/foo.rs\nindex 1..2 100644\n--- a/foo.rs\n+++ b/foo.rs\n@@ -1,2 +1,2 @@\n-old\n+new\n context\ndiff --git a/bar.rs b/bar.rs\nnew file mode 100644\nindex 0..3 100644\n--- /dev/null\n+++ b/bar.rs\n@@ -0,0 +1,1 @@\n+hello\n";
        let sections = split_diff_into_sections(diff);
        assert_eq!(sections.len(), 2);
        let (a0, d0, _) = count_changes(&sections[0]);
        assert_eq!((a0, d0), (1, 1));
        let (a1, d1, _) = count_changes(&sections[1]);
        assert_eq!((a1, d1), (1, 0));
    }

    #[test]
    fn counts_changes() {
        let diff = "diff --git a/foo.rs b/foo.rs\nindex 1..2 100644\n--- a/foo.rs\n+++ b/foo.rs\n@@ -1,3 +1,3 @@\n-a\n+b\n+c\n d\n-e\n";
        let sections = split_diff_into_sections(diff);
        let (adds, dels, _) = count_changes(&sections[0]);
        assert_eq!(adds, 2);
        assert_eq!(dels, 2);
    }

    #[test]
    fn handles_path_with_whitespace_positionally() {
        // "diff --git" header with a quoted path; our parser ignores the path
        // text and just splits sections by position.
        let diff = "diff --git \"a/foo bar.rs\" \"b/foo bar.rs\"\nindex 1..2 100644\n--- \"a/foo bar.rs\"\n+++ \"b/foo bar.rs\"\n@@ -1,1 +1,1 @@\n-old\n+new\n";
        let sections = split_diff_into_sections(diff);
        assert_eq!(sections.len(), 1);
        let (a, d, _) = count_changes(&sections[0]);
        assert_eq!((a, d), (1, 1));
    }
}
