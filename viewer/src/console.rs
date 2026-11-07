//! The console (`~`): a command line over the game, as Fallout 3's. A line
//! typed and entered runs as a script statement on the player's game
//! (`world::scripting::Runner`, the interpreter the game's scripts run on):
//! `player.additem f 100`, `setstage MQ01 10`, `coc Megaton01`. A query
//! (a function whose name starts with `Get` or `Is`, or a line starting
//! with `?`) shows its value as `>> 12.00`. `tfc` switches the free camera.
//! Up and Down step through the lines entered before; `~` or Escape closes.
//!
//! [stand-in] The game's console is drawn by its own code (not a menu
//! file) and selects references with the mouse; here it's a text panel at
//! the bottom of the screen, and commands run on the player unless they name
//! a reference (`Ref.Command`).

use std::collections::VecDeque;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::dialogue::DialogueState;
use crate::scripts::Scripts;
use crate::GameFiles;

/// Lines kept on screen.
const SHOWN: usize = 12;
/// Lines kept at all.
const KEPT: usize = 200;

/// The console's state.
#[derive(Resource, Default)]
pub struct Console {
    pub open: bool,
    input: String,
    lines: VecDeque<String>,
    history: Vec<String>,
    /// The history line Up and Down are on.
    back: Option<usize>,
    /// Lines entered, run by [`run_console`].
    pending: Vec<String>,
    /// `tfc` asked for the free camera to switch (`walk::toggle_walking`).
    pub toggle_free_camera: bool,
}

impl Console {
    fn say(&mut self, line: impl Into<String>) {
        self.lines.push_back(line.into());
        while self.lines.len() > KEPT {
            self.lines.pop_front();
        }
    }

    /// The panel's text: the last lines and the one being typed.
    fn text(&self) -> String {
        let start = self.lines.len().saturating_sub(SHOWN);
        let mut out: Vec<&str> = self.lines.iter().skip(start).map(String::as_str).collect();
        let typing = format!("> {}_", self.input);
        out.push(&typing);
        out.join("\n")
    }

    /// One key typed while the console is open.
    fn key(&mut self, key: &Key, code: KeyCode) {
        if code == KeyCode::Backquote {
            self.open = false;
            return;
        }
        match key {
            Key::Escape => self.open = false,
            Key::Enter => {
                let line = std::mem::take(&mut self.input).trim().to_string();
                self.back = None;
                if !line.is_empty() {
                    self.say(format!("> {line}"));
                    if self.history.last() != Some(&line) {
                        self.history.push(line.clone());
                    }
                    self.pending.push(line);
                }
            }
            Key::Backspace => {
                self.input.pop();
            }
            Key::Space => self.input.push(' '),
            Key::ArrowUp if !self.history.is_empty() => {
                let i = self.back.map_or(self.history.len() - 1, |i| i.saturating_sub(1));
                self.back = Some(i);
                self.input = self.history[i].clone();
            }
            Key::ArrowDown => match self.back {
                Some(i) if i + 1 < self.history.len() => {
                    self.back = Some(i + 1);
                    self.input = self.history[i + 1].clone();
                }
                _ => {
                    self.back = None;
                    self.input.clear();
                }
            },
            Key::Character(c) => {
                if c != "`" && c != "~" {
                    self.input.push_str(c);
                }
            }
            _ => {}
        }
    }
}

/// Whether a console line asks for a value: a `Get…` or `Is…` function, or
/// `?expression`. The expression to ask, if so.
pub fn query(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix('?') {
        return Some(rest.trim());
    }
    let first = line.split_whitespace().next()?;
    let function = first.rsplit('.').next().unwrap_or(first).to_ascii_lowercase();
    (function.starts_with("get") || function.starts_with("is")).then_some(line)
}

#[derive(Component)]
struct ConsoleText;

pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Console>()
            .add_systems(Startup, spawn_panel)
            // Before the game reads the keys: while the console is open they
            // are its alone.
            .add_systems(
                PreUpdate,
                console_keys.after(bevy::input::InputSystem),
            )
            .add_systems(Update, (run_console, show_panel));
    }
}

fn spawn_panel(mut commands: Commands) {
    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(48.0),
            left: Val::Px(16.0),
            right: Val::Px(16.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        Visibility::Hidden,
        ConsoleText,
    ));
}

