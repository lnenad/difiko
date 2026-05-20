use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use ratatui::style::Color;
use std::fs;
use std::path::{Path, PathBuf};

/// User-defined color overrides. Loaded once at startup from
/// `<config_dir>/theme.json`. Missing keys fall back to the built-in
/// defaults baked into the accessor functions in the parent module.
#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub bg: Option<Color>,
    pub fg: Option<Color>,
    pub dim: Option<Color>,
    pub accent: Option<Color>,
    pub accent_dim: Option<Color>,
    pub add: Option<Color>,
    pub del: Option<Color>,
    pub hunk: Option<Color>,
    pub add_bg: Option<Color>,
    pub del_bg: Option<Color>,
    pub add_bg_strong: Option<Color>,
    pub del_bg_strong: Option<Color>,
    pub status_add: Option<Color>,
    pub status_mod: Option<Color>,
    pub status_del: Option<Color>,
    pub status_ren: Option<Color>,
    pub highlight_bg: Option<Color>,
    pub highlight_bg_dim: Option<Color>,
    pub search_current_bg: Option<Color>,
    pub search_current_fg: Option<Color>,
    pub search_other_bg: Option<Color>,
    pub search_other_fg: Option<Color>,
}

impl Theme {
    /// Set the matching field by JSON key name. Returns false if the key
    /// doesn't correspond to a known field — the caller treats that as a
    /// user error worth surfacing.
    fn set(&mut self, key: &str, color: Color) -> bool {
        let slot: &mut Option<Color> = match key {
            "bg" => &mut self.bg,
            "fg" => &mut self.fg,
            "dim" => &mut self.dim,
            "accent" => &mut self.accent,
            "accent_dim" => &mut self.accent_dim,
            "add" => &mut self.add,
            "del" => &mut self.del,
            "hunk" => &mut self.hunk,
            "add_bg" => &mut self.add_bg,
            "del_bg" => &mut self.del_bg,
            "add_bg_strong" => &mut self.add_bg_strong,
            "del_bg_strong" => &mut self.del_bg_strong,
            "status_add" => &mut self.status_add,
            "status_mod" => &mut self.status_mod,
            "status_del" => &mut self.status_del,
            "status_ren" => &mut self.status_ren,
            "highlight_bg" => &mut self.highlight_bg,
            "highlight_bg_dim" => &mut self.highlight_bg_dim,
            "search_current_bg" => &mut self.search_current_bg,
            "search_current_fg" => &mut self.search_current_fg,
            "search_other_bg" => &mut self.search_other_bg,
            "search_other_fg" => &mut self.search_other_fg,
            _ => return false,
        };
        *slot = Some(color);
        true
    }
}

/// Result of loading theme.json. The theme is *always* populated (with
/// defaults for keys we couldn't apply); `issues` is a human-readable
/// list of per-key problems the caller should toast on startup so the
/// user knows their overrides aren't all being honoured.
#[derive(Debug, Default)]
pub struct ThemeLoad {
    pub theme: Theme,
    pub issues: Vec<String>,
}

/// Recognised color strings. Names match ratatui's `Color` variants
/// (case-insensitive), plus `#RRGGBB`, `rgb(r, g, b)`, and bare decimal
/// indices `0`..=`255` for the 256-color palette.
pub(super) fn parse_color(s: &str) -> Option<Color> {
    let trimmed = s.trim();

    // #RRGGBB
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }
    // rgb(r, g, b)
    if let Some(inner) = trimmed
        .strip_prefix("rgb(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 3 {
            let r: u8 = parts[0].parse().ok()?;
            let g: u8 = parts[1].parse().ok()?;
            let b: u8 = parts[2].parse().ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }
    // Decimal 0..=255 → indexed palette.
    if let Ok(n) = trimmed.parse::<u8>() {
        return Some(Color::Indexed(n));
    }
    // Named colors. Lowercased; `grey` is an alias for `gray`.
    let lower = trimmed.to_ascii_lowercase().replace("grey", "gray");
    Some(match lower.as_str() {
        "reset" | "default" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "darkgray" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => return None,
    })
}

