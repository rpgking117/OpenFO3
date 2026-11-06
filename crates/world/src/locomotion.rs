//! How fast someone walks and runs, when the player may run and jump, and
//! how a jump starts and is steered in the air. The physics crate moves the
//! character controller; the viewer reads the keys.
//!
//! - Speed ([`walk_speed`], [`run_speed`], [`speed`]): from `Fallout3.exe`
//!   1.7.0.3, `005845b0` and `00584710`, called by `Actor::vf112` (walking,
//!   `006fb200`) and `Actor::vf113` (running, `006fb2e0`). Fallout 3 slows
//!   people by the weight of what they have equipped, not by armour class
//!   and weapon type as New Vegas does.
//!
//! The rest is still New Vegas's (`FalloutNV.exe` 1.4.0.525, evidence and gaps:
//! `docs/MOVEMENT.md`), to be re-derived from Fallout 3 (`audit/nv_sourced.txt`):
//! - Running and jumping allowed ([`may_run`], [`may_jump`]): the player's
//!   controls handler at `0093e860`.
//! - The jump ([`jump_height`], [`jump_speed`]): `00930640` and the jumping
//!   state `00cd4280`; steering in the air ([`air_gain`]): the in-air state
//!   `00cd3fb0`.

use esm::{FormId, FourCC, LoadOrder};

use crate::body_parts::{av as part_av, ignores_crippled_limbs, is_crippled};
use crate::scripting::{base_of, game_setting, Facts, GameState};

const CREA: FourCC = FourCC::new(b"CREA");

/// SpeedMult, the actor value the speed scales with (21).
pub const SPEED_MULT: u16 = 21;

/// The upper-body biped slot (index 2, mask 0x4): the armour whose weight
/// class slows its wearer (`00885a50` asks `00891b90` for slot 2).
pub const UPPER_BODY: u32 = 1 << 2;

/// The game settings the speed rules read (`GMST` where the data sets
/// them, else the exe's defaults; `FalloutNV.esm` sets the base speed, the
/// sneak multiplier, both armour penalties and the big two-hander penalty).
#[derive(Debug, Clone, PartialEq)]
pub struct SpeedSettings {
    /// `fMoveBaseSpeed` (exe 85, data 77).
    pub base: f32,
    /// `fMoveRunMult` (4).
    pub run_mult: f32,
    /// `fMoveSneakMult` (exe 0.6, data 0.57).
    pub sneak_mult: f32,
    /// `fMoveNoWeaponMult` (1.1): people with their weapon put away.
    pub no_weapon_mult: f32,
    /// `fMoveWeightMin` (0) and `fMoveWeightMax` (150): the equipped weight
    /// that slows, between them (above the most counts as the most).
    pub weight_min: f32,
    pub weight_max: f32,
    /// `fMoveEncumEffect` (0.4), and `fMoveEncumEffectNoWea` (0.3) for people
    /// with their weapon put away: how much the most weight slows.
    pub encum_effect: f32,
    pub encum_effect_no_weapon: f32,
    /// `fMoveOneCrippledLegSpeedMult` (0.85), `fMoveTwoCrippledLegsSpeedMult`
    /// (0.75).
    pub one_leg: f32,
    pub two_legs: f32,
    /// `fJumpHeightMin` (64, not in the data) and `fJumpSwimmingMult` (2).
    pub jump_height: f32,
    pub jump_swimming_mult: f32,
    /// `fGrabMaxWeightRunning` (50): carrying (grabbing) something heavier
    /// stops the player running.
    pub grab_max_weight_running: f32,
}

