//! The Pip-Boy's miscellaneous statistics (`GetPCMiscStat`,
//! `ModPCMiscStat`), by Fallout 3's own names and numbers: 26 of them
//! (`0048c6b0` makes them from the names at `00f4ca80`). The game bumps them
//! where things happen: 1 on finding a place, 4 on each lock picked, 5 on each
//! terminal hacked, 11 on a mine disarmed, 13 on a visit's first pocket
//! picked, 15 on each skill book read (`world::living` and the others); kills
//! by the player count as people or creatures (where the game bumps those
//! isn't traced).
//!
//! New Vegas has 43, the same first 16 and then others ("Times Slept" at 23
//! where Fallout 3 has "Contracts Completed"); none of its extra ones is
//! bumped here.

use crate::scripting::GameState;

/// The statistics' names, by number (`00f4ca80`).
pub const NAMES: [&str; 26] = [
    "Quests Completed",
    "Locations Discovered",
    "People Killed",
    "Creatures Killed",
    "Locks Picked",
    "Computers Hacked",
    "Stimpaks Taken",
    "Rad-X Taken",
    "RadAway Taken",
    "Chems Taken",
    "Times Addicted",
    "Mines Disarmed",
    "Speech Successes",
    "Pockets Picked",
    "Pants Exploded",
    "Books Read",
    "Bobbleheads Found",
    "Weapons Created",
    "People Mezzed",
    "Captives Rescued",
    "Sandman Kills",
    "Paralyzing Punches",
    "Robots Disabled",
    "Contracts Completed",
    "Corpses Eaten",
    "Mysterious Stranger Visits",
];

pub const QUESTS_COMPLETED: u8 = 0;
pub const LOCATIONS_DISCOVERED: u8 = 1;
pub const PEOPLE_KILLED: u8 = 2;
pub const CREATURES_KILLED: u8 = 3;
pub const LOCKS_PICKED: u8 = 4;
pub const COMPUTERS_HACKED: u8 = 5;
pub const MINES_DISARMED: u8 = 11;
pub const POCKETS_PICKED: u8 = 13;
pub const BOOKS_READ: u8 = 15;

/// A statistic's number by its name (any case).
pub fn index(name: &str) -> Option<u8> {
    NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name.trim()))
        .map(|i| i as u8)
}

/// A statistic now. Quests completed are counted from the quests.
pub fn get(state: &GameState, stat: u8) -> u32 {
    if stat == QUESTS_COMPLETED {
        return state.completed.len() as u32 + state.misc_stats.get(&stat).copied().unwrap_or(0);
    }
    state.misc_stats.get(&stat).copied().unwrap_or(0)
}

/// Adds to a statistic (`ModPCMiscStat` takes negative amounts too; it
/// doesn't go below 0). Every change counts for the challenges about that
/// statistic (`004d5e10` → `005f5950(11, …)`), which look when scripts next
/// run (`world::more_functions::challenges::catch_up`).
pub fn bump(state: &mut GameState, stat: u8, by: i64) {
    let v = state.misc_stats.entry(stat).or_insert(0);
    *v = (i64::from(*v) + by).max(0) as u32;
    if by != 0 {
        state.more.challenges.stat_bumps.push((stat, by));
    }
}
