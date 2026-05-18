#[cfg(all(unix, not(target_os = "macos")))]
use anyhow::anyhow;
use anyhow::{Context, Result};
use std::env;
use std::path::Path;
use std::process::Command;

pub fn relaunch_in_new_window() -> Result<()> {
    let exe = env::current_exe().context("locating current executable")?;
    let args: Vec<String> = env::args()
        .skip(1)
        .filter(|a| a != "--new-window" && a != "-w")
        .collect();
    spawn_detached(&exe, &args)
}

#[cfg(target_os = "windows")]
fn spawn_detached(exe: &Path, args: &[String]) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // `cmd /c start "" <exe> <args...>` is the canonical way to spawn a
    // console child onto its own new window. CREATE_NO_WINDOW on the cmd
    // process keeps the helper invisible (start has already detached the
    // grandchild before cmd exits).
    Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("spawning new console window via cmd /c start")?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn spawn_detached(exe: &Path, args: &[String]) -> Result<()> {
    let cmdline = quote_posix(exe, args);
    let escaped = cmdline.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "tell application \"Terminal\"\n    do script \"{}\"\n    activate\nend tell",
        escaped
    );
    Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn()
        .context("invoking osascript to open Terminal")?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_detached(exe: &Path, args: &[String]) -> Result<()> {
    let mut candidates: Vec<(String, &'static [&'static str])> = Vec::new();
    if let Ok(t) = env::var("TERMINAL") {
        candidates.push((t, &["-e"]));
    }
    candidates.extend([
        ("x-terminal-emulator".to_string(), &["-e"][..]),
        ("gnome-terminal".to_string(), &["--"][..]),
        ("konsole".to_string(), &["-e"][..]),
        ("alacritty".to_string(), &["-e"][..]),
        ("kitty".to_string(), &[][..]),
        ("xterm".to_string(), &["-e"][..]),
    ]);

    for (term, flags) in &candidates {
        let mut cmd = Command::new(term);
        cmd.args(*flags);
        cmd.arg(exe);
        cmd.args(args);
        match cmd.spawn() {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e).context(format!("spawning {term}")),
        }
    }
    Err(anyhow!(
        "no terminal emulator found; set $TERMINAL or install one of: \
         x-terminal-emulator, gnome-terminal, konsole, alacritty, kitty, xterm"
    ))
}

#[cfg(unix)]
fn quote_posix(exe: &Path, args: &[String]) -> String {
    let mut out = shell_quote(&exe.to_string_lossy());
    for a in args {
        out.push(' ');
        out.push_str(&shell_quote(a));
    }
    out
}

#[cfg(unix)]
fn shell_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}
