//! The launcher's settings, kept in `%APPDATA%\OpenFO3\launcher.toml`, and
//! the folders OpenFO3 uses. Nothing here touches the Fallout 3
//! installation or the original game's own settings and saves.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How the game's window is shown (the game's `--window`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

impl WindowMode {
    pub fn arg(self) -> &'static str {
        match self {
            WindowMode::Windowed => "windowed",
            WindowMode::Borderless => "borderless",
            WindowMode::Fullscreen => "fullscreen",
        }
    }
}

/// Where a game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Start {
    /// The game's main menu (continue, load, new game).
    #[default]
    MainMenu,
    /// Straight into a new game.
    NewGame,
}

/// One plugin in the load order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Fallout 3's `Data` folder.
    pub data_dir: Option<PathBuf>,
    /// The load order, in order.
    pub plugins: Vec<Plugin>,
    pub window: WindowMode,
    pub resolution: (u32, u32),
    pub vsync: bool,
    pub start: Start,
    pub subtitles: bool,
    pub skip_movies: bool,
    pub brightness: f32,
    /// Keep the launcher open while the game runs.
    pub keep_open: bool,
    /// The game's program, if not the one beside the launcher.
    pub game_exe: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            data_dir: None,
            plugins: Vec::new(),
            window: WindowMode::Windowed,
            resolution: (1920, 1080),
            vsync: false,
            start: Start::MainMenu,
            subtitles: false,
            skip_movies: false,
            brightness: 1.0,
            keep_open: false,
            game_exe: None,
        }
    }
}

/// `%APPDATA%\OpenFO3`: the launcher's settings and the plugin list it
/// hands the game.
pub fn config_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("OpenFO3")
}

/// `Documents\My Games\OpenFO3`: saves and the game's log, beside (never
/// inside) the original game's `My Games\Fallout3`.
pub fn user_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Documents")
        .join("My Games")
        .join("OpenFO3")
}

pub fn saves_dir() -> PathBuf {
    user_dir().join("Saves")
}

pub fn log_path() -> PathBuf {
    user_dir().join("openfo3.log")
}

fn config_path() -> PathBuf {
    config_dir().join("launcher.toml")
}

/// The plugin list written for the game (`--plugins`): the enabled ones,
/// one per line, as Fallout 3's own `plugins.txt`.
pub fn plugins_path() -> PathBuf {
    config_dir().join("plugins.txt")
}

impl Config {
    pub fn load() -> Config {
        std::fs::read_to_string(config_path())
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(config_path(), text).map_err(|e| e.to_string())
    }

    /// Writes the enabled plugins for the game.
    pub fn write_plugins(&self) -> Result<PathBuf, String> {
        std::fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
        let mut text = String::from("# Written by the OpenFO3 launcher.\r\n");
        for p in self.plugins.iter().filter(|p| p.enabled) {
            text.push_str(&p.name);
            text.push_str("\r\n");
        }
        let path = plugins_path();
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

/// Whether a folder is a Fallout 3 `Data` folder.
pub fn is_data_dir(dir: &Path) -> bool {
    dir.join("Fallout3.esm").is_file()
}
