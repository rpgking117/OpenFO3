//! The "You're SPECIAL!" book's rules (`SPECIALBookMenu`, Fallout3.exe
//! 1.7.0.3): the nursery's `ShowSPECIALBookMenuParams 40`
//! (`CG01SpecialBookSCRIPT`) shares out the points the player's seven SPECIAL
//! attributes may add up to. The menu itself, a 3D book whose pages show each
//! value as number pictures, is drawn by `cellview::special_book` and shown
//! by the viewer (`viewer/src/game_menus/special_book.rs`).
//!
//! * Opening (`Cmd_ShowSPECIALBookMenu` `0052a2f0` → `006794c0`): the points
//!   given, or 35 without (`00679590`).
//! * The total (`00674df0`) is the seven attributes' sum; what's left is the
//!   points less it.
//! * Minus (`00676880`): the attribute one lower, never below 1; plus
//!   (`00676940`): one higher, never above 10, and only while points are
//!   left. Each sets the player's base value at once (`0076b7f0`) and plays
//!   `OBJBookSpecialNumber` when the value could move.
//! * The buttons show (`00675f10`, `00675f80`): minus from 2 up, plus below
//!   10 while points are left.
//! * The message picture (`00675de0`): one for more than one point left, one
//!   for one left, one for none.

use esm::LoadOrder;

use crate::chargen::SPECIAL;
use crate::dialogue::PLAYER_REF;
use crate::scripting::{Facts, GameState};

/// The points without a number (`00679590`: `006794c0(0x23)`).
pub const DEFAULT_POINTS: i32 = 35;
/// An attribute's least and most (`00676880`, `00676940`).
pub const LEAST: i32 = 1;
pub const MOST: i32 = 10;
/// The sound a value's change makes (`00676880`, `00676940`).
pub const NUMBER_SOUND: &str = "OBJBookSpecialNumber";

/// Which message picture shows (`00675de0`): +0x1b0, +0x1b4, +0x1b8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    MoreLeft,
    OneLeft,
    NoneLeft,
}

/// One attribute's base value (0 Strength .. 6 Luck), as the player has it.
pub fn value(order: &LoadOrder, state: &GameState, attribute: usize) -> i32 {
    let facts = Facts {
        order,
        state,
        speaker: None,
    };
    SPECIAL
        .get(attribute)
        .and_then(|&av| facts.base_actor_value(PLAYER_REF, av))
        .map_or(LEAST, |v| v.round() as i32)
}

/// The seven attributes' sum (`00674df0`).
pub fn total(order: &LoadOrder, state: &GameState) -> i32 {
    (0..SPECIAL.len()).map(|i| value(order, state, i)).sum()
}

/// The points left to share.
pub fn remaining(order: &LoadOrder, state: &GameState, points: i32) -> i32 {
    points - total(order, state)
}

fn set(state: &mut GameState, attribute: usize, to: i32) {
    if let Some(&av) = SPECIAL.get(attribute) {
        state.actor_values.insert((PLAYER_REF, av), f64::from(to));
    }
}

/// Minus (`00676880`): one lower, not below 1. Whether it changed.
// Translated from 00676880 (decompiled, Fallout3.exe 1.7.0.3)
pub fn decrease(order: &LoadOrder, state: &mut GameState, attribute: usize) -> bool {
    let now = value(order, state, attribute);
    let to = (now - 1).max(LEAST);
    set(state, attribute, to);
    to != now
}

/// Plus (`00676940`): one higher, not above 10, while points are left.
/// Whether it changed.
// Translated from 00676940 (decompiled, Fallout3.exe 1.7.0.3)
pub fn increase(order: &LoadOrder, state: &mut GameState, attribute: usize, points: i32) -> bool {
    let now = value(order, state, attribute);
    if total(order, state) >= points {
        return false;
    }
    let to = (now + 1).min(MOST);
    set(state, attribute, to);
    to != now
}

/// Whether minus shows (`00675f10`).
pub fn minus_shown(order: &LoadOrder, state: &GameState, attribute: usize) -> bool {
    value(order, state, attribute) >= 2
}

/// Whether plus shows (`00675f80`).
pub fn plus_shown(order: &LoadOrder, state: &GameState, attribute: usize, points: i32) -> bool {
    value(order, state, attribute) < MOST && total(order, state) < points
}

/// The message picture for the points left (`00675de0`).
pub fn message(left: i32) -> Message {
    if left > 1 {
        Message::MoreLeft
    } else if left == 1 {
        Message::OneLeft
    } else {
        Message::NoneLeft
    }
}

