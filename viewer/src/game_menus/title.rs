//! The game's main menu, as Fallout3.exe 1.7.0.3 shows it: the start menu
//! as the title (`ui::menus::start`, not the pause menu) over the loading
//! menu's slideshow (`ui::menus::loading`), with the title music.
//!
//! * It opens at the start (`--main-menu`, or the Data folder alone) and
//!   after the pause menu's Main Menu (`00679960`). The new game's place is
//!   loaded behind it and waits: virtual time stops, the scripts don't run
//!   (`scripts::run_scripts`), the player isn't ready, and a black picture
//!   covers it.
//! * The slideshow: the loading screens flagged for the main menu
//!   (`world::loading_screens`), picked at random up to
//!   `iMaxScreens_MainMenu`, on `LoadingAnim01.NIF`'s slides, changing every
//!   `fMainMenuBkgdUpdateInterval` seconds with the model's own sequences.
//! * The music: `Data\Music\Special\` + `sTitleMusic` + `.mp3` on a menu deck
//!   (`006598d0`), the music manager held meanwhile.
//! * New (`00680d10`) asks for a new game ([`NewGameRequest`],
//!   `scripts::save_and_load`), and the menu closes. Continue and Load load a
//!   save through the save system, which closes the menu once it has worked
//!   ([`TitleScreen::leave`]). Quit's Exit Game ends the program.
//!
//! * Crew (`0067af30`): the title and list fade out, then the credits roll
//!   (`ui::menus::credits`, `Data\Credits.txt`) rises over the slideshow (the
//!   credits menu's own dark background is hidden when it comes from the main
//!   menu, `0063ce90`); a click, a key or its end closes it and the title fades
//!   back in.
//!
//! Not done: Downloads (the extra content page). Labelled guesses: the model
//! drawn flat with x across and y up, farthest (largest z) first, as the pause
//! background is (`start::background_draws`).

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::time::Real;
use bevy::window::PrimaryWindow;
use cellview::Game;
use ui::draw::{DrawItem, DrawKind};
use ui::menus::loading::{self, InGameMenu, Slideshow};
use ui::names::t;
use ui::TileId;

use super::{start, GameMenus, OpenMenu};
use crate::GameFiles;

/// A load wants the loading screen, and where it goes (`None`: nowhere
/// given, as after New). `title_frame` opens it.
#[derive(Resource, Default)]
pub struct LoadingWanted(pub Option<Option<world::loading_screens::Destination>>);

/// The main menu's state.
#[derive(Resource, Default)]
pub struct TitleScreen {
    /// Open it once there's a screen and the place behind is up.
    pub wanted: bool,
    /// It's up: the world behind waits.
    pub up: bool,
    /// A save loaded from it: it closes.
    pub leave: bool,
    show: Option<Slideshow>,
    model: Option<Option<LoadingModel>>,
    /// The music started (the manager may not be up yet).
    music: bool,
    /// Show the welcome screens first (the program's first main menu).
    welcome: bool,
    /// They're showing: the title waits.
    welcoming: bool,
    /// The sequence and time the sounds were last played up to.
    sound_at: Option<(&'static str, f32)>,
    /// The credits rolling: what's in them, when they started, how long the
    /// roll is.
    credits: Option<Roll>,
    /// New was chosen at this time: the slideshow stays up as the loading
    /// screen while the new game loads, its first stage runs and the intro
    /// movie plays (the game's loading menu, opened by `00680d10`).
    pub loading: Option<f64>,
    /// When the new game was ready (it goes a moment after).
    loaded_at: Option<f64>,
    /// A movie has the screen: the loading screen isn't drawn over it.
    movie_up: bool,
    /// The loading menu as the game shows it in play (`LoadingMenu` with no
    /// start menu): its texts, the loading screens behind its pictures, the
    /// one whose words show and when the statistics last changed.
    in_game: Option<InGameMenu>,
    /// The menus' build `in_game` was made in (`Screen::built`).
    in_game_built: u32,
    screens: Vec<world::loading_screens::LoadScreen>,
    shown: Option<usize>,
    stats_at: f64,
    /// New asked for the loading screen (`want_loading`).
    asked: Option<Option<world::loading_screens::Destination>>,
    /// Frames in a row the place has been ready.
    ready_frames: u32,
}

/// The loading screen stays a second and this many frames after the place is
/// ready (a stand-in: when the game takes its menu down isn't traced), so the
/// first frames of the place are drawn behind it.
const READY_FRAMES: u32 = 10;

/// The credits on screen.
struct Roll {
    entries: Vec<ui::menus::credits::Entry>,
    started: f64,
    length: f32,
    speed: f32,
}

impl TitleScreen {
    pub fn at_start(wanted: bool) -> TitleScreen {
        TitleScreen {
            wanted,
            welcome: wanted,
            ..default()
        }
    }

