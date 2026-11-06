//! The plugins in a `Data` folder, kept in the launcher's load order.

use std::path::Path;

use crate::config::Plugin;

/// Fallout 3's official masters, in the game's own order.
pub const OFFICIAL: [&str; 6] = [
    "Fallout3.esm",
    "Anchorage.esm",
    "ThePitt.esm",
    "BrokenSteel.esm",
    "PointLookout.esm",
    "Zeta.esm",
];

pub fn is_official(name: &str) -> bool {
    OFFICIAL.iter().any(|o| o.eq_ignore_ascii_case(name))
}

/// The `.esm` and `.esp` files in `data`, masters first.
pub fn scan(data: &Path) -> Vec<String> {
    let mut masters = Vec::new();
    let mut others = Vec::new();
    if let Ok(entries) = std::fs::read_dir(data) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".esm") {
                masters.push(name);
            } else if lower.ends_with(".esp") {
                others.push(name);
            }
        }
    }
    // The official masters in their order, then the others by name.
    masters.sort_by_key(|n| {
        (
            OFFICIAL
                .iter()
                .position(|o| o.eq_ignore_ascii_case(n))
                .unwrap_or(usize::MAX),
            n.to_ascii_lowercase(),
        )
    });
    others.sort_by_key(|n| n.to_ascii_lowercase());
    masters.extend(others);
    masters
}

/// The load order kept as it was for the files still there, new ones added
/// at the end (official ones on, others off), and Fallout3.esm always on.
pub fn merge(kept: &[Plugin], found: &[String]) -> Vec<Plugin> {
    let mut out: Vec<Plugin> = kept
        .iter()
        .filter(|p| found.iter().any(|f| f.eq_ignore_ascii_case(&p.name)))
        .cloned()
        .collect();
    for f in found {
        if !out.iter().any(|p| p.name.eq_ignore_ascii_case(f)) {
            out.push(Plugin {
                name: f.clone(),
                enabled: is_official(f),
            });
        }
    }
    for p in &mut out {
        if p.name.eq_ignore_ascii_case("Fallout3.esm") {
            p.enabled = true;
        }
    }
    // Masters stay ahead of plugins, as the game loads them.
    out.sort_by_key(|p| !p.name.to_ascii_lowercase().ends_with(".esm"));
    out
}

/// A load order from a `plugins.txt` (the original game's, read only): the
/// listed ones on and in its order, then the rest off.
pub fn from_list(list: &[String], found: &[String]) -> Vec<Plugin> {
    let mut out: Vec<Plugin> = Vec::new();
    for name in list {
        if let Some(f) = found.iter().find(|f| f.eq_ignore_ascii_case(name)) {
            if !out.iter().any(|p| p.name.eq_ignore_ascii_case(f)) {
                out.push(Plugin {
                    name: f.clone(),
                    enabled: true,
                });
            }
        }
    }
    let rest: Vec<Plugin> = found
        .iter()
        .filter(|f| !out.iter().any(|p| p.name.eq_ignore_ascii_case(f)))
        .map(|f| Plugin {
            name: f.clone(),
            enabled: false,
        })
        .collect();
    out.extend(rest);
    merge(&out, found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[Plugin]) -> Vec<(&str, bool)> {
        v.iter().map(|p| (p.name.as_str(), p.enabled)).collect()
    }

    #[test]
    fn new_files_join_official_on_others_off() {
        let found: Vec<String> = ["Fallout3.esm", "Anchorage.esm", "MyMod.esp"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let order = merge(&[], &found);
        assert_eq!(
            names(&order),
            [("Fallout3.esm", true), ("Anchorage.esm", true), ("MyMod.esp", false)]
        );
        // Kept as the player left it; a file gone is dropped; Fallout3.esm
        // stays on.
        let kept = vec![
            Plugin { name: "MyMod.esp".into(), enabled: true },
            Plugin { name: "Gone.esp".into(), enabled: true },
            Plugin { name: "Fallout3.esm".into(), enabled: false },
        ];
        let order = merge(&kept, &found);
        assert_eq!(
            names(&order),
            [("Fallout3.esm", true), ("Anchorage.esm", true), ("MyMod.esp", true)]
        );
    }

    #[test]
    fn the_games_own_list_is_followed() {
        let found: Vec<String> = ["Fallout3.esm", "Zeta.esm", "ThePitt.esm", "A.esp"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let list: Vec<String> = ["Fallout3.esm", "zeta.esm"].iter().map(|s| s.to_string()).collect();
        let order = from_list(&list, &found);
        assert_eq!(
            names(&order),
            [
                ("Fallout3.esm", true),
                ("Zeta.esm", true),
                ("ThePitt.esm", false),
                ("A.esp", false)
            ]
        );
    }
}