/// Path to the theme override file. `None` only on platforms where
/// `ProjectDirs` can't resolve a config directory.
pub fn theme_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("dev", "local", "difiko")?;
    Some(dirs.config_dir().join("theme.json"))
}

/// Read and parse `theme.json` with per-key recovery. The file is
/// absent or unreadable? → defaults, no issues. The JSON is structurally
/// broken? → defaults + one issue describing the parse error. The JSON
/// parses but individual keys are unknown / wrong type / unknown color?
/// → keep the good keys, return per-key issues for the bad ones.
///
/// In every case the caller gets a usable `Theme`; nothing here propagates
/// a fatal error, so a malformed theme.json can never block startup.
pub fn load() -> ThemeLoad {
    let Some(path) = theme_path() else {
        return ThemeLoad::default();
    };
    load_from_path(&path)
}

fn load_from_path(path: &Path) -> ThemeLoad {
    let txt = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return ThemeLoad::default(),
        Err(e) => {
            return ThemeLoad {
                theme: Theme::default(),
                issues: vec![format!("could not read theme.json: {e}")],
            };
        }
    };
    parse_json(&txt)
}

fn parse_json(txt: &str) -> ThemeLoad {
    let value: serde_json::Value = match serde_json::from_str(txt) {
        Ok(v) => v,
        Err(e) => {
            return ThemeLoad {
                theme: Theme::default(),
                issues: vec![format!("theme.json is not valid JSON: {e}")],
            };
        }
    };
    let Some(obj) = value.as_object() else {
        return ThemeLoad {
            theme: Theme::default(),
            issues: vec!["theme.json must be a JSON object at the top level".into()],
        };
    };
    let mut theme = Theme::default();
    let mut issues = Vec::new();
    for (key, val) in obj {
        // Allow comment / metadata fields with a leading underscore so the
        // populated default template's `_comment` doesn't itself become an
        // issue.
        if key.starts_with('_') {
            continue;
        }
        let Some(s) = val.as_str() else {
            issues.push(format!("\"{key}\": value must be a string"));
            continue;
        };
        let Some(color) = parse_color(s) else {
            issues.push(format!("\"{key}\": unknown color {s:?}"));
            continue;
        };
        if !theme.set(key, color) {
            issues.push(format!("\"{key}\": unknown theme key"));
        }
    }
    ThemeLoad { theme, issues }
}

/// Write the *default* theme JSON to disk, creating the config directory
/// if needed. Returns the file path. Used by the "edit-theme" command to
/// give users a populated starting point (every key with its current
/// default) rather than an empty file. Refuses to overwrite an existing
/// file.
pub fn ensure_default_file() -> Result<PathBuf> {
    let path = theme_path().ok_or_else(|| anyhow!("could not resolve config dir"))?;
    if path.exists() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create config dir")?;
    }
    fs::write(&path, default_template()).context("writing default theme.json")?;
    Ok(path)
}