    /// The loading screen is up in play (not the main menu's).
    pub fn loading_shown(&self) -> bool {
        self.loading.is_some() && !self.up
    }

    /// A load starts: the loading screen goes up for it.
    pub fn want_loading(&mut self, to: Option<world::loading_screens::Destination>) {
        self.asked = Some(to);
    }

    /// The world behind should wait.
    pub fn holds_world(&self) -> bool {
        self.up || self.wanted
    }

    /// Crew (`0063ce90`): the roll read from `Data\Credits.txt` with the
    /// menus' fonts, starting now.
    pub fn start_credits(&mut self, game: &Game, ui: &ui::Ui, now: f64) {
        use ui::menus::credits;
        let path = game.assets.data_dir().join("Credits.txt");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("Crew: {} can't be read: {e}", path.display());
                return;
            }
        };
        let (blank, height) = credits::heights(&ui.fonts);
        let entries = credits::parse(&bytes, blank, &height);
        let length = credits::length(&entries, &height);
        let speed = game
            .settings
            .float("Menu", "fCreditsScrollSpeed")
            .unwrap_or(credits::SCROLL_SPEED)
            * credits::MEASURED_SPEED_FACTOR;
        println!(
            "Crew: {} lines and pictures, {length:.0} units long, rising {speed} a second.",
            entries.len()
        );
        self.credits = Some(Roll {
            entries,
            started: now,
            length,
            speed,
        });
    }
}

/// The main menu's New answered Yes: a new game for the save system to start
/// (`scripts::save_and_load`).
#[derive(Resource, Default)]
pub struct NewGameRequest(pub bool);

/// A number below `n` from the clock (the game's own random numbers,
/// `0086c4b0`, aren't followed).
fn roller() -> impl FnMut(usize) -> usize {
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64)
        | 1;
    move |n| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as usize) % n.max(1)
    }
}

/// The welcome screens (`00659400` at the first start): `[Loading]
/// sWelcomeScreen1` .. `4` under `Interface\Loading\`, then
/// `sMainMenuBackground` under `Interface\Main\`, as pictures under
/// `textures\` (`00658960`).
fn welcome_pictures(game: &Game) -> Vec<String> {
    let get = |key: &str, default: &str| {
        game.settings
            .get("Loading", key)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(default)
            .to_ascii_lowercase()
    };
    let mut out: Vec<String> = [
        ("sWelcomeScreen1", "loading_screen01"),
        ("sWelcomeScreen2", "loading_screen_bethsoft"),
        ("sWelcomeScreen3", "loading_screen_bgs"),
        ("sWelcomeScreen4", "loading_screen_legal"),
    ]
    .iter()
    .map(|(k, d)| format!("textures\\interface\\loading\\{}.dds", get(k, d)))
    .collect();
    out.push(format!(
        "textures\\interface\\main\\{}.dds",
        get("sMainMenuBackground", "main_background")
    ));
    out
}

/// The main menu's own pictures: the loading screens flagged for it, picked
/// at random up to `iMaxScreens_MainMenu`.
fn menu_pictures(game: &Game) -> Vec<String> {
    let most = game
        .settings
        .float("Loading", "iMaxScreens_MainMenu")
        .map_or(world::loading_screens::MAIN_MENU_MOST, |n| n.max(0.0) as usize);
    world::loading_screens::pick(world::loading_screens::main_menu(&game.order), most, roller())
        .into_iter()
        .map(|s| s.picture)
        .collect()
}