impl SpeedSettings {
    pub fn read(order: &LoadOrder) -> SpeedSettings {
        let d = SpeedSettings::defaults();
        let g = |name: &str, default: f32| game_setting(order, name).unwrap_or(default);
        SpeedSettings {
            base: g("fMoveBaseSpeed", d.base),
            run_mult: g("fMoveRunMult", d.run_mult),
            sneak_mult: g("fMoveSneakMult", d.sneak_mult),
            no_weapon_mult: g("fMoveNoWeaponMult", d.no_weapon_mult),
            weight_min: g("fMoveWeightMin", d.weight_min),
            weight_max: g("fMoveWeightMax", d.weight_max),
            encum_effect: g("fMoveEncumEffect", d.encum_effect),
            encum_effect_no_weapon: g("fMoveEncumEffectNoWea", d.encum_effect_no_weapon),
            one_leg: g("fMoveOneCrippledLegSpeedMult", d.one_leg),
            two_legs: g("fMoveTwoCrippledLegsSpeedMult", d.two_legs),
            jump_height: g("fJumpHeightMin", d.jump_height),
            jump_swimming_mult: g("fJumpSwimmingMult", d.jump_swimming_mult),
            grab_max_weight_running: g("fGrabMaxWeightRunning", d.grab_max_weight_running),
        }
    }

    /// The exe's own defaults (the settings' static initialisers; Fallout3.exe
    /// `00f61080` .. `00f610d4`, `00f62298`, `00f622a4` for the speed's).
    pub fn defaults() -> SpeedSettings {
        SpeedSettings {
            base: 85.0,
            run_mult: 4.0,
            sneak_mult: 0.6,
            no_weapon_mult: 1.1,
            weight_min: 0.0,
            weight_max: 150.0,
            encum_effect: 0.4,
            encum_effect_no_weapon: 0.3,
            one_leg: 0.85,
            two_legs: 0.75,
            jump_height: 64.0,
            jump_swimming_mult: 2.0,
            grab_max_weight_running: 50.0,
        }
    }
}

/// What the speed depends on, as `Actor::vf112` (`006fb200`) gathers it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedFacts {
    /// SpeedMult (actor value 21).
    pub speed_mult: f32,
    /// Legs crippled (left and right mobility conditions at most 0,
    /// `00646800`): 0, 1 or 2.
    pub crippled_legs: u8,
    /// IgnoreCrippledLimbs (actor value 72) above 0.
    pub ignores_crippled_limbs: bool,
    /// Actor vfunc +0x358 (the player's `00954cc0`, over-encumbered; 0 for
    /// other people and creatures, `0047c850`). While it holds the legs'
    /// multiplier applies even to those ignoring crippled limbs.
    pub over_encumbered: bool,
    /// Actor vfunc +0x218: 1 for people and the player (`008d0360`), 0 for
    /// creatures (`0047c850`): only people move faster with their weapon
    /// put away.
    pub person: bool,
    /// The weight of what's equipped (`0047f1e0`: armour and weapons, not
    /// ammunition), the value `Actor::vf112` keeps through slots 0x30c / 0x310.
    pub equipped_weight: f32,
    /// Whether the weapon is drawn (process vfunc +0x450).
    pub weapon_drawn: bool,
    /// Sneaking: movement flags 0x400 without 0x800 (`004997b0`).
    pub sneaking: bool,
}

impl Default for SpeedFacts {
    fn default() -> Self {
        SpeedFacts {
            speed_mult: 100.0,
            crippled_legs: 0,
            ignores_crippled_limbs: false,
            over_encumbered: false,
            person: true,
            equipped_weight: 0.0,
            weapon_drawn: false,
            sneaking: false,
        }
    }
}

