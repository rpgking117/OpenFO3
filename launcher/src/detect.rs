//! Finding Fallout 3: the installers' registry keys, Steam's libraries and
//! the usual folders. Read-only.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::is_data_dir;

/// Every Fallout 3 `Data` folder found, best first, without repeats.
pub fn data_dirs() -> Vec<PathBuf> {
    let mut installs: Vec<PathBuf> = Vec::new();
    // The installers' own keys (retail, Steam, GOG write one of these).
    for key in [
        r"HKLM\SOFTWARE\WOW6432Node\Bethesda Softworks\Fallout3",
        r"HKLM\SOFTWARE\Bethesda Softworks\Fallout3",
    ] {
        if let Some(p) = registry_value(key, "Installed Path") {
            installs.push(PathBuf::from(p));
        }
    }
    for key in [
        r"HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\1454315831",
        r"HKLM\SOFTWARE\GOG.com\Games\1454315831",
    ] {
        if let Some(p) = registry_value(key, "path") {
            installs.push(PathBuf::from(p));
        }
    }
    // Steam's libraries.
    for library in steam_libraries() {
        for name in ["Fallout 3 goty", "Fallout 3"] {
            installs.push(library.join("steamapps").join("common").join(name));
        }
    }
    // The usual places.
    for p in [
        r"C:\GOG Games\Fallout 3 GOTY",
        r"C:\Program Files (x86)\Bethesda Softworks\Fallout 3",
        r"C:\Program Files\Bethesda Softworks\Fallout 3",
    ] {
        installs.push(PathBuf::from(p));
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for install in installs {
        let data = if is_data_dir(&install) {
            install
        } else {
            install.join("Data")
        };
        if is_data_dir(&data) && !out.iter().any(|d| same(d, &data)) {
            out.push(data);
        }
    }
    out
}

fn same(a: &Path, b: &Path) -> bool {
    let n = |p: &Path| p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    n(a) == n(b)
}

/// A registry value (as `reg query` prints it), if there is one.
fn registry_value(key: &str, name: &str) -> Option<String> {
    let mut cmd = Command::new("reg");
    cmd.args(["query", key, "/v", name]);
    hide_console(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix(name)?.trim_start();
        let rest = rest.strip_prefix("REG_SZ")?;
        Some(rest.trim().to_string())
    })
}

/// Steam's install folder and every library its `libraryfolders.vdf` lists.
fn steam_libraries() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for key in [r"HKCU\SOFTWARE\Valve\Steam", r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam"] {
        for name in ["SteamPath", "InstallPath"] {
            if let Some(p) = registry_value(key, name) {
                roots.push(PathBuf::from(p.replace('/', "\\")));
            }
        }
    }
    roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    let mut out: Vec<PathBuf> = Vec::new();
    for root in roots {
        let add = |out: &mut Vec<PathBuf>, p: PathBuf| {
            if p.is_dir() && !out.iter().any(|o| same(o, &p)) {
                out.push(p);
            }
        };
        add(&mut out, root.clone());
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = std::fs::read_to_string(vdf) {
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("\"path\"") {
                    let p = rest.trim().trim_matches('"').replace("\\\\", "\\");
                    add(&mut out, PathBuf::from(p));
                }
            }
        }
    }
    out
}

/// Runs a helper program without flashing a console window.
pub fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}
