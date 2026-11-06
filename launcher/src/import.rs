//! Importing from the original game: its display and subtitle settings
//! (`Documents\My Games\Fallout3\FalloutPrefs.ini`) and its active plugins
//! (`%LOCALAPPDATA%\Fallout3\plugins.txt`). Both are only read.

use std::path::PathBuf;

/// What the original game's settings say.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Imported {
    pub resolution: Option<(u32, u32)>,
    pub fullscreen: Option<bool>,
    pub subtitles: Option<bool>,
    pub plugins: Option<Vec<String>>,
}

pub fn prefs_path() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("Documents")
        .join("My Games")
        .join("Fallout3")
        .join("FalloutPrefs.ini")
}

pub fn plugins_txt_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("Fallout3")
        .join("plugins.txt")
}

/// Reads both files (whichever exist).
pub fn read() -> Imported {
    let mut out = std::fs::read(prefs_path())
        .map(|b| from_prefs(&String::from_utf8_lossy(&b)))
        .unwrap_or_default();
    out.plugins = std::fs::read(plugins_txt_path())
        .ok()
        .map(|b| parse_plugins(&String::from_utf8_lossy(&b)));
    out
}

/// The display and subtitle settings of a `FalloutPrefs.ini`.
pub fn from_prefs(text: &str) -> Imported {
    let mut section = String::new();
    let (mut w, mut h) = (None, None);
    let mut out = Imported::default();
    for line in text.lines() {
        let line = line.trim();
        if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = s.to_ascii_lowercase();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
        let number = || value.parse::<i64>().ok();
        match (section.as_str(), key.as_str()) {
            ("display", "isize w") => w = number().map(|n| n as u32),
            ("display", "isize h") => h = number().map(|n| n as u32),
            ("display", "bfull screen") => out.fullscreen = number().map(|n| n != 0),
            ("interface", "bgeneralsubtitles") => out.subtitles = number().map(|n| n != 0),
            _ => {}
        }
    }
    if let (Some(w), Some(h)) = (w, h) {
        if w > 0 && h > 0 {
            out.resolution = Some((w, h));
        }
    }
    out
}

/// A `plugins.txt`'s names (lines starting with `#` skipped).
pub fn parse_plugins(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_original_games_prefs() {
        let ini = "[Display]\r\niSize W=2560\r\niSize H=1440\r\nbFull Screen=1\r\n\
                   [Interface]\r\nbGeneralSubtitles=0\r\n";
        let i = from_prefs(ini);
        assert_eq!(i.resolution, Some((2560, 1440)));
        assert_eq!(i.fullscreen, Some(true));
        assert_eq!(i.subtitles, Some(false));
        assert_eq!(
            parse_plugins("# comment\r\nFallout3.esm\r\n\r\nZeta.esm\r\n"),
            ["Fallout3.esm", "Zeta.esm"]
        );
    }
}
