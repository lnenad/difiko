use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "difiko", about = "Keyboard-driven TUI for reviewing local git PRs")]
pub struct Cli {
    /// Path to a local git repository (defaults to current directory if it is a git repo).
    #[arg(long)]
    pub repo: Option<PathBuf>,

    /// Base branch (or any git ref).
    #[arg(long)]
    pub base: Option<String>,

    /// Compare branch (or any git ref).
    #[arg(long)]
    pub compare: Option<String>,

    /// File path to focus when launching directly into the review screen.
    #[arg(long)]
    pub file: Option<String>,

    /// Open the focused file in fullscreen review mode.
    #[arg(long, default_value_t = false)]
    pub fullscreen: bool,

    /// Hide remote-tracking branches from the branch picker.
    #[arg(long, default_value_t = false)]
    pub no_remote_branches: bool,
}
