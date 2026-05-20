use std::path::PathBuf;

/// Split the user's repo-input buffer into (parent directory to scan, frag
/// to fuzzy-filter on). Handles `~`, `~/`, absolute and relative paths.
pub(super) fn split_path_for_completion(buf: &str) -> Option<(PathBuf, String)> {
    if buf.is_empty() {
        let cwd = std::env::current_dir().ok()?;
        return Some((cwd, String::new()));
    }
    if buf == "~" {
        let home = std::env::var_os("HOME")?;
        return Some((PathBuf::from(home), String::new()));
    }
    let (prefix, frag) = match buf.rfind('/') {
        Some(i) => (&buf[..=i], &buf[i + 1..]),
        None => ("", buf),
    };
    let parent: PathBuf = if prefix.is_empty() {
        std::env::current_dir().ok()?
    } else if prefix == "/" {
        PathBuf::from("/")
    } else if prefix == "~/" {
        PathBuf::from(std::env::var_os("HOME")?)
    } else if let Some(rest) = prefix.strip_prefix("~/") {
        let mut p = PathBuf::from(std::env::var_os("HOME")?);
        p.push(rest.trim_end_matches('/'));
        p
    } else {
        PathBuf::from(prefix.trim_end_matches('/'))
    };
    Some((parent, frag.to_string()))
}