/// Opens the main menu when it's wanted (`00681540` with no pause).
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_title(
    game: Res<GameFiles>,
    mut title: ResMut<TitleScreen>,
    mut menus: ResMut<GameMenus>,
    mut queue: ResMut<crate::menus::Menus>,
    mut settings: ResMut<start::GameSettings>,
    mut virt: ResMut<Time<Virtual>>,
    mut player: ResMut<crate::walk::Player>,
    windows: Query<&Window, With<PrimaryWindow>>,
    real: Res<Time<Real>>,
) {
    if !title.wanted || title.up {
        return;
    }
    if !player.ready {
        return;
    }
    let Some(size) = super::window_size(&windows) else {
        return;
    };
    let Some(screen) = super::screen(&mut menus, &game.0, size) else {
        return;
    };
    if !screen.open.is_empty() {
        return;
    }
    title.wanted = false;
    if !start::open(screen, &game.0, &mut settings, false, false, false) {
        return;
    }
    let interval = game
        .0
        .settings
        .float("Loading", "fMainMenuBkgdUpdateInterval")
        .map_or(loading::MAIN_MENU_INTERVAL, f64::from);
    // The first time, the welcome screens first and the title waits for them
    // (`00681540`: `0x80000`).
    let welcome = std::mem::take(&mut title.welcome);
    let pictures = if welcome {
        if let Some(OpenMenu::Start(s)) = screen.open.last_mut() {
            s.menu.hold_title();
        }
        welcome_pictures(&game.0)
    } else {
        menu_pictures(&game.0)
    };
    println!(
        "Main menu: {} {} for the slideshow, changing every {interval} s.",
        pictures.len(),
        if welcome { "welcome screens" } else { "loading screens" }
    );
    title.show = Some(Slideshow::new(pictures, interval, real.elapsed_secs_f64()));
    title.welcoming = welcome;
    title.sound_at = None;
    title.up = true;
    title.leave = false;
    title.music = false;
    virt.pause();
    queue.game_open = true;
    player.ready = false;
}