/// Walking speed, game units a second (Fallout3.exe `005845b0`):
/// `fMoveBaseSpeed` × SpeedMult × 0.01 × the legs' multiplier (one crippled
/// × 0.85, both × 0.75; × 1 when ignoring crippled limbs, unless
/// over-encumbered), × `fMoveNoWeaponMult` for a person with the weapon put
/// away, × (1 − the encumbrance effect × (weight − `fMoveWeightMin`) ÷
/// (`fMoveWeightMax` − `fMoveWeightMin`)), the weight at most
/// `fMoveWeightMax` and the effect `fMoveEncumEffect`, or
/// `fMoveEncumEffectNoWea` with the weapon away; × `fMoveSneakMult` sneaking;
/// below 0 → 0.
// Translated from 005845b0 (decompiled, Fallout3.exe 1.7.0.3)
pub fn walk_speed(settings: &SpeedSettings, facts: &SpeedFacts) -> f32 {
    let weight = facts.equipped_weight.min(settings.weight_max);
    let mut legs = match facts.crippled_legs {
        1 => settings.one_leg,
        2 => settings.two_legs,
        _ => 1.0,
    };
    if !facts.over_encumbered && facts.ignores_crippled_limbs {
        legs = 1.0;
    }
    let scaled = (f64::from(facts.speed_mult) * 0.01) as f32;
    let mut speed = settings.base * scaled * legs;
    let mut effect = settings.encum_effect;
    if facts.person && !facts.weapon_drawn {
        speed *= settings.no_weapon_mult;
        effect = settings.encum_effect_no_weapon;
    }
    let span = settings.weight_max - settings.weight_min;
    if span > 0.0 {
        speed *= 1.0 - effect * ((weight - settings.weight_min) / span);
    }
    if facts.sneaking {
        speed *= settings.sneak_mult;
    }
    if speed >= 0.0 {
        speed
    } else {
        0.0
    }
}

/// Running speed (Fallout3.exe `00584710`): walking × `fMoveRunMult`.
// Translated from 00584710 (decompiled, Fallout3.exe 1.7.0.3)
pub fn run_speed(settings: &SpeedSettings, facts: &SpeedFacts) -> f32 {
    (walk_speed(settings, facts) + 0.0) * settings.run_mult
}

/// Gathers [`SpeedFacts`] for someone from the game's state.
pub fn speed_facts(
    order: &LoadOrder,
    state: &GameState,
    who: FormId,
    sneaking: bool,
) -> SpeedFacts {
    let facts = Facts {
        order,
        state,
        speaker: None,
    };
    let speed_mult = facts
        .current_actor_value(who, SPEED_MULT)
        .map_or(100.0, |v| v as f32);
    let crippled_legs = [part_av::LEFT_MOBILITY, part_av::RIGHT_MOBILITY]
        .into_iter()
        .filter(|&v| is_crippled(order, state, who, v))
        .count() as u8;
    let player = who == crate::dialogue::PLAYER_REF;
    let person = player
        || base_of(order, who)
            .and_then(|b| order.get(b))
            .map_or(true, |r| r.entry.header.kind != CREA);
    // `0047f1e0`: the equipped items' weights (ammunition weighs nothing).
    let mut equipped: Vec<FormId> = crate::impacts::worn_armour(order, state, who);
    if let Some(w) = crate::combat::weapon_in_hand(order, state, who) {
        equipped.push(w.form_id);
    }
    equipped.sort();
    equipped.dedup();
    let equipped_weight = equipped
        .into_iter()
        .map(|item| crate::items::weight(order, item, false).max(0.0))
        .sum();
    SpeedFacts {
        speed_mult,
        crippled_legs,
        ignores_crippled_limbs: ignores_crippled_limbs(order, state, who),
        over_encumbered: player && state.over_encumbered(order, who),
        person,
        equipped_weight,
        weapon_drawn: state.weapon_out.contains(&who),
        sneaking,
    }
}

/// Someone's speed, game units a second: [`run_speed`] running, else
/// [`walk_speed`] (`006fd7f0`). Fallout 3's `Actor::vf113` applies no perk
/// to the run speed (New Vegas's "Modify Run Speed" entry point isn't there);
/// both multiply by a factor from actor vfunc +0x41c when it returns
/// something (not traced, not applied).
pub fn speed(
    order: &LoadOrder,
    state: &GameState,
    settings: &SpeedSettings,
    who: FormId,
    running: bool,
    sneaking: bool,
) -> f32 {
    let facts = speed_facts(order, state, who, sneaking);
    if running {
        run_speed(settings, &facts)
    } else {
        walk_speed(settings, &facts)
    }
}