/// `~` opens it; while open, every key goes to it and the game sees none.
fn console_keys(
    mut console: ResMut<Console>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut motion: ResMut<AccumulatedMouseMotion>,
    mut typed: EventReader<KeyboardInput>,
) {
    if !console.open {
        if keys.just_pressed(KeyCode::Backquote) {
            console.open = true;
            keys.reset_all();
            typed.clear();
        }
        return;
    }
    for event in typed.read() {
        if event.state == ButtonState::Pressed {
            console.key(&event.logical_key, event.key_code);
        }
    }
    keys.reset_all();
    mouse.reset_all();
    *motion = AccumulatedMouseMotion::default();
}

/// The lines entered, run on the game.
fn run_console(
    mut console: ResMut<Console>,
    game: Res<GameFiles>,
    scripts: Res<Scripts>,
    mut state: ResMut<DialogueState>,
) {
    if console.pending.is_empty() {
        return;
    }
    let order = &game.0.order;
    for line in std::mem::take(&mut console.pending) {
        if line.eq_ignore_ascii_case("tfc") {
            console.toggle_free_camera = true;
            console.say("Free camera switched.");
            continue;
        }
        if let Some(expression) = query(&line) {
            let answer = world::scripting::Runner::new(order, &scripts.0, &mut state.0).ask(expression);
            console.say(match answer {
                Some(v) => format!(">> {v:.2}"),
                None => "Couldn't work that out.".to_string(),
            });
            continue;
        }
        if let Err(e) = script::parse(&line) {
            console.say(format!("Error: {e}"));
            continue;
        }
        let mut runner = world::scripting::Runner::new(order, &scripts.0, &mut state.0);
        let flow = runner.run_source(&line, None, None);
        runner.run_item_scripts();
        println!("Console: {line}: {flow:?}");
        if matches!(flow, script::interp::Flow::Stopped) {
            console.say("Not carried out: part of it isn't supported yet.");
        }
    }
}

fn show_panel(console: Res<Console>, mut panel: Query<(&mut Text, &mut Visibility), With<ConsoleText>>) {
    let Ok((mut text, mut visibility)) = panel.single_mut() else {
        return;
    };
    let want = if console.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *visibility != want {
        *visibility = want;
    }
    if console.open {
        let t = console.text();
        if text.0 != t {
            text.0 = t;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(c: &mut Console, s: &str) {
        for ch in s.chars() {
            let key = if ch == ' ' {
                Key::Space
            } else {
                Key::Character(ch.to_string().into())
            };
            c.key(&key, KeyCode::KeyA);
        }
    }

    /// Typing, Enter, history, and `~` or Escape closing it.
    #[test]
    fn lines_are_typed_entered_and_recalled() {
        let mut c = Console {
            open: true,
            ..Default::default()
        };
        typed(&mut c, "player.additem f 100");
        c.key(&Key::Enter, KeyCode::Enter);
        assert_eq!(c.pending, ["player.additem f 100"]);
        assert!(c.input.is_empty());
        typed(&mut c, "tfc");
        c.key(&Key::Backspace, KeyCode::Backspace);
        assert_eq!(c.input, "tf");
        c.key(&Key::Enter, KeyCode::Enter);
        c.key(&Key::ArrowUp, KeyCode::ArrowUp);
        assert_eq!(c.input, "tf");
        c.key(&Key::ArrowUp, KeyCode::ArrowUp);
        assert_eq!(c.input, "player.additem f 100");
        c.key(&Key::ArrowDown, KeyCode::ArrowDown);
        assert_eq!(c.input, "tf");
        // The console key itself isn't typed; it closes the console.
        c.key(&Key::Character("`".into()), KeyCode::Backquote);
        assert!(!c.open);
        c.open = true;
        c.key(&Key::Escape, KeyCode::Escape);
        assert!(!c.open);
    }

    /// Queries show their value; other lines run as statements.
    #[test]
    fn get_and_is_functions_are_queries() {
        assert_eq!(query("player.getav health"), Some("player.getav health"));
        assert_eq!(query("GetStage MQ01"), Some("GetStage MQ01"));
        assert_eq!(query("player.IsInCombat"), Some("player.IsInCombat"));
        assert_eq!(query("?5 + 2"), Some("5 + 2"));
        assert_eq!(query("player.additem f 100"), None);
        assert_eq!(query("setstage MQ01 10"), None);
    }
}