/// Each frame while it's up: the music once the manager has started, the
/// slideshow, and leaving when the menu has gone or a save has loaded.
#[allow(clippy::too_many_arguments)]
pub(crate) fn title_frame(
    game: Res<GameFiles>,
    mut title: ResMut<TitleScreen>,
    mut menus: ResMut<GameMenus>,
    mut music: ResMut<crate::music::Music>,
    mut virt: ResMut<Time<Virtual>>,
    real: Res<Time<Real>>,
    mut sounds: ResMut<crate::sounds::SoundRequests>,
    (keys, mouse): (Res<ButtonInput<KeyCode>>, Res<ButtonInput<MouseButton>>),
    (start_stage, movies, pending): (
        Res<crate::scripts::StartStage>,
        Res<crate::movie::Movies>,
        Res<crate::PendingScene>,
    ),
    (mut wanted, state, deferred, pending_exterior, player, save_wait, outdoors): (
        ResMut<LoadingWanted>,
        Res<crate::dialogue::DialogueState>,
        Res<crate::DeferredLoad>,
        Res<crate::exterior::PendingExterior>,
        Res<crate::walk::Player>,
        Res<crate::scripts::SaveWait>,
        Option<Res<crate::exterior::Exterior>>,
    ),
) {
    let now = real.elapsed_secs_f64();
    // The loading screen after New: the slideshow goes on until the new game
    // is placed (its first stage set, its place spawned) and no movie is
    // playing, then a moment more. (The game's loading menu picks the
    // destination's own loading screens and shows their text; here the main
    // menu's go on.)
    if let Some(to) = wanted.0.take().or_else(|| title.asked.take()) {
        if !title.up {
            begin_in_game(&mut title, &game.0, &state.0, to, now);
        }
    }
    if title.loading.is_some() && !title.up {
        title.movie_up = movies.busy();
        let TitleScreen { show, model, .. } = &mut *title;
        if let (Some(show), Some(Some(model))) = (show.as_mut(), model.as_ref()) {
            show.update(now, |name| model.length(name));
        }
        in_game_texts(&mut title, &mut menus, &game.0, &state.0, now);
        let ready = start_stage.0.is_none()
            && pending.0.is_none()
            && deferred.0.is_none()
            && save_wait.0.is_none()
            && pending_exterior.0.is_none()
            && outdoors.as_deref().is_none_or(|e| e.settled())
            && player.ready
            && !title.movie_up;
        if ready {
            title.ready_frames = title.ready_frames.saturating_add(1);
        } else {
            title.ready_frames = 0;
        }
        match (ready, title.loaded_at) {
            (true, None) => title.loaded_at = Some(now),
            (true, Some(at)) if now - at > 1.0 && title.ready_frames > READY_FRAMES => {
                title.loading = None;
                title.loaded_at = None;
                title.show = None;
                if let Some(m) = title.in_game.take() {
                    if let Some(screen) = menus.screen.as_deref_mut() {
                        if screen.built == title.in_game_built {
                            screen.ui.detach(m.menu);
                        }
                    }
                }
                title.screens.clear();
                title.shown = None;
                println!("Loading screen closed ({now:.2} s real).");
            }
            (false, _) => title.loaded_at = None,
            _ => {}
        }
    }
    if !title.up {
        return;
    }
    // The credits: past their end, or a click or key (`CreditsMenu::vf0B`,
    // `vf0E`), they close and the title comes back.
    if let Some(roll) = &title.credits {
        let gone = ui::menus::credits::scroll((now - roll.started) as f32, roll.speed, roll.length)
            .is_none();
        let pressed = keys.get_just_pressed().next().is_some()
            || mouse.get_just_pressed().next().is_some();
        if gone || pressed {
            title.credits = None;
            println!("Crew closed.");
            if let Some(screen) = menus.screen.as_deref_mut() {
                for m in &mut screen.open {
                    if let OpenMenu::Start(s) = m {
                        if s.menu.is_title() {
                            s.menu.credits_closed(&mut screen.ui);
                        }
                    }
                }
            }
        }
    }
    if !title.music {
        let track = game
            .0
            .settings
            .get("Loading", "sTitleMusic")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("MainTitle");
        let path = format!("music\\special\\{}.mp3", track.to_ascii_lowercase());
        title.music = crate::music::menu_song(
            &mut music,
            &game.0,
            &path,
            crate::music::audio_clock(&real),
        );
        if title.music {
            println!("Main menu music: {path}.");
        }
    }
    let TitleScreen {
        show,
        model,
        sound_at,
        ..
    } = &mut *title;
    let model = model.get_or_insert_with(|| LoadingModel::read(&game.0));
    if let (Some(show), Some(model)) = (show.as_mut(), model.as_ref()) {
        show.update(now, |name| model.length(name));
        // The sequences' `sound:` text keys (`UILoadScreenInitial`,
        // `UILoadScreenMicroFiche01`) as their times pass.
        if show.take_started().is_some() {
            *sound_at = None;
        }
        if let Some((name, Some(t))) = show.pose(now) {
            let from = match *sound_at {
                Some((n, at)) if n == name => at,
                _ => -1.0,
            };
            for sound in model.sounds_between(name, from, t) {
                if let Some(id) = game.0.order.form_by_editor_id(sound) {
                    sounds.0.push(id);
                }
            }
            *sound_at = Some((name, t));
        }
    }
    // A key or click while they show jumps to the last of them, the main
    // menu's background (`StartMenu::vf0E`, `00683e10` → `00658af0`).
    if title.welcoming
        && (keys.get_just_pressed().next().is_some()
            || mouse.get_just_pressed().next().is_some())
    {
        if let Some(show) = title.show.as_mut() {
            if show.skip_to_last(now) {
                println!("Main menu: the welcome screens skipped to the last.");
            }
        }
    }
    // The welcome screens have come round (`StartMenu::vf0B`): the main
    // menu's pictures take over and the title fades in.
    if title.welcoming && title.show.as_ref().is_some_and(Slideshow::wrapped) {
        title.welcoming = false;
        let pictures = menu_pictures(&game.0);
        println!("Main menu: the welcome screens done; {} loading screens.", pictures.len());
        if let Some(show) = title.show.as_mut() {
            show.replace(pictures, now);
        }
        if let Some(screen) = menus.screen.as_deref_mut() {
            for m in &mut screen.open {
                if let OpenMenu::Start(s) = m {
                    s.menu.release_title();
                }
            }
        }
    }
    let leave = title.leave;
    let open = menus.screen.as_deref_mut().is_some_and(|screen| {
        let mut any = false;
        for m in &mut screen.open {
            if let OpenMenu::Start(s) = m {
                if s.menu.is_title() {
                    if leave {
                        s.menu.closed = true;
                    }
                    any = true;
                }
            }
        }
        any
    });
    if leave || !open {
        title.up = false;
        title.leave = false;
        // After New the slideshow stays as the loading screen.
        if title.loading.is_none() {
            title.show = None;
        }
        crate::music::menu_song_stop(&mut music);
        virt.unpause();
        println!("Main menu closed.");
    }
}

