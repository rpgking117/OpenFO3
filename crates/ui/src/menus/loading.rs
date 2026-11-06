//! The loading menu's slideshow (`LoadingMenu`, class 1007,
//! `menus\loading_menu.xml`), as the main menu shows it behind the start menu.
//! From `Fallout3.exe` 1.7.0.3.
//!
//! The menu's `loading_nif` tile holds `Interface\Loading\LoadingAnim01.NIF`:
//! a background, two slides (`Slide01`, `Slide02`) and the swipe, glare and
//! strips over them, with three sequences: `Left` (0.97 s), `Forward` (2.0 s)
//! and `Backward` (2.47 s). Code puts a loading screen's picture on a slide by
//! changing that slide's texture (`00658960`).
//!
//! * Opening (`006598d0`): the model scaled by 1.3333, `Slide01` given the
//!   first picture, `Left` played (`00de2f90`).
//! * Each frame (`00658f40`): while a sequence plays it moves on; when it has
//!   reached its end, the next picture in the list (`00657ea0`, wrapping) goes
//!   on the slide out of view (`00658960(0)`: `Slide02` when `+0x12a` bit 1 is
//!   clear, `Slide01` when it's set), and the next change is due after the
//!   interval (`006578d0`: `fMainMenuBkgdUpdateInterval` on the main menu).
//!   When it's due and nothing plays, bit 1 flips and `Forward` plays when
//!   it's set, `Backward` when it's clear: the slide out of view comes in.
//!
//! A sequence that has finished leaves the model as its end left it
//! (`0044db80` deactivates it; Gamebryo keeps the nodes where they are).
//!
//! At the first start the list is the welcome screens (`00659400`: `[Loading]
//! sWelcomeScreen1` .. `4` under `Interface\Loading\`, then
//! `sMainMenuBackground` under `Interface\Main\`). When it has come round to
//! its first again (`00658f40`: `+0x12a` bit 0), the start menu
//! (`StartMenu::vf0B`) has the list made again from the main menu's loading
//! screens ([`Slideshow::replace`]) and its title fades in.

use crate::menu::MenuCode;
use crate::names::t;
use crate::tile::{TileId, Ui};

/// The menu's file and class (`00659eb0`).
pub const FILE: &str = "menus\\loading_menu.xml";
pub const CLASS: i32 = 1007;
/// The slideshow's model (the `loading_nif` tile's file), under `meshes\`.
pub const MODEL: &str = "meshes\\interface\\loading\\loadinganim01.nif";
/// The model's scale (`006598d0`: `0x3faaa993`).
pub const MODEL_SCALE: f32 = 1.3333;
/// The model's centre in menu units (the file's `loading_nif`: x =
/// 640 × aspect / 4 × 3, the screen's middle; y = 480).
pub const MODEL_Y: f32 = 480.0;
/// The sequences (`00de2f90`, `00658f40`).
pub const OPENING: &str = "Left";
pub const FORWARD: &str = "Forward";
pub const BACKWARD: &str = "Backward";
/// The slides' nodes, whose textures change.
pub const SLIDES: [&str; 2] = ["Slide01", "Slide02"];
/// `[Loading] fMainMenuBkgdUpdateInterval`'s default.
pub const MAIN_MENU_INTERVAL: f64 = 10.0;

/// The slideshow's state.
#[derive(Debug, Clone, PartialEq)]
pub struct Slideshow {
    pictures: Vec<String>,
    /// The picture the list is at (`+0xf0`).
    current: usize,
    /// `+0x12a` bit 0: the list has come round to its first picture again.
    wrapped: bool,
    /// A sequence started since last asked, for its sounds.
    started: Option<&'static str>,
    /// `+0x12a` bit 1: the next change plays `Forward`.
    flip: bool,
    /// The sequence playing and when it started (`+0xf8`).
    playing: Option<(&'static str, f64)>,
    /// The last sequence played, which holds the model at its end.
    last: Option<&'static str>,
    /// When the next change is due (`+0xf8`).
    next: f64,
    interval: f64,
    /// Each slide's picture.
    slides: [Option<String>; 2],
    /// The picture in view once a sequence has ended: the in-game menu's
    /// plate and words are its (`00658f40` calls `00658840` before moving the
    /// list on).
    in_view: Option<usize>,
}

impl Slideshow {
    /// `006598d0`: the first picture on `Slide01` and `Left` playing.
    pub fn new(pictures: Vec<String>, interval: f64, now: f64) -> Slideshow {
        let first = pictures.first().cloned();
        Slideshow {
            pictures,
            current: 0,
            wrapped: false,
            started: Some(OPENING),
            flip: false,
            playing: Some((OPENING, now)),
            last: None,
            next: now,
            interval,
            slides: [first, None],
            in_view: None,
        }
    }

