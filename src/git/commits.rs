use super::command;
use crate::model::Commit;
use anyhow::Result;
use std::path::Path;

const FORMAT: &str = "--pretty=format:%H%x1f%h%x1f%an%x1f%ad%x1f%s%x1f%b%x1e";

pub async fn load_commits(repo: &Path, base: &str, compare: &str) -> Result<Vec<Commit>> {
    if super::is_working_tree(compare) {
        // Working tree has no commits between itself and base by definition.
        return Ok(Vec::new());
    }
    let range = format!("{base}..{compare}");
    let stdout = command::run(repo, &["log", "--date=short", FORMAT, &range, "--"]).await?;
    Ok(parse_commits(&stdout))
}

fn parse_commits(stdout: &str) -> Vec<Commit> {
    stdout
        .split('\u{1e}')
        .map(|s| s.trim_matches(['\n', '\r']))
        .filter(|s| !s.is_empty())
        .filter_map(parse_one)
        .collect()
}

fn parse_one(entry: &str) -> Option<Commit> {
    let mut parts = entry.split('\u{1f}');
    let hash = parts.next()?.to_string();
    let short_hash = parts.next()?.to_string();
    let author = parts.next()?.to_string();
    let date = parts.next()?.to_string();
    let subject = parts.next()?.to_string();
    let body = parts.next().unwrap_or("").trim().to_string();
    Some(Commit {
        hash,
        short_hash,
        author,
        date,
        subject,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_commits() {
        let input = "abc\u{1f}abc\u{1f}Alice\u{1f}2026-01-02\u{1f}Subject one\u{1f}body line\n\u{1e}def\u{1f}def\u{1f}Bob\u{1f}2026-01-03\u{1f}Subject two\u{1f}\u{1e}";
        let commits = parse_commits(input);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].author, "Alice");
        assert_eq!(commits[0].body, "body line");
        assert_eq!(commits[1].subject, "Subject two");
        assert_eq!(commits[1].body, "");
    }
}
