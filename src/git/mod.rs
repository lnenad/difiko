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

/// Pseudo-ref label used in the Compare picker to mean "the on-disk working
/// tree" (uncommitted state). `[` is forbidden in real git refnames per
/// `git check-ref-format`, so this can never collide with a real branch/tag.
pub const WORKING_TREE_REF: &str = "[working tree]";

pub fn is_working_tree(r: &str) -> bool {
    r == WORKING_TREE_REF
}

pub async fn ensure_git_repo(repo: &Path) -> anyhow::Result<()> {
    let out = command::run(repo, &["rev-parse", "--is-inside-work-tree"]).await?;
    if out.trim() != "true" {
        anyhow::bail!("Path is not inside a git work tree: {}", repo.display());
    }
    Ok(())
}