/// A load in the game begins (`006598d0`): its loading screens
/// (`world::loading_screens::in_game`) in the slideshow, changing every
/// `[Loading] fLoadingBkgdUpdateInterval`; the menu with its texts is made on
/// the next frame (`in_game_texts`).
fn begin_in_game(
    title: &mut TitleScreen,
    game: &Game,
    state: &world::scripting::GameState,
    to: Option<world::loading_screens::Destination>,
    now: f64,
) {
    let only = state.more.location_screens_only;
    let screens = world::loading_screens::in_game(&game.order, to, only, roller());
    println!(
        "Loading screen for {to:?} ({now:.2} s real): {} picture(s){}.",
        screens.len(),
        if only { ", the place's own only" } else { "" }
    );
    let interval = game
        .settings
        .float("Loading", "fLoadingBkgdUpdateInterval")
        .map_or(loading::IN_GAME_INTERVAL, f64::from);
    let pictures = screens.iter().map(|s| s.picture.clone()).collect();
    title.show = Some(Slideshow::new(pictures, interval, now));
    title.screens = screens;
    title.loading = Some(now);
    title.loaded_at = None;
    title.ready_frames = 0;
    title.shown = None;
    title.stats_at = now;
}

/// The in-game loading menu's texts (`00658bf0`, `00658840`, `00658760`):
/// made once there's a screen, then the shown picture's plate and words as
/// pictures change, and the next three statistics every
/// `[Loading] fLoadingTextUpdateInterval`.
fn in_game_texts(
    title: &mut TitleScreen,
    menus: &mut GameMenus,
    game: &Game,
    state: &world::scripting::GameState,
    now: f64,
) {
    let Some(screen) = menus.screen.as_deref_mut() else {
        return;
    };
    let values: Vec<u32> = (0..world::stats::NAMES.len())
        .map(|i| world::stats::get(state, i as u8))
        .collect();
    // The menus were built again (the window's size changed): the loading
    // menu is made again in them.
    if title.in_game.is_some() && title.in_game_built != screen.built {
        title.in_game = None;
        title.shown = None;
    }
    if title.in_game.is_none() {
        let mut menu = InGameMenu::new(roller());
        match screen.load(game, loading::FILE, &mut menu) {
            Ok(tile) => {
                menu.menu = tile;
                let (level, progress) = level_progress(&game.order, state);
                let objective = current_objective(&game.order, state);
                menu.show_progress(&mut screen.ui, objective.as_deref(), level, progress);
                menu.next_stats(&mut screen.ui, &world::stats::NAMES, &values);
                title.in_game = Some(menu);
                title.in_game_built = screen.built;
                title.stats_at = now;
            }
            Err(e) => {
                println!("The loading menu can't be shown: {e}");
                return;
            }
        }
    }
    let TitleScreen {
        in_game,
        show,
        screens,
        shown,
        stats_at,
        ..
    } = title;
    let (Some(menu), Some(show)) = (in_game.as_mut(), show.as_ref()) else {
        return;
    };
    let current = show.in_view();
    if current.is_some() && *shown != current {
        if let Some(s) = current.and_then(|i| screens.get(i)) {
            let icon = s.picture.strip_prefix("textures\\").unwrap_or(&s.picture);
            menu.show_screen(&mut screen.ui, icon, &s.text);
        }
        *shown = current;
    }
    let interval = game
        .settings
        .float("Loading", "fLoadingTextUpdateInterval")
        .map_or(loading::IN_GAME_INTERVAL, f64::from);
    if now - *stats_at >= interval {
        menu.next_stats(&mut screen.ui, &world::stats::NAMES, &values);
        *stats_at = now;
    }
}