/// A number's two digits as the book shows the points left (`00675e50`):
/// none at 0, the ones alone below 10.
pub fn remaining_digits(left: i32) -> (Option<u8>, Option<u8>) {
    if left <= 0 {
        return (None, None);
    }
    let tens = (left / 10 >= 1).then_some((left / 10).min(9) as u8);
    (tens, Some((left % 10) as u8))
}

/// The page a turn plays its sequence for, by the page it reaches
/// (`00675a00`): forward to page n plays `FORWARD[n]` (the table at
/// `00f6b36c`, from page 1), back to page n plays `BACK[n]` (`00f6b34c`).
/// The two tables touch: `BACK[8]` is `FORWARD[0]`.
pub const FORWARD: [&str; 10] = [
    "TurnRight",
    "Forward",
    "Backward",
    "Left",
    "Right",
    "FastForward",
    "FastBackward",
    "FastLeft",
    "FastRight",
    "TurnLeft",
];
pub const BACK: [&str; 9] = [
    "Attack2",
    "Attack1",
    "AimUp",
    "Aim",
    "Holster",
    "JumpLand",
    "JumpLoop",
    "JumpStart",
    "TurnRight",
];
/// The pages: 0 the cover, 1–7 one attribute each, 8 the index of all
/// seven, 9 the back cover (`00675a00` stops past 9).
pub const INDEX_PAGE: u8 = 8;
pub const LAST_PAGE: u8 = 9;
/// The sound of a page turning (`00675a00`).
pub const PAGE_SOUND: &str = "OBJBookSpecialPageTurn";
/// The index's focus moving (`vf03` ids 6 and 7, the gamepad's).
pub const FOCUS_SOUND: &str = "OBJBookSpecialFocus";
/// "All done" with points left (`vf03` id 4, `00676360`).
pub const NOT_DONE_SOUND: &str = "UIActivateNothing";

/// The book's own state (`SPECIALBookMenu`: the page at `+0x48`, the
/// index's focus at `+0x4c`, the points at `+0x58`, whether a page may be
/// turned at `+0x54`; the way the last turn went is a static, `00f6b348`).
#[derive(Debug, Clone, PartialEq)]
pub struct Book {
    pub page: u8,
    pub forward: bool,
    pub focus: i32,
    pub points: i32,
    pub ready: bool,
}

impl Book {
    /// As the menu starts (`00630b00`: page 0, focus 1; `00f6b348` holds 1).
    pub fn new(points: i32) -> Book {
        Book {
            page: 0,
            forward: true,
            focus: 1,
            points,
            ready: true,
        }
    }

    /// Turns a page (`00675a00`): forward up to the back cover, back down
    /// to the cover. The sequence to play when it turned; the direction is
    /// kept even when it didn't.
    // Translated from 00675a00 (decompiled, Fallout3.exe 1.7.0.3)
    pub fn turn(&mut self, forward: bool) -> Option<&'static str> {
        self.forward = forward;
        if forward {
            if self.page >= LAST_PAGE {
                return None;
            }
            self.page += 1;
        } else {
            if self.page < 1 {
                return None;
            }
            self.page -= 1;
        }
        Some(self.sequence().unwrap_or(""))
    }

    /// The sequence the page shows (`00675470`): the last turn's, none on
    /// the cover after a forward turn (none has been made).
    pub fn sequence(&self) -> Option<&'static str> {
        let page = usize::from(self.page);
        if self.forward {
            (page >= 1).then(|| FORWARD.get(page).copied()).flatten()
        } else {
            BACK.get(page).copied()
        }
    }

    /// Whether a page may be turned (`00675470`): when the turn's sequence
    /// has played out, or on the cover.
    pub fn update_ready(&mut self, sequence_done: bool) {
        self.ready = sequence_done || self.page < 1;
    }

    /// The attribute (0 Strength .. 6 Luck) the page shows (`00675be0`):
    /// pages 1 to 7.
    ///
    /// **Not carried over:** on the index page, without a gamepad, the game
    /// takes actor value 12 (page + 4, Action Points), so the arrow keys
    /// there change the player's Action Points; here they do nothing.
    pub fn attribute(&self) -> Option<usize> {
        (1..=7).contains(&self.page).then(|| usize::from(self.page) - 1)
    }

    /// The index's focus up (`vf03` id 6) or down (7), between rows 1 and
    /// 7. Whether it moved (the focus sound plays).
    pub fn move_focus(&mut self, up: bool) -> bool {
        if self.page != INDEX_PAGE {
            return false;
        }
        if up && self.focus > 1 {
            self.focus -= 1;
            true
        } else if !up && self.focus < 7 {
            self.focus += 1;
            true
        } else {
            false
        }
    }
}

/// "All done" (`vf03` id 4, `00676360`): the book closes only once every
/// point is spent.
pub fn may_finish(order: &LoadOrder, state: &GameState, points: i32) -> bool {
    total(order, state) == points
}
