use super::command;
use anyhow::Result;
use std::path::Path;

pub async fn list_branches(repo: &Path, include_remote: bool) -> Result<Vec<String>> {
    let mut args: Vec<&str> = vec!["branch", "--format=%(refname:short)"];
    if include_remote {
        args.insert(1, "-a");
    }
    let stdout = command::run(repo, &args).await?;
    let mut branches: Vec<String> = stdout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.contains("HEAD ->"))
        .map(|l| l.to_string())
        .collect();
    branches.sort();
    branches.dedup();
    Ok(branches)
}
