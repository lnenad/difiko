//! Cross-platform "open this file in whatever the user has configured" —
//! Windows uses `cmd /c start`, macOS uses `open`, Linux uses `xdg-open`.
//! This launches a *graphical* handler (the user's default app for the
//! file's MIME type), not a terminal editor — running $EDITOR inside the
//! same terminal would fight our TUI for the screen.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn open_in_default_app(path: &Path) -> Result<()> {
    spawn(path).with_context(|| format!("opening {}", path.display()))
}

#[cfg(target_os = "windows")]
fn spawn(path: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW keeps the cmd.exe helper invisible. The empty
    // string after `start` is the (intentionally blank) window title —
    // omitting it makes `start` treat the next quoted argument as the
    // title instead of the file path.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("spawning cmd /c start")?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn spawn(path: &Path) -> Result<()> {
    Command::new("open")
        .arg(path)
        .spawn()
        .context("spawning open(1)")?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn(path: &Path) -> Result<()> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .context("spawning xdg-open (install xdg-utils?)")?;
    Ok(())
}
