//! Kleine Einstellungsdatei (merkt sich z. B. das Farbschema). Format: `schluessel=wert` je Zeile.

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Settings {
    pub dark: bool,
}

pub fn parse(text: &str) -> Settings {
    let mut s = Settings::default();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == "theme" {
                s.dark = v.trim().eq_ignore_ascii_case("dark");
            }
        }
    }
    s
}

pub fn render(s: &Settings) -> String {
    format!("theme={}\n", if s.dark { "dark" } else { "light" })
}

fn config_file() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }?;
    Some(base.join("Pyphi7").join("settings.txt"))
}

pub fn load() -> Settings {
    config_file().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| parse(&t)).unwrap_or_default()
}

pub fn save(s: &Settings) {
    if let Some(p) = config_file() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, render(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        assert!(parse(&render(&Settings { dark: true })).dark);
        assert!(!parse(&render(&Settings { dark: false })).dark);
        assert!(!parse("").dark);
        assert!(parse("x=1\ntheme = Dark\n").dark);
    }
}
