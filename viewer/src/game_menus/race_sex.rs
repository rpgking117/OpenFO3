//! The face menu (`ui::menus::race_sex`): a script's `ShowRaceMenu`
//! (`menus::Menu::Character(CharacterMenu::RaceSex)`, the birth scene's gene
//! projection). What it asks is carried out by `world::chargen::race_sex` on
//! the game state's `player_face`; the head on its screen follows from that
//! (`race_sex_preview`).
//!
//! Its two questions are message boxes with the buttons No and Yes
//! (`00619700`, both pass `sNo` then `sYes`). "Randomize your head?" goes ahead
//! on button 1 (`0066e530`), Yes. "Are you sure...?" (`0066c550`) reads the
//! other way in the instructions: button 1 stays and anything else closes,
//! which would make Yes refuse. **Unsettled:** the difference between the two
//! calls is only a flag at the box's `+0x1c`, not traced. Here Yes finishes,
//! as the words say; check it against the original game.

use std::sync::Arc;

use cellview::Game;
use esm::FormId;
use nif::ctl::{Ctl, Kind};
use ui::menus::message::MessageBox;
use ui::menus::race_sex::{Choice, RaceSexMenu, Request, Snapshot, FILE};
use world::chargen::{appearance, race_sex as rules, CharacterMenu};
use world::scripting::GameState;

use super::{message, OpenMenu, Screen};
use crate::menus::Menu;

/// The boxes' owners: their callbacks' addresses.
const CONFIRM_DONE: u32 = 0x0066_c550;
const CONFIRM_RANDOMIZE: u32 = 0x0066_e530;
/// The box's kind (`0x12`, both calls).
const CONFIRM_KIND: i32 = 0x12;
/// The button that goes ahead.
const YES: i32 = 1;

/// The menu and what it works with.
pub struct RaceSexScreen {
    pub menu: RaceSexMenu,
    controls: Arc<Ctl>,
}

/// Whether this module shows a request.
pub fn takes(menu: &Menu) -> bool {
    matches!(menu, Menu::Character(CharacterMenu::RaceSex { .. }))
}

fn female(state: &GameState) -> bool {
    state.player_female.unwrap_or(false)
}

/// The menu's picture of the game's side.
fn snapshot(game: &Game, state: &GameState, ctl: &Ctl) -> Snapshot {
    let order = &game.order;
    let female = female(state);
    let face = rules::PlayerFace::of_player(order, state.player_face.as_ref()).unwrap_or_default();
    let choices = |list: Vec<appearance::Choice>| -> Vec<Choice> {
        list.into_iter()
            .map(|c| Choice {
                form: c.form.0,
                name: c.name.unwrap_or_default(),
            })
            .collect()
    };
    let total = rules::total(order, &face, female);
    let ini = |key: &str, default: f32| game.settings.float("Interface", key).unwrap_or(default);
    Snapshot {
        female,
        race: face.race.0,
        races: choices(appearance::races(order)),
        hair: face.hair.map(|f| f.0),
        hairs: choices(appearance::hair(order, face.race, female)),
        eyes: face.eyes.map(|f| f.0),
        eye_colors: choices(appearance::eyes(order, face.race, female)),
        head_part: face.head_parts.first().map(|f| f.0),
        head_parts: choices(rules::head_parts(order)),
        hair_color: face.hair_color,
        presets: rules::presets(order, face.race, female).len() as i32,
        shape: rules::control_values(ctl, &total, Kind::Shape),
        tone: rules::control_values(ctl, &total, Kind::Texture),
        slider_min: ini("fRSMFaceSliderDefaultMin", -5.0),
        slider_max: ini("fRSMFaceSliderDefaultMax", 5.0),
    }
}

/// Opens the menu (`00670be0`).
pub fn open(screen: &mut Screen, game: &Game, state: &mut GameState, request: Menu) {
    let Menu::Character(CharacterMenu::RaceSex { mode }) = request else {
        return;
    };
    let controls = game
        .assets
        .read(rules::CONTROLS_FILE)
        .ok()
        .flatten()
        .and_then(|bytes| Ctl::parse(&bytes).ok());
    let Some(controls) = controls else {
        println!("The face menu can't work without {}.", rules::CONTROLS_FILE);
        return;
    };
    let mut menu = RaceSexMenu::new(0, mode);
    let tile = match screen.load(game, FILE, &mut menu) {
        Ok(tile) => tile,
        Err(e) => {
            println!("The face menu can't be shown: {e}");
            return;
        }
    };
    menu.menu = tile;
    // What the menu changes starts as the player is.
    if state.player_face.is_none() {
        state.player_face = rules::PlayerFace::of_npc(&game.order, world::dialogue::PLAYER_BASE);
    }
    let snap = snapshot(game, state, &controls);
    menu.set_starting_zoom(
        game.settings
            .float("Interface", "fRSMStartingZoom")
            .unwrap_or(0.7),
    );
    if !menu.open(&mut screen.ui, snap) {
        println!("MENUS: Race Sex Menu Creation Failed.");
        screen.ui.detach(tile);
        return;
    }
    println!("Face menu: mode {mode}.");
    screen.open.push(OpenMenu::RaceSex(Box::new(RaceSexScreen {
        menu,
        controls: Arc::new(controls),
    })));
}

