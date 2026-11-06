//! Loading screens (`LSCR`): the pictures the loading menu and the main menu
//! show, from `Fallout3.exe` 1.7.0.3.
//!
//! The loading menu (`LoadingMenu`, `menus\loading_menu.xml`) keeps a list of
//! them (`+0xe8`). `00659400` fills it. On the main menu (`+0x12a` bit 5: the
//! start menu up and not the pause menu) `00658b60` takes every `LSCR` whose
//! record flags have bit 10 (`0x400`, "Displays in Main Menu"); in the game it
//! takes those whose conditions hold for the place being loaded (not done
//! here). `00658050` then picks from them at random, without repeats, until the
//! list holds `iMaxScreens_MainMenu` (20) on the main menu, else `iMaxScreens`
//! (4). Each one's `ICON` is a picture under `Textures\` (`00658960`).
//!
//! In the game (`00659400`), with somewhere being loaded (the menu's +0x100):
//! the screens whose locations (`LNAM`) name it go in first, `[Loading]
//! iNumLocationSpecificScreens` (1) of them, then screens with no location until
//! the list holds `iMaxScreens` (4). After a script's
//! `SetLocationSpecificLoadScreensOnly 1` (`01071b70`; the Vault 101 quests
//! set it) only the location's own, up to `iMaxScreens`. With nowhere given,
//! the ones with no location. A picture already in the list isn't taken again
//! (`00657ec0`). Every `LNAM` in the game's plugins names an interior cell or a
//! worldspace itself (its other fields 0), so a place outdoors, which the game
//! matches by worldspace and grid (`00516010`), matches none of them.
//!
//! The first start's welcome screens (`sWelcomeScreen1` .. `4`, then
//! `Interface\Main\` + `sMainMenuBackground`) aren't records and aren't here:
//! they show while the game's files load, which happens here before there's a
//! window.

use esm::{FormId, FourCC, LoadOrder};

const LSCR: FourCC = FourCC::new(b"LSCR");
const ICON: FourCC = FourCC::new(b"ICON");
const DESC: FourCC = FourCC::new(b"DESC");
const LNAM: FourCC = FourCC::new(b"LNAM");

/// `[Loading] iMaxScreens` and `iNumLocationSpecificScreens` (this install's
/// `Fallout.ini`).
pub const IN_GAME_MOST: usize = 4;
pub const LOCATION_MOST: usize = 1;

/// Where a load goes, for its loading screens (`00516010`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// An interior cell.
    Cell(FormId),
    /// A worldspace, as itself.
    World(FormId),
    /// Somewhere outdoors (no `LNAM` in the game matches it).
    Outdoors,
}

/// The record flag of a loading screen shown on the main menu (`00658b60`
/// tests the form's flags `>> 10 & 1`).
pub const MAIN_MENU_FLAG: u32 = 0x400;

/// How many the main menu keeps (`[Loading] iMaxScreens_MainMenu`).
pub const MAIN_MENU_MOST: usize = 20;

/// One loading screen.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadScreen {
    pub form: FormId,
    /// The picture as the renderer finds it: `textures\` + the record's `ICON`,
    /// lower case.
    pub picture: String,
    /// The text under it in the game's loading screens (`DESC`).
    pub text: String,
}

/// Every loading screen the main menu may show, in load order.
pub fn main_menu(order: &LoadOrder) -> Vec<LoadScreen> {
    let mut out = Vec::new();
    for rr in order.records_of_type(LSCR) {
        let id = rr.form_id;
        if rr.entry.header.flags & MAIN_MENU_FLAG == 0 {
            continue;
        }
        let Ok(record) = rr.record() else {
            continue;
        };
        let Some(icon) = record.get(ICON).map(|s| s.zstring()).filter(|s| !s.is_empty()) else {
            continue;
        };
        out.push(LoadScreen {
            form: id,
            picture: format!("textures\\{}", icon.trim().to_ascii_lowercase()),
            text: record.get(DESC).map(|s| s.zstring()).unwrap_or_default(),
        });
    }
    out
}

/// Every loading screen with a picture, with the places it's for (`LNAM`).
fn all_screens(order: &LoadOrder) -> Vec<(LoadScreen, Vec<FormId>)> {
    let mut out = Vec::new();
    for rr in order.records_of_type(LSCR) {
        let Ok(record) = rr.record() else {
            continue;
        };
        let Some(icon) = record.get(ICON).map(|s| s.zstring()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let places = record
            .get_all(LNAM)
            .filter(|s| s.data.len() >= 4)
            .map(|s| rr.plugin.to_global(FormId(u32::from_le_bytes([s.data[0], s.data[1], s.data[2], s.data[3]]))))
            .filter(|f| f.0 != 0)
            .collect();
        out.push((
            LoadScreen {
                form: rr.form_id,
                picture: format!("textures\\{}", icon.trim().to_ascii_lowercase()),
                text: record.get(DESC).map(|s| s.zstring()).unwrap_or_default(),
            },
            places,
        ));
    }
    out
}

/// The loading screens a load in the game shows (see the module notes):
/// `location_only` is `SetLocationSpecificLoadScreensOnly`'s flag; `roll(n)`
/// gives a number below `n`.
pub fn in_game(
    order: &LoadOrder,
    to: Option<Destination>,
    location_only: bool,
    mut roll: impl FnMut(usize) -> usize,
) -> Vec<LoadScreen> {
    let all = all_screens(order);
    let mut out: Vec<LoadScreen> = Vec::new();
    let mut take = |out: &mut Vec<LoadScreen>, mut from: Vec<LoadScreen>, most: usize| {
        while out.len() < most && !from.is_empty() {
            let i = roll(from.len()).min(from.len() - 1);
            let s = from.swap_remove(i);
            if !out.iter().any(|o| o.picture.eq_ignore_ascii_case(&s.picture)) {
                out.push(s);
            }
        }
    };
    let generic = || -> Vec<LoadScreen> {
        all.iter()
            .filter(|(_, places)| places.is_empty())
            .map(|(s, _)| s.clone())
            .collect()
    };
    let place = match to {
        Some(Destination::Cell(f) | Destination::World(f)) => Some(f),
        _ => None,
    };
    match to {
        Some(_) => {
            let here: Vec<LoadScreen> = all
                .iter()
                .filter(|(_, places)| place.is_some_and(|p| places.contains(&p)))
                .map(|(s, _)| s.clone())
                .collect();
            let most = if !location_only && LOCATION_MOST <= IN_GAME_MOST {
                LOCATION_MOST
            } else {
                IN_GAME_MOST
            };
            take(&mut out, here, most);
            if out.len() < IN_GAME_MOST && !location_only {
                take(&mut out, generic(), IN_GAME_MOST);
            }
        }
        None => take(&mut out, generic(), IN_GAME_MOST),
    }
    out
}

/// `00658050`: up to `most` of them in a random order, none twice. `roll(n)`
/// gives a number below `n` (the game's random number, `0086c4b0`).
pub fn pick(
    mut screens: Vec<LoadScreen>,
    most: usize,
    mut roll: impl FnMut(usize) -> usize,
) -> Vec<LoadScreen> {
    let mut out = Vec::new();
    while out.len() < most && !screens.is_empty() {
        let i = roll(screens.len()).min(screens.len() - 1);
        out.push(screens.swap_remove(i));
    }
    out
}
