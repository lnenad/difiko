use super::command;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct BlameLine {
    pub short_hash: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, Default)]
pub struct Blame {
    pub by_line: HashMap<u32, BlameLine>,
}

pub async fn load_blame(repo: &Path, git_ref: &str, file: &str) -> Result<Blame> {
    // `--` already separates ref from path here.
    let stdout = command::run(repo, &["blame", "--porcelain", git_ref, "--", file]).await?;
    Ok(parse_porcelain(&stdout))
}

fn parse_porcelain(stdout: &str) -> Blame {
    let mut by_line: HashMap<u32, BlameLine> = HashMap::new();
    // Per-commit cached metadata: hash -> (short, author, date).
    let mut commits: HashMap<String, (String, String, String)> = HashMap::new();

    let mut current_hash = String::new();
    let mut current_final: u32 = 0;
    let mut current_author = String::new();
    let mut current_author_time: i64 = 0;
    let mut current_author_tz_secs: i32 = 0;

    for line in stdout.lines() {
        if let Some(content_marker) = line.strip_prefix('\t') {
            let _ = content_marker;
            let entry = commits.entry(current_hash.clone()).or_insert_with(|| {
                let short = current_hash.chars().take(7).collect::<String>();
                let local_secs = current_author_time + current_author_tz_secs as i64;
                let date = format_date_ymd(local_secs);
                (short, current_author.clone(), date)
            });
            by_line.insert(
                current_final,
                BlameLine {
                    short_hash: entry.0.clone(),
                    author: entry.1.clone(),
                    date: entry.2.clone(),
                },
            );
        } else if let Some((key, rest)) = line.split_once(' ') {
            if is_commit_hash(key) {
                current_hash = key.to_string();
                let mut nums = rest.split_whitespace();
                let _orig: u32 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                current_final = nums.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                if !commits.contains_key(&current_hash) {
                    current_author.clear();
                    current_author_time = 0;
                    current_author_tz_secs = 0;
                }
            } else if key == "author" {
                current_author = rest.to_string();
            } else if key == "author-time" {
                current_author_time = rest.parse().unwrap_or(0);
            } else if key == "author-tz" {
                current_author_tz_secs = parse_tz_offset(rest);
            }
        }
    }
    Blame { by_line }
}

fn is_commit_hash(s: &str) -> bool {
    // Accept SHA-1 (40 chars) and SHA-256 (64 chars).
    matches!(s.len(), 40 | 64) && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_tz_offset(s: &str) -> i32 {
    // "+0530" / "-0800" → seconds east of UTC.
    let s = s.trim();
    if s.len() != 5 {
        return 0;
    }
    let sign: i32 = match s.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return 0,
    };
    let h: i32 = s[1..3].parse().unwrap_or(0);
    let m: i32 = s[3..5].parse().unwrap_or(0);
    sign * (h * 3600 + m * 60)
}

// Howard Hinnant's civil_from_days algorithm.
fn format_date_ymd(epoch_seconds: i64) -> String {
    if epoch_seconds == 0 {
        return String::new();
    }
    let days = epoch_seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_lines_same_commit() {
        let porcelain = "abc1234567890123456789012345678901234567 1 1 2\nauthor Alice\nauthor-time 1700000000\nauthor-tz +0000\nsummary first\nfilename foo\n\thello\nabc1234567890123456789012345678901234567 2 2\n\tworld\n";
        let blame = parse_porcelain(porcelain);
        assert_eq!(blame.by_line.len(), 2);
        let l1 = &blame.by_line[&1];
        let l2 = &blame.by_line[&2];
        assert_eq!(l1.author, "Alice");
        assert_eq!(l2.author, "Alice");
        assert_eq!(l1.short_hash, "abc1234");
    }

    #[test]
    fn accepts_sha256_hash() {
        // 64-char SHA-256 commit object id.
        let h = "0".repeat(64);
        let porcelain = format!(
            "{h} 1 1 1\nauthor Bob\nauthor-time 1700000000\nauthor-tz +0000\nsummary x\nfilename foo\n\thi\n"
        );
        let blame = parse_porcelain(&porcelain);
        assert_eq!(blame.by_line.len(), 1);
        assert_eq!(blame.by_line[&1].author, "Bob");
    }

    #[test]
    fn empty_input_yields_empty_blame() {
        let blame = parse_porcelain("");
        assert!(blame.by_line.is_empty());
    }

    #[test]
    fn cross_commit_caches_per_hash() {
        // Two commits, one line each. Hash B reuses cached metadata if seen
        // again; both lines should land with their own author/date.
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let porcelain = format!(
            "{a} 1 1 1\nauthor Alice\nauthor-time 1700000000\nauthor-tz +0000\nsummary x\nfilename foo\n\tline-a\n\
             {b} 2 2 1\nauthor Bob\nauthor-time 1700086400\nauthor-tz +0000\nsummary y\nfilename foo\n\tline-b\n"
        );
        let blame = parse_porcelain(&porcelain);
        assert_eq!(blame.by_line.len(), 2);
        assert_eq!(blame.by_line[&1].author, "Alice");
        assert_eq!(blame.by_line[&2].author, "Bob");
        assert_eq!(blame.by_line[&1].short_hash, "aaaaaaa");
        assert_eq!(blame.by_line[&2].short_hash, "bbbbbbb");
    }

    #[test]
    fn malformed_tz_offset_defaults_to_zero() {
        // tz offset must be exactly 5 chars; anything else falls back to 0.
        assert_eq!(parse_tz_offset(""), 0);
        assert_eq!(parse_tz_offset("+05"), 0);
        assert_eq!(parse_tz_offset("0500"), 0); // missing sign
        assert_eq!(parse_tz_offset("+0530"), 5 * 3600 + 30 * 60);
        assert_eq!(parse_tz_offset("-0800"), -8 * 3600);
    }

    #[test]
    fn non_hash_lines_are_ignored() {
        // Garbage before the first commit-hash line — must not panic.
        let porcelain =
            "previous-filename foo\nboundary\nabc1234567890123456789012345678901234567 1 1 1\n\
             author Z\nauthor-time 1700000000\nauthor-tz +0000\nsummary s\nfilename foo\n\tx\n";
        let blame = parse_porcelain(porcelain);
        assert_eq!(blame.by_line.len(), 1);
        assert_eq!(blame.by_line[&1].author, "Z");
    }

    #[test]
    fn invalid_hex_in_hash_position_is_ignored() {
        // Right length but non-hex chars — must be rejected by is_commit_hash.
        assert!(!is_commit_hash(&"z".repeat(40)));
        assert!(!is_commit_hash(&"a".repeat(39))); // wrong length
        assert!(is_commit_hash(&"a".repeat(40)));
        assert!(is_commit_hash(&"f".repeat(64)));
    }

    #[test]
    fn format_date_ymd_zero_is_empty() {
        assert_eq!(format_date_ymd(0), "");
    }

    #[test]
    fn applies_tz_offset_to_date() {
        // 1700000000 = 2023-11-14 22:13:20 UTC. With +0500 offset, local date
        // is 2023-11-15 (next day).
        let porcelain =
            "abc1234567890123456789012345678901234567 1 1 1\nauthor Alice\nauthor-time 1700000000\nauthor-tz +0500\nsummary x\nfilename foo\n\thi\n";
        let blame = parse_porcelain(porcelain);
        assert_eq!(blame.by_line[&1].date, "2023-11-15");
    }
}