/// The player's level and how far to the next (`00657c10`): 0 at the most
/// there is.
fn level_progress(order: &esm::LoadOrder, state: &world::scripting::GameState) -> (u32, f32) {
    let level = state.player_level;
    if level >= world::experience::max_level(order) {
        return (u32::from(level), 0.0);
    }
    let from = world::experience::xp_for_level(order, level);
    let to = world::experience::xp_for_level(order, level + 1);
    let xp = world::experience::xp(state);
    let progress = if to > from { (xp - from) / (to - from) } else { 0.0 };
    (u32::from(level), progress.clamp(0.0, 1.0) as f32)
}

/// The current objective (`007705b0`): the active quest's first one shown and
/// not done.
fn current_objective(order: &esm::LoadOrder, state: &world::scripting::GameState) -> Option<String> {
    let active = state.active_quest?;
    let (&(_, index), _) = state
        .objectives
        .iter()
        .find(|(&(q, _), &done)| q == active && !done)?;
    world::quest::Quest::load(order, active)
        .and_then(|q| q.objective(index).map(|o| o.text.clone()))
}

/// The loading screen over everything while a place loads, unless a movie
/// has the screen: black, the slideshow and the in-game menu's texts.
pub(crate) fn loading_draws(
    ui: &mut ui::Ui,
    built: u32,
    game: &Game,
    title: &mut TitleScreen,
    items: &mut Vec<DrawItem>,
    files: &mut dyn ui::draw::Textures,
    now: f64,
) {
    if title.loading.is_none() || title.up || title.movie_up {
        return;
    }
    let TitleScreen {
        show,
        model,
        in_game,
        in_game_built,
        ..
    } = title;
    // Made in an earlier build of the menus: black until it's made again.
    let Some(menu) = current_menu(in_game.as_ref(), *in_game_built, built) else {
        items.push(black(ui.screen, 1.0e6));
        return;
    };
    let depth = ui.screen_depth(menu.menu);
    items.push(black(menu.menu, depth));
    if let Some(show) = show.as_ref() {
        let model = model.get_or_insert_with(|| LoadingModel::read(game));
        if let Some(model) = model.as_ref() {
            let width = ui.number(ui.screen, t::WIDTH);
            model.draws(show, now, [width / 2.0, loading::MODEL_Y], menu.menu, depth, items);
        }
    }
    items.extend(ui::draw_list(ui, menu.menu, files, &|_| None));
}

/// The loading menu, if it was made in this build of the menus (`built`): the
/// menus are built again when the window's size changes, and a tile of the old
/// ones is nothing (or something else) in the new.
fn current_menu(menu: Option<&InGameMenu>, made_in: u32, built: u32) -> Option<&InGameMenu> {
    menu.filter(|_| made_in == built)
}

/// Black over the whole screen.
fn black(tile: TileId, depth: f32) -> DrawItem {
    DrawItem {
        tile,
        depth,
        color: [0.0, 0.0, 0.0, 1.0],
        kind: DrawKind::Model {
            texture: None,
            triangles: vec![
                [
                    ([0.0, 0.0], [0.0, 0.0]),
                    ([4000.0, 0.0], [1.0, 0.0]),
                    ([0.0, 4000.0], [0.0, 1.0]),
                ],
                [
                    ([4000.0, 0.0], [1.0, 0.0]),
                    ([4000.0, 4000.0], [1.0, 1.0]),
                    ([0.0, 4000.0], [0.0, 1.0]),
                ],
            ],
            alpha: Vec::new(),
            blend: None,
        },
    }
}