/// Hand-written JSON template with every overridable key set to its
/// built-in default. Users can delete any line to fall back to the
/// default; everything they leave is honoured.
fn default_template() -> &'static str {
    r#"{
  "_comment": "Edit this file to override difiko's theme. Restart difiko after saving. Delete any key to keep the default. Color values: a named color (black/red/green/yellow/blue/magenta/cyan/gray/darkgray/lightred/lightgreen/lightyellow/lightblue/lightmagenta/lightcyan/white), 'reset' for terminal default, '#RRGGBB' hex, 'rgb(r, g, b)' decimal, or 0..255 for the 256-color palette.",

  "fg": "reset",
  "bg": "reset",
  "dim": "darkgray",

  "accent": "cyan",
  "accent_dim": "blue",

  "add": "green",
  "del": "red",
  "hunk": "magenta",

  "add_bg": "rgb(15, 40, 15)",
  "del_bg": "rgb(55, 15, 15)",
  "add_bg_strong": "rgb(30, 95, 30)",
  "del_bg_strong": "rgb(140, 30, 30)",

  "status_add": "green",
  "status_mod": "yellow",
  "status_del": "red",
  "status_ren": "magenta",

  "highlight_bg": "rgb(40, 50, 70)",
  "highlight_bg_dim": "rgb(30, 35, 50)",

  "search_current_bg": "magenta",
  "search_current_fg": "white",
  "search_other_bg": "yellow",
  "search_other_fg": "black"
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_colors() {
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("CYAN"), Some(Color::Cyan));
        assert_eq!(parse_color("darkgray"), Some(Color::DarkGray));
        assert_eq!(parse_color("darkgrey"), Some(Color::DarkGray));
        assert_eq!(parse_color("reset"), Some(Color::Reset));
    }

    #[test]
    fn parses_hex_and_rgb() {
        assert_eq!(parse_color("#1e2a40"), Some(Color::Rgb(0x1e, 0x2a, 0x40)));
        assert_eq!(parse_color("rgb(15, 40, 15)"), Some(Color::Rgb(15, 40, 15)));
        assert_eq!(parse_color("rgb(0,0,0)"), Some(Color::Rgb(0, 0, 0)));
    }

    #[test]
    fn parses_indexed() {
        assert_eq!(parse_color("196"), Some(Color::Indexed(196)));
        assert_eq!(parse_color("0"), Some(Color::Indexed(0)));
    }

    #[test]
    fn rejects_unknown() {
        assert_eq!(parse_color("not-a-color"), None);
        assert_eq!(parse_color("#abc"), None); // 3-digit hex unsupported
        assert_eq!(parse_color("rgb(1, 2)"), None);
    }

    #[test]
    fn empty_json_yields_default_theme() {
        let load = parse_json("{}");
        assert!(load.theme.accent.is_none());
        assert!(load.issues.is_empty());
    }

    #[test]
    fn partial_override_keeps_other_fields_unset() {
        let json = r##"{ "accent": "magenta", "add": "#00ff00" }"##;
        let load = parse_json(json);
        assert_eq!(load.theme.accent, Some(Color::Magenta));
        assert_eq!(load.theme.add, Some(Color::Rgb(0, 0xff, 0)));
        assert!(load.theme.del.is_none());
        assert!(load.issues.is_empty());
    }

    #[test]
    fn unknown_color_keeps_other_keys() {
        // The bad "accent" entry is reported, but "add" still applies.
        let json = r#"{ "accent": "burgundy", "add": "green" }"#;
        let load = parse_json(json);
        assert!(load.theme.accent.is_none());
        assert_eq!(load.theme.add, Some(Color::Green));
        assert_eq!(load.issues.len(), 1);
        assert!(load.issues[0].contains("accent"));
        assert!(load.issues[0].contains("burgundy"));
    }

    #[test]
    fn unknown_key_is_reported() {
        let json = r#"{ "not_a_field": "red", "accent": "cyan" }"#;
        let load = parse_json(json);
        assert_eq!(load.theme.accent, Some(Color::Cyan));
        assert_eq!(load.issues.len(), 1);
        assert!(load.issues[0].contains("not_a_field"));
    }

    #[test]
    fn non_string_value_is_reported() {
        let json = r#"{ "accent": 42 }"#;
        let load = parse_json(json);
        assert!(load.theme.accent.is_none());
        assert_eq!(load.issues.len(), 1);
        assert!(load.issues[0].contains("accent"));
    }

    #[test]
    fn broken_json_yields_defaults_and_one_issue() {
        // Trailing comma — structurally invalid JSON.
        let load = parse_json(r#"{ "accent": "cyan", }"#);
        assert!(load.theme.accent.is_none());
        assert_eq!(load.issues.len(), 1);
        assert!(load.issues[0].contains("not valid JSON"));
    }

    #[test]
    fn comment_underscore_keys_are_ignored() {
        let json = r#"{ "_comment": "hi", "accent": "cyan" }"#;
        let load = parse_json(json);
        assert_eq!(load.theme.accent, Some(Color::Cyan));
        assert!(load.issues.is_empty());
    }

    #[test]
    fn top_level_array_is_reported() {
        let load = parse_json("[]");
        assert!(load.theme.accent.is_none());
        assert_eq!(load.issues.len(), 1);
        assert!(load.issues[0].contains("object"));
    }
}
