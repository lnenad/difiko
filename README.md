# difiko

A keyboard-driven terminal UI for reviewing local git PRs. Pick two refs, walk
file diffs in a sidebar, mark files reviewed, filter by commit, and view blame —
all without leaving your terminal. Single static binary, no server.

This is a Rust replacement for the React + Express web app in the parent
directory. The web UI is still available; this crate is self-contained.

## Quick start

```sh
# from inside any git repo
cargo run --release

# or skip the setup screen entirely with CLI args
cargo run --release -- --repo /path/to/repo --base main --compare HEAD

# or jump directly into fullscreen review of one file
cargo run --release -- \
  --repo /path/to/repo --base main --compare HEAD \
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

If `--repo`, `--base`, and `--compare` are all valid, the Setup screen is
skipped and the diff loads immediately.

## Build

Requires the Rust 2021 toolchain (1.70+ should be fine; tested on 1.95).

```sh
cargo build --release
# binary at target/release/difiko (~1.6 MB stripped)
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
└ j/k:scroll  J/K:next file  m:reviewed  b:blame  u:split  F:fullscreen ?─┘
```

### Fullscreen

Single-file view with prev/next file navigation. Same diff renderer, no
sidebar/commits.

## Keybinding cheat sheet

Press **`?`** at any time for the in-app help overlay.

### Global

| Key         | Action                                                |
| ----------- | ----------------------------------------------------- |
| `?` / F1    | Toggle help overlay                                   |
| `:`         | Command palette (Review / Fullscreen)                 |
| `Ctrl-c`    | Quit (any screen)                                     |
| `q`         | Quit (Review screen; exits Fullscreen)                |
| `Esc`       | Close modal → exit Fullscreen → soft-back to Setup (Compare focused, state kept) → full reset |

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
| q / Esc        | Exit fullscreen                     |

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

Same as parent project.