/// The slideshow under the main menu's other pictures: black over the world
/// behind, then the model's pieces as they stand now.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draws(
    ui: &mut ui::Ui,
    open: &[OpenMenu],
    game: &Game,
    title: &mut TitleScreen,
    items: &mut Vec<DrawItem>,
    menu_tile: TileId,
    first: usize,
    now: f64,
) {
    if !title.up {
        return;
    }
    let mine = open.iter().any(|m| {
        matches!(m, OpenMenu::Start(s) if s.menu.menu == menu_tile && s.menu.is_title())
    });
    if !mine {
        return;
    }
    let TitleScreen { show, model, .. } = title;
    let Some(show) = show.as_ref() else {
        return;
    };
    let model = model.get_or_insert_with(|| LoadingModel::read(game));
    let depth = ui.screen_depth(menu_tile);
    let width = ui.number(ui.screen, t::WIDTH);
    let mut out = vec![DrawItem {
        tile: menu_tile,
        depth,
        color: [0.0, 0.0, 0.0, 1.0],
        kind: DrawKind::Model {
            texture: None,
            triangles: vec![
                [
                    ([0.0, 0.0], [0.0, 0.0]),
                    ([4000.0, 0.0], [1.0, 0.0]),
                    ([0.0, 4000.0], [0.0, 1.0]),
                ],
                [
                    ([4000.0, 0.0], [1.0, 0.0]),
                    ([4000.0, 4000.0], [1.0, 1.0]),
                    ([0.0, 4000.0], [0.0, 1.0]),
                ],
            ],
            alpha: Vec::new(),
            blend: None,
        },
    }];
    if let Some(model) = model.as_ref() {
        model.draws(show, now, [width / 2.0, loading::MODEL_Y], menu_tile, depth, &mut out);
    }
    let at = first.min(items.len());
    items.splice(at..at, out);
    // The credits over everything of the menu's, from the credits rectangle
    // (x 640, moved 214 right when the screen isn't 4:3, `0063ce90`).
    if let Some(roll) = &title.credits {
        use ui::menus::credits;
        if let Some(y) = credits::scroll((now - roll.started) as f32, roll.speed, roll.length) {
            let x = 640.0 + if width > 1280.5 { credits::WIDESCREEN_SHIFT } else { 0.0 };
            let height = ui.number(ui.screen, t::HEIGHT);
            items.extend(credits::draw_items(
                &ui.fonts,
                &roll.entries,
                [x, y],
                height,
                menu_tile,
                depth + 10.0,
            ));
        }
    }
}

/// One piece of the slideshow's model.
struct Piece {
    /// The nodes from the top down to the shape, each with its own transform:
    /// what the sequences move.
    nodes: Vec<(String, nif::Transform)>,
    /// The top node's turn: the file's tilt, which stands the model up with
    /// the picture across x and y and depth along z.
    tilt: nif::math::Mat3,
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    triangles: Vec<[u16; 3]>,
    texture: Option<String>,
    blend: Option<(u8, u8)>,
    /// Which slide it is (0 `Slide01`, 1 `Slide02`), whose texture changes.
    slide: Option<usize>,
}

/// `LoadingAnim01.NIF`: its pieces, farthest first, and its sequences.
pub struct LoadingModel {
    pieces: Vec<Piece>,
    sequences: HashMap<String, nif::Sequence>,
}

impl LoadingModel {
    fn read(game: &Game) -> Option<LoadingModel> {
        let bytes = game.assets.read(loading::MODEL).ok().flatten()?;
        let nif = nif::Nif::parse(bytes)
            .map_err(|e| println!("  {}: {e}", loading::MODEL))
            .ok()?;
        let scene = nif.placed_scene().ok()?;
        let sequences: HashMap<String, nif::Sequence> = nif
            .sequences()
            .ok()?
            .into_iter()
            .map(|s| (s.name.clone(), s))
            .collect();
        let mut pieces: Vec<(f32, Piece)> = Vec::new();
        for mesh in &scene.meshes {
            let tf = &mesh.transform;
            let tilt = scene
                .root_transform
                .map_or(nif::Transform::IDENTITY.rotation, |t| t.rotation);
            let z = mesh
                .positions
                .iter()
                .map(|p| {
                    let r = nif::math::mat_vec(&tf.rotation, *p);
                    let placed = [0, 1, 2].map(|k| r[k] * tf.scale + tf.translation[k]);
                    nif::math::mat_vec(&tilt, placed)[2]
                })
                .sum::<f32>()
                / mesh.positions.len().max(1) as f32;
            let slide = loading::SLIDES
                .iter()
                .position(|s| mesh.nodes.iter().any(|(n, _)| n.eq_ignore_ascii_case(s)));
            let blend = mesh
                .alpha
                .as_ref()
                .filter(|a| a.blending())
                .map(|a| (((a.flags >> 1) & 0xf) as u8, ((a.flags >> 5) & 0xf) as u8));
            pieces.push((
                z,
                Piece {
                    nodes: mesh.nodes.clone(),
                    tilt,
                    positions: mesh.positions.clone(),
                    uvs: mesh.uvs.clone(),
                    triangles: mesh.triangles.clone(),
                    texture: mesh.diffuse_texture().map(|t| t.to_ascii_lowercase()),
                    blend,
                    slide,
                },
            ));
        }
        // Farthest (largest z) first.
        pieces.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        println!(
            "  {}: {} pieces, sequences {:?}.",
            loading::MODEL,
            pieces.len(),
            sequences.keys().collect::<Vec<_>>()
        );
        Some(LoadingModel {
            pieces: pieces.into_iter().map(|(_, p)| p).collect(),
            sequences,
        })
    }