/// Whether the player runs (movement flag 0x200; Fallout3.exe
/// `PlayerCharacter::vfBE` `00788350`, New Vegas's `0093e860`, keyboard; the
/// aiming test is FO3's process vfunc +0x400 through `006f9660`):
/// running is wanted (the Run key held, or with Always Run on not held),
/// and the player isn't over-encumbered, isn't looking down the sights
/// (process vfunc +0x404, `GetIronSights` (Xbox PDB)) and isn't carrying
/// something heavier than `fGrabMaxWeightRunning`. (Nor in process anim
/// action 7, vfunc +0x3e4 `GetAnimAction` (Xbox PDB); which action that is
/// isn't traced, and it isn't modelled.)
pub fn may_run(
    settings: &SpeedSettings,
    wanted: bool,
    over_encumbered: bool,
    iron_sights: bool,
    grabbed_weight: f32,
) -> bool {
    wanted && !over_encumbered && !iron_sights && grabbed_weight <= settings.grab_max_weight_running
}

/// Whether the Jump control may start a jump (`0093e860`): never while
/// over-encumbered (actor vfunc +0x358, the player's `00954cc0`). The
/// other tests on the way (menus, the sit state, a special idle) belong to
/// their own systems.
pub fn may_jump(over_encumbered: bool) -> bool {
    !over_encumbered
}

/// How high a jump goes (Fallout3.exe `MobileObject::vf95`, `007668b0`):
/// `fJumpHeightMin` × the actor's whole scale (`004e6d80`), ×
/// `fJumpSwimmingMult` swimming (controller state 5). The same as New Vegas's
/// `00930640`.
// Translated from 007668b0 (decompiled, Fallout3.exe 1.7.0.3)
pub fn jump_height(settings: &SpeedSettings, scale: f32, swimming: bool) -> f32 {
    let h = scale * settings.jump_height;
    if swimming {
        h * settings.jump_swimming_mult
    } else {
        h
    }
}

/// The jump's upward speed (the jumping state `00cd4280`): √(2 × |gravity ×
/// the controller's gravity multiplier| × height), units a second, for
/// `gravity` in units a second squared (the multiplier is 1).
pub fn jump_speed(gravity: f32, height: f32) -> f32 {
    (2.0 * gravity.abs() * height.abs()).sqrt()
}

/// The controller's air control (info +0x8c, copied to controller +0x534):
/// actor vfunc +0x2e4, 1 for everyone (`0087d6c0`).
pub const AIR_CONTROL: f32 = 1.0;
/// `bhkCharacterController::fJumpMoveMult` and `fJumpMoveBase` (Xbox PDB):
/// statics 0.3 (`011b0140`) and 0 (`01267bbc`), not the game settings of
/// the same names (nothing copies those).
pub const JUMP_MOVE_MULT: f32 = 0.3;
pub const JUMP_MOVE_BASE: f32 = 0.0;