    /// `00658f40`: one frame. `length` gives a sequence's length in seconds.
    // Translated from 00658f40 (decompiled, Fallout3.exe 1.7.0.3)
    pub fn update(&mut self, now: f64, length: impl Fn(&str) -> f32) {
        match self.playing {
            Some((name, start)) => {
                if now - start >= f64::from(length(name)) {
                    if !self.pictures.is_empty() {
                        self.in_view = Some(self.current);
                        self.current = (self.current + 1) % self.pictures.len();
                        if self.current == 0 {
                            self.wrapped = true;
                        }
                        let out_of_view = if self.flip { 0 } else { 1 };
                        self.slides[out_of_view] = Some(self.pictures[self.current].clone());
                    }
                    self.last = Some(name);
                    self.playing = None;
                    self.next = now + self.interval;
                }
            }
            None if now >= self.next => {
                self.flip = !self.flip;
                let name = if self.flip { FORWARD } else { BACKWARD };
                self.playing = Some((name, now));
                self.started = Some(name);
            }
            None => {}
        }
    }

    /// The sequence to pose the model with and its time: the one playing, else
    /// the last one at its end (`None` for its time: the end).
    pub fn pose(&self, now: f64) -> Option<(&'static str, Option<f32>)> {
        match self.playing {
            Some((name, start)) => Some((name, Some((now - start).max(0.0) as f32))),
            None => self.last.map(|name| (name, None)),
        }
    }

    /// The picture on a slide (0 `Slide01`, 1 `Slide02`), if one's been put
    /// there.
    pub fn picture(&self, slide: usize) -> Option<&str> {
        self.slides.get(slide).and_then(|p| p.as_deref())
    }

    /// The list has come round to its first picture (`+0x12a` bit 0).
    pub fn wrapped(&self) -> bool {
        self.wrapped
    }

    /// A key or click during the first start's pictures (`00658af0`, from the
    /// start menu's `vf0E` and frame while it waits): unless the list has
    /// come round or a sequence is playing, the list jumps to its last
    /// picture (the main menu's background), which goes on the slide out of
    /// view, and the change is due now. Whether it did.
    // Translated from 00658af0 (decompiled, Fallout3.exe 1.7.0.3)
    pub fn skip_to_last(&mut self, now: f64) -> bool {
        if self.wrapped || self.playing.is_some() || self.pictures.is_empty() {
            return false;
        }
        self.current = self.pictures.len() - 1;
        let out_of_view = if self.flip { 0 } else { 1 };
        self.slides[out_of_view] = Some(self.pictures[self.current].clone());
        self.next = now;
        true
    }

    /// The sequence started since last asked (its text keys' sounds play).
    pub fn take_started(&mut self) -> Option<&'static str> {
        self.started.take()
    }

    /// `00659400` with the start menu up: a new list. Unless a sequence is
    /// playing, its first picture goes on the slide out of view and the next
    /// change is due after the interval (`00658960(0)`, `006578d0`).
    // Translated from 00659400 (decompiled, Fallout3.exe 1.7.0.3)
    pub fn replace(&mut self, pictures: Vec<String>, now: f64) {
        self.pictures = pictures;
        self.current = 0;
        self.wrapped = false;
        if self.playing.is_none() {
            if let Some(first) = self.pictures.first() {
                let out_of_view = if self.flip { 0 } else { 1 };
                self.slides[out_of_view] = Some(first.clone());
            }
            self.next = now + self.interval;
        }
    }

    pub fn pictures(&self) -> &[String] {
        &self.pictures
    }

    /// The picture the list is at (`+0xf0`).
    pub fn current(&self) -> usize {
        self.current
    }