/// The numbers Randomize draws, from the game's dice.
struct Dice<'a>(&'a mut GameState);

impl rules::Dice for Dice<'_> {
    /// `00be54b0`: the polar method.
    fn normal(&mut self) -> f32 {
        loop {
            let a = self.uniform() * 2.0 - 1.0;
            let b = self.uniform() * 2.0 - 1.0;
            let s = a * a + b * b;
            if s > 0.0 && s <= 1.0 {
                return b * (-2.0 * s.ln() / s).sqrt();
            }
        }
    }

    fn uniform(&mut self) -> f32 {
        (self.0.roll() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// `MatchFaceGeometry` (`0053ff90`): `who` moves toward `to`'s face.
pub fn match_face(game: &Game, state: &mut GameState, who: FormId, to: FormId, percent: f32) {
    let order = &game.order;
    let Some(ctl) = game
        .assets
        .read(rules::CONTROLS_FILE)
        .ok()
        .flatten()
        .and_then(|bytes| Ctl::parse(&bytes).ok())
    else {
        return;
    };
    let (Some((base, mut face)), Some((_, other))) =
        (rules::face_of(order, state, who), rules::face_of(order, state, to))
    else {
        return;
    };
    rules::match_face(order, &ctl, &mut face, &other, percent);
    state.npc_faces.insert(base, face);
}

/// A question asked over the menu.
fn ask(screen: &mut Screen, game: &Game, owner: u32, text: &str, no: &str, yes: &str) {
    let alpha = message::background_alpha(screen);
    let mut b = MessageBox::new(text, None, &[no, yes], CONFIRM_KIND, alpha);
    b.owner = Some(owner);
    message::show(screen, game, b);
}

/// What the menu asked for, carried out; every frame, with the time passed.
/// Returns sounds to play.
pub fn after(screen: &mut Screen, game: &Game, state: &mut GameState, dt: f32) -> Vec<FormId> {
    let order = &game.order;
    // The answers to its questions.
    let mut answer = |owner: u32| -> Option<i32> {
        screen.open.iter_mut().find_map(|m| match m {
            OpenMenu::Message(box_menu) => box_menu.take_pressed_for(Some(owner)),
            _ => None,
        })
    };
    let done = answer(CONFIRM_DONE);
    let randomize = answer(CONFIRM_RANDOMIZE);
    let mut sounds = Vec::new();
    let mut questions = Vec::new();
    let Screen { ui, open, .. } = screen;
    for m in open.iter_mut() {
        let OpenMenu::RaceSex(s) = m else {
            continue;
        };
        s.menu.frame(dt * 1000.0);
        if randomize == Some(YES) {
            s.menu.requests.push(Request::Randomize);
        }
        if done == Some(YES) {
            s.menu.finish(ui);
            // Everyone's packages looked at again once it's closed
            // (`GameState::evaluate_everyone`).
            state.evaluate_everyone = true;
        }
        for name in s.menu.sounds.drain(..) {
            if let Some(id) = order.form_by_editor_id(&name) {
                sounds.push(id);
            }
        }
        let requests = std::mem::take(&mut s.menu.requests);
        if requests.is_empty() {
            continue;
        }
        let sex = female(state);
        let ctl = s.controls.clone();
        let mut face = rules::PlayerFace::of_player(order, state.player_face.as_ref())
            .unwrap_or_default();
        let mut changed = false;
        for request in requests {
            changed = true;
            match request {
                Request::Sex { female } => {
                    state.player_female = Some(female);
                    rules::change_sex(order, &mut face, female);
                }
                Request::Race(race) => {
                    let sex = female(state);
                    rules::change_race(order, &mut face, FormId(race), sex);
                }
                Request::Preset(n) => {
                    let list = rules::presets(order, face.race, sex);
                    if let Some(&preset) = usize::try_from(n).ok().and_then(|n| list.get(n)) {
                        rules::apply_preset(order, &mut face, preset, sex);
                    }
                }
                Request::Randomize => {
                    rules::randomize(order, &ctl, &mut face, sex, &mut Dice(state));
                    s.menu.randomized();
                }
                Request::Hair(form) => face.hair = Some(FormId(form)),
                Request::HeadPart(form) => {
                    face.head_parts = form.map(FormId).into_iter().collect();
                }
                Request::Eyes(form) => face.eyes = Some(FormId(form)),
                Request::HairColor(color) => face.hair_color = color,
                Request::Control {
                    tone,
                    control,
                    value,
                } => {
                    let kind = if tone { Kind::Texture } else { Kind::Shape };
                    rules::set_control(
                        order,
                        &ctl,
                        &mut face,
                        sex,
                        kind,
                        control as usize,
                        value,
                    );
                }
                Request::ConfirmRandomize { text, no, yes } => {
                    changed = false;
                    questions.push((CONFIRM_RANDOMIZE, text, no, yes));
                }
                Request::ConfirmDone { text, no, yes } => {
                    changed = false;
                    questions.push((CONFIRM_DONE, text, no, yes));
                }
            }
        }
        if changed {
            state.player_face = Some(face);
            let snap = snapshot(game, state, &ctl);
            s.menu.sync(ui, snap);
        }
    }
    for (owner, text, no, yes) in questions {
        ask(screen, game, owner, &text, &no, &yes);
    }
    sounds
}
