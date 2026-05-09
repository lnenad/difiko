pub mod blame;
pub mod branches;
pub mod command;
pub mod commits;
pub mod diff;
pub mod parse;

pub use blame::{load_blame, Blame};
pub use branches::list_branches;
pub use commits::load_commits;
pub use diff::{load_commit_diff, load_diff};

use std::path::Path;

pub async fn ensure_git_repo(repo: &Path) -> anyhow::Result<()> {
    let out = command::run(repo, &["rev-parse", "--is-inside-work-tree"]).await?;
    if out.trim() != "true" {
        anyhow::bail!("Path is not inside a git work tree: {}", repo.display());
    }
    Ok(())
}
