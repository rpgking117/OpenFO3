//! The New Vegas host's place when the viewer is built without it.
//!
//! `crates/host` runs code translated from FalloutNV.exe 1.4.0.525 on the
//! player's own copy of that program. OpenFO3 never uses it: a Fallout 3
//! installation has no FalloutNV.exe, and every system that could call it
//! already falls back to this project's own code (`Option<host::Host>`). So
//! by default (without the cargo feature `nv-host`) the viewer is built
//! with this stand-in instead, and neither the host nor the translated
//! `engine*` crates are compiled or needed. [`Host::start`] never succeeds,
//! so nothing else here is ever reached; the rest only gives the callers the
//! same names.

use std::path::Path;

use esm::{FormId, LoadOrder};

/// Never made ([`Host::start`] always says why not).
pub struct Host {
    pub movement: player_move::PlayerMovement,
    pub quests: quest_vm::QuestVm,
}

impl Host {
    pub fn start(_data: &Path) -> Result<Host, String> {
        Err("this build has no New Vegas host (the cargo feature nv-host)".to_string())
    }

    pub fn describe(&self) -> String {
        String::new()
    }

    pub fn static_skips_tsv(&self) -> String {
        String::new()
    }

    pub fn move_player(
        &mut self,
        _order: &LoadOrder,
        _cell: Option<FormId>,
        _input: &player_move::MoveInput,
    ) -> Option<player_move::MoveOutput> {
        None
    }

    pub fn set_keys_down(&mut self, _down: &[u8]) {}

    pub fn mouse(&mut self, _motion: [i32; 3], _buttons: [bool; 8]) {}

    pub fn main_frame(&mut self) -> bool {
        false
    }

    pub fn begin_main_frame(&mut self) -> bool {
        false
    }

    pub fn finish_main_frame(&mut self) -> bool {
        false
    }

    pub fn run_ai_threads(&mut self, _seconds: f32) -> Option<Vec<player_move::BodyPose>> {
        None
    }
}

impl world::scripting::QuestVm for Host {
    fn run_quest(
        &mut self,
        _order: &LoadOrder,
        _quest: FormId,
        _script: FormId,
        _parsed: &script::Script,
        _locals: &mut script::interp::Locals,
        _seconds_passed: f32,
    ) -> bool {
        false
    }
}

pub mod player_move {
    use esm::FormId;

    #[derive(Debug, Clone, Default)]
    pub struct PlayerMovement {
        pub probe: bool,
    }

    impl PlayerMovement {
        pub fn summary(&self) -> String {
            String::new()
        }

        pub fn stops_tsv(&self) -> String {
            String::new()
        }

        pub fn missing_tsv(&self, _names: &dyn Fn(u32) -> Option<(String, String)>) -> String {
            String::new()
        }
    }

    pub struct BodyPose {
        pub reference: FormId,
        pub rotation: [[f32; 3]; 3],
        pub translation: [f32; 3],
    }

    pub mod move_bits {
        pub const WALKING: u32 = 0x100;
    }

    // Filled by the callers; nothing here reads them.
    #[allow(dead_code)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MoveInput {
        pub feet: [f32; 3],
        pub heading: f32,
        pub move_bits: u32,
        pub seconds: f32,
        pub jump: bool,
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct MoveOutput {
        pub feet: [f32; 3],
        pub velocity: [f32; 3],
    }
}

pub mod quest_vm {
    use esm::LoadOrder;

    #[derive(Debug, Clone, Default)]
    pub struct QuestVm;

    impl QuestVm {
        pub fn summary(&self) -> String {
            String::new()
        }
    }

    /// None: without the host no quest runs through the game's interpreter.
    pub fn eligible(_order: &LoadOrder, _scripts: &world::scripting::ScriptCache) -> (usize, usize) {
        (0, 0)
    }
}
