//! `--background` test runs stay out of the user's way: the window opens
//! behind the others without the focus and never holds the mouse
//! (`main.rs`). Windows still hands the focus to a new window when the
//! user hasn't typed for a while; when that happens the focus goes straight
//! back to the window that had it before this one opened.

use bevy::prelude::*;
use bevy::window::WindowFocused;

/// The window in front when the viewer started (Windows' window handle).
#[derive(Resource, Clone, Copy)]
pub struct UserWindow(pub isize);

#[cfg(windows)]
mod win {
    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> isize;
        pub fn SetForegroundWindow(window: isize) -> i32;
    }
}

/// The window in front now (0 off Windows).
pub fn foreground() -> isize {
    #[cfg(windows)]
    {
        // SAFETY: no arguments; returns a handle or 0.
        unsafe { win::GetForegroundWindow() }
    }
    #[cfg(not(windows))]
    {
        0
    }
}

/// A test run plays only what its script asks: whatever the user does at the
/// keyboard and mouse is forgotten, even with the window handed the focus for
/// a moment, so their typing can't move or shoot in it. (Before the test keys,
/// which are then pressed as asked.)
pub fn forget_real_input(
    user: Option<Res<UserWindow>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut motion: ResMut<bevy::input::mouse::AccumulatedMouseMotion>,
    mut scroll: ResMut<bevy::input::mouse::AccumulatedMouseScroll>,
) {
    // (The resource is always there: 0 in a normal game, whose input is the
    // player's.)
    if user.is_none_or(|u| u.0 == 0) {
        return;
    }
    keys.reset_all();
    mouse.reset_all();
    *motion = Default::default();
    *scroll = Default::default();
}

/// Gives the focus back when the window takes it.
pub fn give_focus_back(mut focused: EventReader<WindowFocused>, user: Option<Res<UserWindow>>) {
    let Some(user) = user else {
        focused.clear();
        return;
    };
    for event in focused.read() {
        if event.focused && user.0 != 0 {
            #[cfg(windows)]
            // SAFETY: a window handle taken from Windows; a stale one only
            // makes the call fail.
            unsafe {
                win::SetForegroundWindow(user.0);
            }
            println!("--background: the window took the focus; handed back.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a key held down is still held after the system has run.
    fn held_after(window: isize) -> bool {
        let mut app = App::new();
        app.insert_resource(UserWindow(window))
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<bevy::input::mouse::AccumulatedMouseMotion>()
            .init_resource::<bevy::input::mouse::AccumulatedMouseScroll>()
            .add_systems(Update, forget_real_input);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::KeyW)
    }

    #[test]
    fn a_normal_game_keeps_the_players_input_and_a_test_run_forgets_it() {
        assert!(held_after(0), "a normal game has no user window to hand back to");
        assert!(!held_after(5), "a --background run plays only its script");
    }
}
