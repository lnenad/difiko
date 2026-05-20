<h1 align="center">difiko</h1>

<p align="center"><em>Review local git PRs from your terminal — no browser, no server, no <code>gh</code> dance.</em></p>

<p align="center">
  <a href="https://github.com/lnenad/difiko/releases/latest"><img src="https://img.shields.io/github/v/release/lnenad/difiko?style=flat-square&color=blue&label=release" alt="Latest release"></a>
  <a href="https://github.com/lnenad/difiko/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/lnenad/difiko/ci.yml?style=flat-square&label=ci" alt="CI"></a>
  <a href="https://github.com/lnenad/difiko/releases"><img src="https://img.shields.io/github/downloads/lnenad/difiko/total?style=flat-square&color=brightgreen" alt="Downloads"></a>
  <a href="#license"><img src="https://img.shields.io/badge/license-MIT_OR_Apache--2.0-blue?style=flat-square" alt="License"></a>
  <a href="#"><img src="https://img.shields.io/badge/built_with-Rust-orange?style=flat-square&logo=rust&logoColor=white" alt="Built with Rust"></a>
  <a href="https://github.com/lnenad/difiko/stargazers"><img src="https://img.shields.io/github/stars/lnenad/difiko?style=flat-square&color=yellow" alt="GitHub stars"></a>
</p>

Pick two refs, walk file diffs in a sidebar, mark files reviewed, filter by
commit, view blame. Works on any local repo — no remote setup, no auth, no
hosted UI. Single static binary.

<!-- TODO: record a demo with `vhs demo.tape` and commit demo.gif, then drop it here:
<p align="center"><img src="demo.gif" alt="difiko demo" width="780"></p>
-->

## Features

- **Side-by-side or unified diff**, with optional **syntect-powered syntax
  highlighting** and **character-level word-diff overlay**
- **Fuzzy file & branch pickers**, tree / flat sidebar, mark-reviewed state that
  survives navigation and restarts
- **Inline git blame gutter** — author + short hash next to every line, on demand
- **Per-commit diff filtering** from the commits panel, plus a
  `[working tree]` pseudo-ref for uncommitted changes
- **User-defined theme** via `theme.json` — open it from the app with **Ctrl+T**
- **Single binary**, ~4.5 MB stripped. No daemon, no IPC, no network. Just
  `git` via `tokio::process::Command`.

## Install

### macOS / Linux

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/lnenad/difiko/releases/latest/download/difiko-installer.sh | sh
```

### Homebrew

```sh
brew install lnenad/tap/difiko
```

### Windows (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/lnenad/difiko/releases/latest/download/difiko-installer.ps1 | iex"
```

### From source

```sh
cargo install --git https://github.com/lnenad/difiko
```

