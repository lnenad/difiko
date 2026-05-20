use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use ratatui::style::Color;
use serde::de::{self, Deserializer};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// User-defined color overrides. Loaded once at startup from
/// `<config_dir>/theme.json`. Missing keys fall back to the built-in
/// defaults baked into the accessor functions in the parent module.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub bg: Option<Col>,
    pub fg: Option<Col>,
    pub dim: Option<Col>,
    pub accent: Option<Col>,
    pub accent_dim: Option<Col>,
    pub add: Option<Col>,
    pub del: Option<Col>,
    pub hunk: Option<Col>,
    pub add_bg: Option<Col>,
    pub del_bg: Option<Col>,
    pub add_bg_strong: Option<Col>,
    pub del_bg_strong: Option<Col>,
    pub status_add: Option<Col>,
    pub status_mod: Option<Col>,
    pub status_del: Option<Col>,
    pub status_ren: Option<Col>,
    pub highlight_bg: Option<Col>,
    pub highlight_bg_dim: Option<Col>,
    pub search_current_bg: Option<Col>,
    pub search_current_fg: Option<Col>,
    pub search_other_bg: Option<Col>,
    pub search_other_fg: Option<Col>,
}

/// Wrapper so JSON strings like `"cyan"` / `"#1e2a40"` / `"rgb(30, 40, 50)"`
/// land as a `ratatui::style::Color` via the helper deserializer below.
#[derive(Debug, Clone, Copy)]
pub struct Col(pub Color);

impl<'de> Deserialize<'de> for Col {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_color(&s)
            .map(Col)
            .ok_or_else(|| de::Error::custom(format!("unknown color: {s:?}")))
    }
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

/// Read and parse `theme.json`. Returns `Theme::default()` if the file is
/// absent or unreadable; returns an error only for malformed JSON so the
/// caller can surface a toast.
pub fn load() -> Result<Theme> {
    let Some(path) = theme_path() else {
        return Ok(Theme::default());
    };
    let txt = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Theme::default()),
        Err(e) => return Err(e).context("reading theme.json"),
    };
    serde_json::from_str(&txt).context("parsing theme.json")
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
        let t: Theme = serde_json::from_str("{}").unwrap();
        assert!(t.accent.is_none());
        assert!(t.add.is_none());
    }

    #[test]
    fn partial_override_keeps_other_fields_unset() {
        let json = r##"{ "accent": "magenta", "add": "#00ff00" }"##;
        let t: Theme = serde_json::from_str(json).unwrap();
        assert_eq!(t.accent.map(|c| c.0), Some(Color::Magenta));
        assert_eq!(t.add.map(|c| c.0), Some(Color::Rgb(0, 0xff, 0)));
        assert!(t.del.is_none());
    }

    #[test]
    fn unknown_color_in_known_field_is_an_error() {
        let json = r#"{ "accent": "burgundy" }"#;
        let r: serde_json::Result<Theme> = serde_json::from_str(json);
        assert!(r.is_err());
    }
}