    /// The picture that came into view as the last sequence ended (none
    /// before the first has): the one whose words show.
    pub fn in_view(&self) -> Option<usize> {
        self.in_view
    }
}

/// The in-game loading menu's text tiles, by their `id` in the file (the
/// constructor `00658590` keeps them by it).
pub mod ids {
    pub const PLATE_NUMBER: i32 = 1;
    pub const INFO: i32 = 2;
    pub const OBJECTIVE: i32 = 3;
    /// Each statistic's name and number.
    pub const STATS: [(i32, i32); 3] = [(4, 5), (6, 7), (8, 9)];
    pub const LEVEL_METER: i32 = 10;
    pub const PLATE_LABEL: i32 = 13;
    pub const OBJECTIVE_LABEL: i32 = 14;
    pub const LEVEL_LABEL: i32 = 15;
}

/// The objective shown without one (`00f62d48`).
pub const NO_OBJECTIVE: &str = "None";

/// How many statistics the menu goes round (`00658bf0`: 0x1a, the
/// Pip-Boy's).
pub const STAT_COUNT: usize = 26;

/// `[Loading] fLoadingTextUpdateInterval`'s and `fLoadingBkgdUpdateInterval`'s
/// value in this install's `Fallout.ini`: the statistics and the pictures
/// change this often.
pub const IN_GAME_INTERVAL: f64 = 10.0;

/// The loading menu while a place loads in the game: its tiles by id and the
/// statistics' order.
#[derive(Debug, Clone)]
pub struct InGameMenu {
    pub menu: TileId,
    tiles: [Option<TileId>; 16],
    /// The statistics in a random order (`+0x68`), and the next to show
    /// (`+0x128`).
    order: [usize; STAT_COUNT],
    next: usize,
}

impl MenuCode for InGameMenu {
    fn class(&self) -> i32 {
        CLASS
    }

    fn set_tile(&mut self, id: i32, tile: TileId) {
        if let Some(slot) = usize::try_from(id).ok().and_then(|i| self.tiles.get_mut(i)) {
            *slot = Some(tile);
        }
    }

    fn click(&mut self, _ui: &mut Ui, _id: i32, _tile: Option<TileId>, _now: f64) {}
}

impl InGameMenu {
    /// `00658bf0`'s first step: the statistics in a random order, each put
    /// in a slot `roll(26)` picks until it finds an empty one.
    pub fn new(mut roll: impl FnMut(usize) -> usize) -> InGameMenu {
        let mut order = [STAT_COUNT; STAT_COUNT];
        for i in 0..STAT_COUNT {
            loop {
                let k = roll(STAT_COUNT).min(STAT_COUNT - 1);
                if order[k] == STAT_COUNT {
                    order[k] = i;
                    break;
                }
            }
        }
        InGameMenu {
            menu: 0,
            tiles: [None; 16],
            order,
            next: 0,
        }
    }

    fn text(&self, ui: &mut Ui, id: i32, text: &str) {
        if let Some(&Some(tile)) = usize::try_from(id).ok().and_then(|i| self.tiles.get(i)) {
            ui.set_string(tile, t::STRING, text);
            ui.set_number(tile, t::VISIBLE, 1.0);
        }
    }

    /// `00658840`: the picture showing's plate ("VDSG Plate", "No. 123-45"
    /// from its file's name, [`plate_number`]) and its words (`DESC`).
    pub fn show_screen(&self, ui: &mut Ui, icon: &str, words: &str) {
        self.text(ui, ids::PLATE_LABEL, "VDSG Plate");
        self.text(ui, ids::PLATE_NUMBER, &plate_number(icon));
        self.text(ui, ids::INFO, words);
    }

    /// `00658bf0`'s rest: the current objective (`007705b0`: the active
    /// quest's first one shown and not done; "None", `00f62d48`, without
    /// one), the labels, the level and the way to the next (`00657c10`) on
    /// the meter's `user0` and `user1`.
    pub fn show_progress(&self, ui: &mut Ui, objective: Option<&str>, level: u32, progress: f32) {
        self.text(ui, ids::OBJECTIVE, objective.unwrap_or(NO_OBJECTIVE));
        self.text(ui, ids::OBJECTIVE_LABEL, "CURRENT OBJECTIVE");
        self.text(ui, ids::LEVEL_LABEL, "Level Progress");
        if let Some(&Some(meter)) = self.tiles.get(ids::LEVEL_METER as usize) {
            ui.set_number(meter, t::USER0, progress);
            ui.set_number(meter, t::USER0 + 1, level as f32);
        }
    }

