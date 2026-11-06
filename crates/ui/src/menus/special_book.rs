//! Fallout 3's "You're SPECIAL!" book's menu (`SPECIALBookMenu`, class
//! 0x424, `menus\chargen\specialbookmenu.xml`, Fallout3.exe 1.7.0.3): its
//! clicks and keys, queued for the viewer, which carries them out on the
//! book (`world::chargen::special_book`) and its model
//! (`cellview::special_book`). The file has no pictures: an invisible
//! button per action and a hot rectangle over the whole screen.
//!
//! * Clicks (`vf03`, `00676430`): 0 next page, 1 previous page, 2 plus,
//!   3 minus, 4 "all done", 5 anywhere (the model's buttons are tried),
//!   6 and 7 the index's focus up and down (`xup`, `xdown`).
//! * Keys (`vf0E`, `00676260`): Right (and 9) plus, Left (and 10) minus,
//!   each carried out only when it would change the value; the keys are
//!   used either way.
//! * The A key is "all done" (`00675470` polls the key at A's place each
//!   frame, Q's with a French `sLanguage`; here the letter typed, A on
//!   either keyboard).

use crate::menu::{special, MenuCode};
use crate::tile::{TileId, Ui};

pub const FILE: &str = "menus\\chargen\\specialbookmenu.xml";
pub const CLASS: i32 = 0x424;

/// The click ids (`00676430`).
pub mod click {
    pub const NEXT: i32 = 0;
    pub const PREVIOUS: i32 = 1;
    pub const INCREASE: i32 = 2;
    pub const DECREASE: i32 = 3;
    pub const DONE: i32 = 4;
    pub const PICK: i32 = 5;
    pub const FOCUS_UP: i32 = 6;
    pub const FOCUS_DOWN: i32 = 7;
}

/// What the player did, for the viewer to carry out in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// A click id (`vf03`).
    Click(i32),
    /// Right (9): plus, if it would change the value (`00676260`).
    Increase,
    /// Left (10): minus, likewise.
    Decrease,
}

#[derive(Debug, Clone)]
pub struct SpecialBookMenu {
    pub menu: TileId,
    pub closed: bool,
    pub inputs: Vec<Input>,
}

impl SpecialBookMenu {
    pub fn new(menu: TileId) -> SpecialBookMenu {
        SpecialBookMenu {
            menu,
            closed: false,
            inputs: Vec::new(),
        }
    }

    pub fn take_inputs(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.inputs)
    }
}

impl MenuCode for SpecialBookMenu {
    fn class(&self) -> i32 {
        CLASS
    }

    fn click(&mut self, _ui: &mut Ui, id: i32, _tile: Option<TileId>, _now: f64) {
        if !self.closed && (click::NEXT..=click::FOCUS_DOWN).contains(&id) {
            self.inputs.push(Input::Click(id));
        }
    }

    fn key(&mut self, _ui: &mut Ui, code: u32, _now: f64) -> bool {
        if self.closed || !matches!(char::from_u32(code), Some('a' | 'A')) {
            return false;
        }
        self.inputs.push(Input::Click(click::DONE));
        true
    }

    // Translated from 00676260 (decompiled, Fallout3.exe 1.7.0.3)
    fn special_key(&mut self, _ui: &mut Ui, code: i32, _now: f64) -> bool {
        if self.closed {
            return false;
        }
        match code {
            special::RIGHT | special::A => self.inputs.push(Input::Increase),
            special::LEFT | 10 => self.inputs.push(Input::Decrease),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_and_arrows_queue_in_order() {
        let mut ui = crate::menus::test_support::ui();
        let mut menu = SpecialBookMenu::new(0);
        menu.click(&mut ui, click::PICK, None, 0.0);
        assert!(menu.special_key(&mut ui, special::RIGHT, 0.0));
        assert!(menu.special_key(&mut ui, special::LEFT, 0.0));
        assert!(!menu.special_key(&mut ui, special::UP, 0.0));
        menu.click(&mut ui, 9, None, 0.0);
        assert!(menu.key(&mut ui, 'a' as u32, 0.0));
        assert!(!menu.key(&mut ui, 'b' as u32, 0.0));
        assert_eq!(
            menu.take_inputs(),
            vec![
                Input::Click(click::PICK),
                Input::Increase,
                Input::Decrease,
                Input::Click(click::DONE)
            ]
        );
        menu.closed = true;
        menu.click(&mut ui, click::NEXT, None, 0.0);
        assert!(menu.take_inputs().is_empty());
    }
}
