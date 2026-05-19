// Smoke test against the local infra repo. Skipped in environments where it doesn't exist.
use std::path::PathBuf;

#[tokio::test]
async fn loads_branches_diff_commits_from_real_repo() {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        eprintln!("skipping: no HOME/USERPROFILE in env");
        return;
    };
    let repo = PathBuf::from(home).join("projects/infra");
    if !repo.join(".git").exists() {
        eprintln!("skipping: {} is not a git repo", repo.display());
        return;
    }

    difiko::git::ensure_git_repo(&repo)
        .await
        .expect("ensure_git_repo");
    let branches = difiko::git::list_branches(&repo, true)
        .await
        .expect("list_branches");
    assert!(!branches.is_empty(), "expected branches");
    assert!(branches.iter().any(|b| b == "main"), "expected main");

    let commits = difiko::git::load_commits(&repo, "main", "HEAD")
        .await
        .expect("load_commits");
    let diff = difiko::git::load_diff(&repo, "main", "HEAD")
        .await
        .expect("load_diff");
    eprintln!(
        "branches: {}, commits: {}, files: {}",
        branches.len(),
        commits.len(),
        diff.len()
    );
}
