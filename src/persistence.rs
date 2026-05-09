use crate::model::FileChange;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub entries: HashMap<String, ReviewEntry>,
}

fn default_version() -> u32 {
    SCHEMA_VERSION
}

impl Default for StateFile {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            entries: HashMap::new(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReviewEntry {
    pub snapshot: String,
    pub reviewed_files: Vec<String>,
}

pub struct Store {
    path: PathBuf,
    state: StateFile,
}

pub struct OpenResult {
    pub store: Store,
    /// Set when an existing file was unparseable; the corrupt copy was renamed
    /// to this path and a fresh state was started.
    pub recovered_backup: Option<PathBuf>,
}

impl Store {
    pub fn open() -> Result<OpenResult> {
        let path = state_path()?;
        if !path.exists() {
            return Ok(OpenResult {
                store: Self {
                    path,
                    state: StateFile::default(),
                },
                recovered_backup: None,
            });
        }
        let txt = fs::read_to_string(&path).context("read state file")?;
        match serde_json::from_str::<StateFile>(&txt) {
            Ok(state) => Ok(OpenResult {
                store: Self { path, state },
                recovered_backup: None,
            }),
            Err(_) => {
                let backup = backup_path(&path);
                let _ = fs::rename(&path, &backup);
                Ok(OpenResult {
                    store: Self {
                        path,
                        state: StateFile::default(),
                    },
                    recovered_backup: Some(backup),
                })
            }
        }
    }

    pub fn load_reviewed(&self, repo: &str, base: &str, compare: &str, files: &[FileChange]) -> HashSet<String> {
        let key = make_key(repo, base, compare);
        let snapshot = snapshot_for(files);
        match self.state.entries.get(&key) {
            Some(entry) if entry.snapshot == snapshot => {
                entry.reviewed_files.iter().cloned().collect()
            }
            _ => HashSet::new(),
        }
    }

    pub fn save_reviewed(
        &mut self,
        repo: &str,
        base: &str,
        compare: &str,
        files: &[FileChange],
        reviewed: &HashSet<String>,
    ) -> Result<()> {
        let key = make_key(repo, base, compare);
        let snapshot = snapshot_for(files);
        let entry = ReviewEntry {
            snapshot,
            reviewed_files: reviewed.iter().cloned().collect(),
        };
        self.state.entries.insert(key, entry);
        self.flush()
    }

    fn flush(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).context("create state dir")?;
        }
        let json = serde_json::to_string_pretty(&self.state)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).context("write temp state file")?;
        fs::rename(&tmp, &self.path).context("rename state file into place")?;
        Ok(())
    }
}

fn state_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("dev", "local", "difiko")
        .context("could not resolve a data directory for difiko")?;
    Ok(dirs.data_dir().join("state.json"))
}

fn backup_path(path: &Path) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    path.with_extension(format!("json.broken-{ts}"))
}

fn make_key(repo: &str, base: &str, compare: &str) -> String {
    format!("{repo}::{base}::{compare}")
}

fn snapshot_for(files: &[FileChange]) -> String {
    let mut paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    paths.sort_unstable();
    let mut hasher = Sha256::new();
    for p in &paths {
        hasher.update(p.as_bytes());
        hasher.update([0u8]);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FileStatus;

    fn fc(path: &str) -> FileChange {
        FileChange {
            path: path.into(),
            old_path: None,
            status: FileStatus::Modified,
            diff_lines: Vec::new(),
            additions: 0,
            deletions: 0,
            binary: false,
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir();
        let path = dir.join("state.json");
        let mut store = Store {
            path: path.clone(),
            state: StateFile::default(),
        };
        let files = vec![fc("a.rs"), fc("b.rs")];
        let reviewed: HashSet<String> = ["a.rs".to_string()].into_iter().collect();
        store.save_reviewed("/r", "main", "feat", &files, &reviewed).unwrap();

        let txt = fs::read_to_string(&path).unwrap();
        let parsed: StateFile = serde_json::from_str(&txt).unwrap();
        let entry = parsed.entries.get("/r::main::feat").unwrap();
        assert_eq!(entry.reviewed_files, vec!["a.rs"]);
    }

    #[test]
    fn snapshot_mismatch_returns_empty() {
        let dir = tempdir();
        let path = dir.join("state.json");
        let mut store = Store {
            path: path.clone(),
            state: StateFile::default(),
        };
        let original = vec![fc("a.rs"), fc("b.rs")];
        let reviewed: HashSet<String> = ["a.rs".to_string()].into_iter().collect();
        store.save_reviewed("/r", "main", "feat", &original, &reviewed).unwrap();

        let changed = vec![fc("a.rs"), fc("c.rs")];
        let loaded = store.load_reviewed("/r", "main", "feat", &changed);
        assert!(loaded.is_empty(), "snapshot mismatch should return empty set");
    }

    #[test]
    fn snapshot_match_returns_reviewed() {
        let dir = tempdir();
        let path = dir.join("state.json");
        let mut store = Store {
            path: path.clone(),
            state: StateFile::default(),
        };
        let files = vec![fc("a.rs"), fc("b.rs")];
        let reviewed: HashSet<String> = ["b.rs".to_string()].into_iter().collect();
        store.save_reviewed("/r", "main", "feat", &files, &reviewed).unwrap();

        let loaded = store.load_reviewed("/r", "main", "feat", &files);
        assert_eq!(loaded, reviewed);
    }

    #[test]
    fn snapshot_invariant_to_path_order() {
        let a = vec![fc("a.rs"), fc("b.rs")];
        let b = vec![fc("b.rs"), fc("a.rs")];
        assert_eq!(snapshot_for(&a), snapshot_for(&b));
    }

    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("difiko-test-{ts}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