    /// The sounds a sequence's text keys name (`sound: X`) from just after
    /// `from` up to `to` (seconds into it).
    fn sounds_between(&self, name: &str, from: f32, to: f32) -> Vec<&str> {
        let Some(s) = self.sequences.get(name) else {
            return Vec::new();
        };
        s.text_keys
            .iter()
            .filter(|(t, _)| {
                let at = t - s.start;
                at > from && at <= to
            })
            .filter_map(|(_, text)| text.trim().strip_prefix("sound:").map(str::trim))
            .collect()
    }

    /// A sequence's length in seconds (none: at once).
    fn length(&self, name: &str) -> f32 {
        self.sequences
            .get(name)
            .map_or(0.0, |s| (s.stop - s.start).max(0.0))
    }

    /// The pieces as the slideshow's sequence poses them now, in the menus'
    /// units around `centre`, scaled by 1.3333 (`006598d0`).
    fn draws(
        &self,
        show: &Slideshow,
        now: f64,
        centre: [f32; 2],
        tile: TileId,
        depth: f32,
        out: &mut Vec<DrawItem>,
    ) {
        let pose = show.pose(now);
        let sequence = pose.and_then(|(name, _)| self.sequences.get(name));
        let time = match (sequence, pose) {
            (Some(s), Some((_, Some(t)))) => s.start + t.min(s.stop - s.start),
            (Some(s), _) => s.stop,
            _ => 0.0,
        };
        let tracks: HashMap<&str, &nif::Track> = sequence
            .map(|s| s.tracks.iter().map(|t| (t.node.as_str(), t)).collect())
            .unwrap_or_default();
        let k = loading::MODEL_SCALE;
        for piece in &self.pieces {
            // The top node (tilted in the file) is the one the game replaces
            // when it places the model (`nif::Nif::placed_scene`).
            let mut placed = nif::Transform::IDENTITY;
            for (name, local) in piece.nodes.iter().skip(1) {
                let local = match tracks.get(name.as_str()) {
                    Some(track) => track.sample(time).apply(local),
                    None => *local,
                };
                placed = placed.then_child(&local);
            }
            let at = |i: u16| {
                let p = piece.positions.get(i as usize).copied().unwrap_or([0.0; 3]);
                let r = nif::math::mat_vec(&placed.rotation, p);
                let v = [0, 1, 2].map(|k| r[k] * placed.scale + placed.translation[k]);
                let [x, y, _] = nif::math::mat_vec(&piece.tilt, v);
                let uv = piece.uvs.get(i as usize).copied().unwrap_or([0.0, 0.0]);
                // The menus' y runs down the screen, as the model's does.
                ([centre[0] + k * x, centre[1] + k * y], uv)
            };
            let triangles = piece.triangles.iter().map(|t| t.map(at)).collect();
            let texture = piece
                .slide
                .and_then(|s| show.picture(s))
                .map(str::to_string)
                .or_else(|| piece.texture.clone());
            out.push(DrawItem {
                tile,
                depth,
                color: [1.0, 1.0, 1.0, 1.0],
                kind: DrawKind::Model {
                    texture,
                    triangles,
                    alpha: Vec::new(),
                    blend: piece.blend,
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The intro movie skipped after the window changed size: the loading
    /// menu made before is not drawn from the new menus (it crashed the game).
    #[test]
    fn a_loading_menu_from_an_earlier_build_of_the_menus_is_not_used() {
        let menu = InGameMenu::new(roller());
        assert!(current_menu(Some(&menu), 3, 3).is_some());
        assert!(current_menu(Some(&menu), 3, 4).is_none());
        assert!(current_menu(None, 3, 3).is_none());
    }
}