/// The share of the gap between the wanted and the current horizontal
/// velocity the in-air state closes each controller update (`00cd3fb0`):
/// air control × 0.3 + 0, or all of it when the controller flags 0x1800
/// are set (not traced; left out). No air control (0) leaves the velocity
/// alone.
pub fn air_gain(air_control: f32) -> f32 {
    if air_control > 0.0 {
        air_control * JUMP_MOVE_MULT + JUMP_MOVE_BASE
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> SpeedSettings {
        SpeedSettings::defaults()
    }

    /// Fallout 3's walking speed (`005845b0`): the base, × 1.1 with the
    /// weapon away, less the equipped weight's share of 150 times 0.4 (0.3
    /// with the weapon away), × 0.6 sneaking; running × 4.
    #[test]
    fn walking_and_running_follow_fallout_3s_formula() {
        let s = data();
        let f = SpeedFacts::default();
        // Weapon away, nothing equipped: 85 × 1.1.
        assert_eq!(walk_speed(&s, &f), 85.0f32 * 1.1);
        let drawn = SpeedFacts {
            weapon_drawn: true,
            ..f
        };
        assert_eq!(walk_speed(&s, &drawn), 85.0);
        assert_eq!(run_speed(&s, &drawn), 340.0);
        // 30 weight drawn: × (1 − 0.4 × 30 / 150).
        let loaded = SpeedFacts {
            equipped_weight: 30.0,
            ..drawn
        };
        assert_eq!(walk_speed(&s, &loaded), 85.0 * (1.0 - 0.4 * (30.0 / 150.0)));
        // Weapon away: × 1.1 and the smaller effect, 0.3.
        let away = SpeedFacts {
            weapon_drawn: false,
            ..loaded
        };
        assert_eq!(
            walk_speed(&s, &away),
            85.0 * 1.1 * (1.0 - 0.3 * (30.0 / 150.0))
        );
        // Above the most counts as the most.
        let power_armour = SpeedFacts {
            equipped_weight: 400.0,
            ..drawn
        };
        assert_eq!(walk_speed(&s, &power_armour), 85.0 * (1.0 - 0.4));
        // Sneaking × 0.6, after the rest.
        let sneak = SpeedFacts {
            sneaking: true,
            ..drawn
        };
        assert_eq!(walk_speed(&s, &sneak), 85.0 * 0.6);
        // Creatures don't get the weapon-away bonus, and keep the 0.4.
        let creature = SpeedFacts {
            person: false,
            equipped_weight: 30.0,
            ..f
        };
        assert_eq!(walk_speed(&s, &creature), 85.0 * (1.0 - 0.4 * (30.0 / 150.0)));
        // Never below 0.
        let stuck = SpeedFacts {
            speed_mult: -50.0,
            ..f
        };
        assert_eq!(walk_speed(&s, &stuck), 0.0);
    }

    #[test]
    fn crippled_legs_slow_unless_ignored_and_not_over_encumbered() {
        let s = data();
        let one = SpeedFacts {
            crippled_legs: 1,
            weapon_drawn: true,
            ..SpeedFacts::default()
        };
        assert_eq!(walk_speed(&s, &one), 85.0 * 0.85);
        let two = SpeedFacts {
            crippled_legs: 2,
            ..one
        };
        assert_eq!(walk_speed(&s, &two), 85.0 * 0.75);
        let ignored = SpeedFacts {
            ignores_crippled_limbs: true,
            ..two
        };
        assert_eq!(walk_speed(&s, &ignored), 85.0);
        // Over-encumbered the game doesn't ask about ignoring them.
        let heavy = SpeedFacts {
            over_encumbered: true,
            ..ignored
        };
        assert_eq!(walk_speed(&s, &heavy), 85.0 * 0.75);
    }

    #[test]
    fn running_and_jumping_are_refused_as_the_controls_do() {
        let s = data();
        assert!(may_run(&s, true, false, false, 0.0));
        assert!(!may_run(&s, false, false, false, 0.0));
        assert!(!may_run(&s, true, true, false, 0.0));
        assert!(!may_run(&s, true, false, true, 0.0));
        assert!(may_run(&s, true, false, false, 50.0));
        assert!(!may_run(&s, true, false, false, 50.5));
        assert!(may_jump(false));
        assert!(!may_jump(true));
    }

    #[test]
    fn jumps_rise_their_height_and_steer_at_three_tenths() {
        let s = data();
        assert_eq!(jump_height(&s, 1.0, false), 64.0);
        assert_eq!(jump_height(&s, 1.0, true), 128.0);
        let g = 686.614;
        let v = jump_speed(g, 64.0);
        // v² = 2gh: the apex 64 up.
        assert!((v * v / (2.0 * g) - 64.0).abs() < 1e-3);
        assert!((v - 296.45).abs() < 0.01, "{v}");
        assert_eq!(air_gain(AIR_CONTROL), 0.3);
        assert_eq!(air_gain(0.0), 0.0);
    }
}
