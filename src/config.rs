use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_true")]
    pub word_diff: bool,
    #[serde(default = "default_true")]
    pub syntax_highlight: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            word_diff: true,
            syntax_highlight: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        let Ok(txt) = fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&txt).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path().context("could not resolve config dir")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("create config dir")?;
        }
        let json = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json).context("write temp config file")?;
        fs::rename(&tmp, &path).context("rename config file into place")?;
        Ok(())
    }
}

fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("dev", "local", "difiko")?;
    Some(dirs.config_dir().join("config.json"))
}