Pre-built archives for `x86_64-{linux,windows,macos}` and `aarch64-{linux,macos}` are attached to every [GitHub Release](https://github.com/lnenad/difiko/releases).

## Quick start

```sh
# from inside any git repo
difiko

# or skip the setup screen entirely with CLI args
difiko --repo /path/to/repo --base main --compare HEAD

# or jump directly into fullscreen review of one file
difiko --repo /path/to/repo --base main --compare HEAD \
       --file src/foo.rs --fullscreen
```

CLI flags:

| Flag                    | Meaning                                                     |
| ----------------------- | ----------------------------------------------------------- |
| `--repo <PATH>`         | Path to a local git repo. Defaults to cwd if it's a repo.   |
| `--base <REF>`          | Base ref (branch, tag, hash). Optional.                     |
| `--compare <REF>`       | Compare ref. Optional.                                      |
| `--file <PATH>`         | Auto-select this file in the sidebar on launch.             |
| `--fullscreen`          | Open `--file` in fullscreen mode immediately.               |
| `--no-remote-branches`  | Hide remote-tracking branches from the picker.              |
| `--new-window` / `-w`   | Re-launch in a new terminal window and exit this process.   |
| `--no-word-diff`        | Disable per-line word-level diff highlighting.              |
| `--no-syntax`           | Disable syntax highlighting.                                |

If `--repo`, `--base`, and `--compare` are all valid, the Setup screen is
skipped and the diff loads immediately.

### Working-tree compare

The Compare branch picker has a `[working tree]` entry pinned at the top.
Pick it to diff your **uncommitted on-disk state** (staged + unstaged) against
the chosen base — useful for reviewing changes before committing. The Commits
panel is empty in this mode (working tree has no commits by definition); blame
falls back to `HEAD`.

## Build

Requires the Rust 2021 toolchain (1.70+ should be fine; tested on 1.95).

```sh
cargo build --release
# binary at target/release/difiko (~4.5 MB stripped — syntect carries bundled syntaxes)
```

```sh
cargo test                  # unit + integration tests
cargo test --lib            # unit tests only
```

## Three screens

### Setup

Pick the repository, base, and compare refs.

- If the launch directory (or `--repo`) is already a valid git repo, focus
  starts on **Base** so you can dive straight into branch selection. Otherwise
  focus starts on the Repository field.
- If `--base` is passed and the active HEAD branch is different from it, the
  Setup screen is skipped entirely: compare is set to HEAD and the diff loads
  immediately. If `--base` is passed but HEAD == base (so we can't infer a
  compare), focus jumps to the **Compare** field so the picker is one
  keystroke away.
- The Repository field has a directory autocomplete dropdown:
  - **Type** to filter matches at the current path level
  - **↑ / ↓** cycle highlighted matches (re-shows the dropdown if hidden)
  - **Enter** with the dropdown visible accepts the highlight; pressed again
    once hidden, loads branches
  - **Shift-→** is a power-user accept (also hides the dropdown)
  - `~`, absolute paths, and relative-to-cwd paths all work
- The Base / Compare fields open a fuzzy branch picker when you press
  **Enter**, **Space**, or any printable character. The picker lands the
  cursor on the currently selected branch (with a `▶` marker).
- When branches load, **base** auto-fills to `main` / `master` and **compare**
  auto-fills to the active HEAD branch (when they're different). Changing the
  Repository path clears the prior preselection so the new repo gets fresh
  defaults.
- After picking the base branch, the compare picker opens automatically as the
  next step.

### Review

Three panes — Sidebar | Diff | Commits — with focused-pane border highlighting.

```
┌ repo: foo  base: main → feature/x   files: 12  +234 -45  reviewed: 3/12 ┐
│┌─ Files (Tree) ─────────┐┌─ src/components/ReviewSidebar.jsx  M  +42 -8 ┐│
││ [✓] App.jsx           ││ @@ -10,7 +10,7 @@                            ││
││  └─ src/              ││  const [collapsed, setCollapsed] = ...       ││
││     └─ components/    ││ -return (                                    ││
││         ReviewSidebar.││ +return useMemo(() => (                      ││
│└────────────────────────┘└────────────────────────────────────────────┘│
│┌─ Commits (3) ─────────────────────────────────────────────────────────┐│
││  abc1234  Refactor sidebar           nenad   2026-05-04                ││
│└───────────────────────────────────────────────────────────────────────┘│
└ j/k:scroll  J/K:next file  m:reviewed  b:blame  u:split  W:word  S:syntax ─┘
```

### Fullscreen

Single-file view with prev/next file navigation. Same diff renderer, no
sidebar/commits.

## Keybindings

Press **`?`** at any time for the in-app help overlay — it lists every binding
in context. The highlights:

| Key         | Action                                                |
| ----------- | ----------------------------------------------------- |
| `?` / F1    | Toggle help overlay                                   |
| `:`         | Command palette (Review / Fullscreen)                 |
| `Ctrl-t`    | Edit `theme.json` in your OS default app              |
| `Ctrl-c`    | Quit (any screen)                                     |
| `j` / `k`   | Move / scroll                                         |
| `Tab`       | Cycle pane focus (Sidebar → Diff → Commits)           |
| `m`         | Mark current file reviewed                            |
| `u`         | Toggle Unified / Split diff                           |
| `F`         | Fullscreen current file                               |
| `/`         | Fuzzy filter files                                    |
| `Esc`       | Close modal → exit Fullscreen → back to Setup → reset |

<details>
<summary><strong>Full cheat sheet</strong> — every binding by screen / pane</summary>

### Setup — Repository field

| Key             | Action                                        |
| --------------- | --------------------------------------------- |
| type / Backspace| Edit path                                     |
| ↑ / ↓           | Cycle directory matches                       |
| Shift-→         | Accept highlighted directory                  |
| Enter (dropdown visible)   | Accept highlighted directory       |
| Enter (dropdown hidden)    | Load branches at this path         |
| Tab / Shift-Tab | Next / previous field                         |

### Review — pane navigation

| Key             | Action                                        |
| --------------- | --------------------------------------------- |
| Tab / Shift-Tab | Cycle pane focus                              |
| 1 / 2 / 3       | Focus Sidebar / Diff / Commits directly       |

### Review — Sidebar focused

| Key       | Action                                                     |
| --------- | ---------------------------------------------------------- |
| j/k, ↓/↑  | Move selection                                             |
| g / G     | Top / bottom                                               |
| Enter / l | Open file in diff pane                                     |
| Space     | Toggle folder collapse (Tree mode)                         |
| t         | Toggle Tree / Flat sidebar                                 |
| /         | Fuzzy filter files                                         |

### Review — Diff focused

| Key            | Action                              |
| -------------- | ----------------------------------- |
| j/k, ↓/↑       | Scroll one line vertically          |
| h/l, ←/→       | Scroll 8 columns horizontally       |
| PgUp / PgDn    | Page scroll                         |
| Ctrl-d/Ctrl-u  | Half-page scroll                    |
| Ctrl-f/Ctrl-b  | Page scroll                         |
| Ctrl-j/Ctrl-k  | Page scroll                         |
| Ctrl-↓/Ctrl-↑  | Page scroll                         |
| g / G          | Top / bottom of file                |
| J / K          | Next / previous file                |

### Review — modeless actions

| Key | Action                                       |
| --- | -------------------------------------------- |
| m   | Mark current file reviewed                   |
| b   | Toggle git blame gutter                      |
| u   | Toggle Unified / Split diff                  |
| W   | Toggle word-level diff highlighting          |
| S   | Toggle syntax highlighting                   |
| F   | Open current file in fullscreen              |
| c   | Toggle commits panel                         |
| r   | Reload diff                                  |
| B   | Open branch picker                           |
| R   | Clear reviewed set (press twice within 2s to confirm) |

### Review — Commits focused

| Key   | Action                                            |
| ----- | ------------------------------------------------- |
| Enter | Toggle filter-by-this-commit                      |
| Space | Expand/collapse commit body                       |
| c     | Close commits panel                               |

### Fullscreen

| Key            | Action                              |
| -------------- | ----------------------------------- |
| j/k, ↓/↑       | Scroll one line vertically          |
| h/l, ←/→       | Scroll 8 columns horizontally       |
| PgUp / PgDn    | Page scroll                         |
| Ctrl-d/u, Ctrl-j/k, Ctrl-↓/↑ | Page / half-page scroll |
| g / G          | Top / bottom                        |
| J / Space      | Next file                           |
| K              | Previous file                       |
| m              | Toggle reviewed                     |
| u              | Toggle Unified / Split              |
| b              | Toggle git blame gutter             |
| W              | Toggle word diff                    |
| S              | Toggle syntax highlighting          |
| q / Esc        | Exit fullscreen                     |

</details>

## Config

User preferences (word diff on/off, syntax on/off) persist to:

```
~/Library/Application Support/dev.local.difiko/config.json   # macOS
~/.config/difiko/config.json                                 # Linux
%APPDATA%\local\difiko\config\config.json                    # Windows
```

Toggle at runtime with **W** (word diff) and **S** (syntax). Both default
on. CLI flags `--no-word-diff` and `--no-syntax` override the file for a
single session without persisting.

### Theme

Colors are overridable via `theme.json`, alongside `config.json` in the
same directory. Open it with **Ctrl+T** (or the `edit-theme` entry in the
`:` command palette) — difiko writes a populated default file (every key
with its built-in value), opens it in your OS default app for `.json`
files, and toasts a reminder that **changes require a restart**.

Every value is a color string: a ratatui name (`cyan`, `darkgray`,
`lightred`, ...), `reset` for the terminal default, `#RRGGBB` hex,
`rgb(r, g, b)` decimal, or a `0..255` index for the 256-color palette.
Delete any key to keep the default; an unrecognised value in any field
falls back to the whole defaults and surfaces a toast on next launch.

## Persistence

Reviewed-files state is saved to:

```
~/Library/Application Support/dev.local.difiko/state.json   # macOS
~/.local/share/difiko/state.json                            # Linux
```

Keyed by `<repo>::<base>::<compare>`. The set is invalidated automatically if
the file list changes (sha256 of sorted paths is stored as a snapshot). Nothing
else persists — branches and diffs are re-fetched from git each session.

The state file is versioned and written atomically (temp + rename). If it's
ever unparseable on launch, it's renamed to `state.json.broken-<unix-ts>` and
a fresh state is started; a toast on first frame surfaces the rename so you
notice rather than silently losing data.

## Implementation notes

- **Single binary**: Rust + `ratatui` + `crossterm` + `tokio` + `clap`. No
  network, no server, no IPC. Talks directly to local `git` via
  `tokio::process::Command`.
- **Syntax highlighting** via `syntect` (default themes bundled). Each file's
  diff content is highlighted in a single pass so multi-line constructs
  (block comments, multi-line strings) keep context across lines. Output
  is cached per-file until the diff reloads or the user toggles syntax
  off. When syntax is on, add/del rows get a subtle dark green/red
  background tint so they stay visually distinct from context lines.
- **Word diff** via `similar` at **character** granularity. Adjacent
  `-`/`+` lines are paired positionally within each run; the renderer
  paints a stronger bg overlay (over the subtle line tint) on the
  precise bytes that changed — same two-tier look as delta / Claude
  Code. Char-level is used instead of word-level because the word
  tokenizer treats quoted strings and dotted numbers as single tokens,
  which would highlight all of `"0.1.3"` instead of just the `3`.
- **Async**: git operations don't block the UI; results flow back through an
  `mpsc` channel and are matched against monotonic request IDs so stale results
  are discarded after navigation.
- **Diff rendering**: parses `git diff` output and renders each line as styled
  ratatui spans. Split mode pairs `-` and `+` lines side-by-side.
- **Blame**: `git blame --porcelain <ref>` parsed into a `HashMap<u32,
  BlameLine>`; gutter is rendered with author + 7-char hash. Loaded lazily,
  cached per (ref, file).

See `AGENTS.md` for module-level architecture and conventions.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