    /// `00658760` for each of the three slots: the next statistic in the
    /// order, its name and its number (`%d`), going round.
    pub fn next_stats(&mut self, ui: &mut Ui, names: &[&str], values: &[u32]) {
        for (name_id, value_id) in ids::STATS {
            let stat = self.order[self.next];
            let name = names.get(stat).copied().unwrap_or("");
            let value = values.get(stat).copied().unwrap_or(0);
            self.text(ui, name_id, name);
            self.text(ui, value_id, &value.to_string());
            self.next = (self.next + 1) % STAT_COUNT;
        }
    }
}

/// A loading screen's plate number (`00658840`): its picture's file name as
/// its record has it (`Interface\Loading\...`), lower case, hashed (each
/// letter plus 33 times the sum so far) and taken modulo 100000, shown as
/// "No." and the hundreds, a dash and the rest: "No. 123-45".
pub fn plate_number(icon: &str) -> String {
    let mut h: u32 = 0;
    for b in icon.bytes() {
        h = u32::from(b.to_ascii_lowercase()).wrapping_add(h.wrapping_mul(0x21));
    }
    let n = h % 100_000;
    format!("No. {:03}-{:02}", n / 100, n % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_follow_the_picture_in_view() {
        let mut show = Slideshow::new(vec!["a".into(), "b".into()], 10.0, 0.0);
        assert_eq!(show.in_view(), None);
        // The opening sequence ends: "a" is in view, the list moves to "b".
        show.update(5.0, |_| 1.0);
        assert_eq!((show.in_view(), show.current()), (Some(0), 1));
        // "b" comes in; when that ends it's the one in view.
        show.update(15.0, |_| 1.0);
        show.update(17.0, |_| 1.0);
        assert_eq!((show.in_view(), show.current()), (Some(1), 0));
    }

    #[test]
    fn plates_are_numbered_from_the_pictures_name() {
        // h = c + 33h over the lower-case name, modulo 100000.
        let mut h: u32 = 0;
        for b in "interface\\loading\\loading_screen01.dds".bytes() {
            h = u32::from(b).wrapping_add(h.wrapping_mul(33));
        }
        let n = h % 100_000;
        let want = format!("No. {:03}-{:02}", n / 100, n % 100);
        assert_eq!(plate_number("Interface\\Loading\\loading_screen01.dds"), want);
        assert!(plate_number("A").starts_with("No. 000-"));
    }

    #[test]
    fn without_an_objective_the_menu_says_none() {
        let xml = "<menu name=\"LoadingMenu\"><class>&LoadingMenu;</class>
             <text name=\"Objective\"><id>3</id><font>1</font></text>
             <text name=\"ObjectiveLabel\"><id>14</id><font>1</font></text>
           </menu>";
        let mut ui = crate::menus::test_support::ui();
        let mut i = 0usize;
        let mut menu = InGameMenu::new(|n| {
            i += 1;
            i % n
        });
        menu.menu = crate::menus::test_support::load(&mut ui, xml, &mut menu);
        menu.show_progress(&mut ui, None, 1, 0.0);
        let shown = |ui: &mut Ui, id: i32| {
            let tile = menu.tiles[id as usize].unwrap();
            ui.string(tile, t::STRING)
        };
        assert_eq!(shown(&mut ui, ids::OBJECTIVE).as_deref(), Some("None"));
        assert_eq!(
            shown(&mut ui, ids::OBJECTIVE_LABEL).as_deref(),
            Some("CURRENT OBJECTIVE")
        );
        menu.show_progress(&mut ui, Some("Talk to Dad"), 1, 0.0);
        assert_eq!(shown(&mut ui, ids::OBJECTIVE).as_deref(), Some("Talk to Dad"));
    }

    #[test]
    fn the_statistics_go_round_in_a_random_order() {
        // 5, 5, then 0, 1, 2 … round and round.
        let mut i = 0usize;
        let mut menu = InGameMenu::new(|n| {
            i += 1;
            if i <= 2 {
                5
            } else {
                (i - 3) % n
            }
        });
        let mut seen = menu.order.to_vec();
        seen.sort();
        assert_eq!(seen, (0..STAT_COUNT).collect::<Vec<_>>());
        // The second roll of 5 found its slot taken and rolled again.
        assert_eq!(menu.order[5], 0);
        assert_eq!(menu.order[0], 1);
        menu.next = STAT_COUNT - 1;
        let mut ui = crate::menus::test_support::ui();
        menu.next_stats(&mut ui, &[], &[]);
        assert_eq!(menu.next, 2);
    }

    fn length(name: &str) -> f32 {
        match name {
            OPENING => 0.97,
            FORWARD => 2.0,
            _ => 2.47,
        }
    }

    fn show(n: usize) -> Slideshow {
        Slideshow::new(
            (0..n).map(|i| format!("p{i}")).collect(),
            MAIN_MENU_INTERVAL,
            0.0,
        )
    }

    /// The slides take turns: the picture after the one in view goes on the
    /// slide out of view as each sequence ends, and the next sequence brings it
    /// in after the interval.
    #[test]
    fn pictures_go_on_the_slide_out_of_view_and_come_in_in_turn() {
        let mut s = show(3);
        assert_eq!(s.picture(0), Some("p0"));
        assert_eq!(s.picture(1), None);
        assert_eq!(s.pose(0.5), Some((OPENING, Some(0.5))));
        s.update(1.0, length);
        assert_eq!(s.picture(1), Some("p1"));
        assert_eq!(s.pose(5.0), Some((OPENING, None)));
        s.update(10.0, length);
        assert_eq!(s.pose(10.0), Some((OPENING, None)));
        s.update(11.0, length);
        assert_eq!(s.pose(11.5), Some((FORWARD, Some(0.5))));
        s.update(13.0, length);
        assert_eq!(s.picture(0), Some("p2"));
        assert_eq!(s.picture(1), Some("p1"));
        s.update(23.0, length);
        assert_eq!(s.pose(23.0).unwrap().0, BACKWARD);
        assert!(!s.wrapped());
        s.update(25.5, length);
        // Back round to the first.
        assert_eq!(s.picture(1), Some("p0"));
        assert_eq!(s.picture(0), Some("p2"));
        assert!(s.wrapped());
    }

    /// The welcome screens give way to the main menu's: the first of the new
    /// list goes on the slide out of view and comes in after the interval.
    #[test]
    fn a_new_list_comes_in_on_the_slide_out_of_view() {
        let mut s = show(2);
        assert_eq!(s.take_started(), Some(OPENING));
        s.update(1.0, length);
        s.update(11.0, length);
        assert_eq!(s.take_started(), Some(FORWARD));
        s.update(13.0, length);
        assert!(s.wrapped());
        s.replace(vec!["menu0".into(), "menu1".into()], 13.5);
        assert_eq!(s.picture(0), Some("menu0"));
        assert_eq!(s.picture(1), Some("p1"));
        s.update(20.0, length);
        assert_eq!(s.take_started(), None);
        s.update(23.5, length);
        assert_eq!(s.take_started(), Some(BACKWARD));
        assert!(!s.wrapped());
    }

    /// A key during the welcome pictures jumps to the last one, which comes in
    /// at once; then the list comes round. Not while a sequence plays.
    #[test]
    fn a_key_jumps_to_the_last_picture() {
        let mut s = show(5);
        assert!(!s.skip_to_last(0.5));
        s.update(1.0, length);
        assert!(s.skip_to_last(3.0));
        assert_eq!(s.picture(1), Some("p4"));
        s.update(3.0, length);
        assert_eq!(s.pose(3.0).map(|p| p.0), Some(FORWARD));
        s.update(5.0, length);
        assert!(s.wrapped());
        assert!(!s.skip_to_last(6.0));
    }

    /// Nothing to show: the sequences still play over the model's own slides.
    #[test]
    fn no_pictures_still_plays() {
        let mut s = show(0);
        s.update(1.0, length);
        s.update(11.0, length);
        assert_eq!(s.pose(11.0).unwrap().0, FORWARD);
        assert_eq!(s.picture(0), None);
    }
}
