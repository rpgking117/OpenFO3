//! What the face menu (`RaceSexMenu`) changes about the player, from
//! `Fallout3.exe` 1.7.0.3. The menu edits the player's base NPC: race
//! (`+0x110`), hair (`+0x198`) and its length (`+0x19c`), eyes (`+0x1a0`),
//! hair colour (`+0x1b8`), head parts (`+0x1bc`) and FaceGen coordinates
//! (`+0x134`, or the block at `+0x194`). Here that is [`PlayerFace`], kept in
//! the game state; the record stays as it is.
//!
//! * A face's coordinates are its own values; what the sliders and the
//!   statistics read is the race's for the sex plus those (`00554d10`, adding
//!   with `0058e230`), and a change is stored back less the race's
//!   (`0058e510`).
//! * Presets (`005595d0`): the people of that race and sex with the preset
//!   flag (`ACBS` flag 4, `+0x34` bit 2) or, when none has it, all of them; not
//!   the player; sorted by name (`00555980`, a byte compare of `+0xd4`).
//! * A preset (`005579a0`) gives its hair (a person with a template: the
//!   race's default), length, eyes, hair colour, head parts and coordinates.
//! * Changing race or sex (the click at `00670e40`) takes the new first preset
//!   (`0066ff30`), then keeps the hair and eyes only if they suit
//!   (`0066fbe0`, [`super::appearance::reconcile_parts`]); female empties the
//!   head parts first.
//! * Randomize (`0066e530` → `0055a310(1, 0, 1)`) keeps age and hair length.

use esm::{FormId, FourCC, LoadOrder};
use nif::ctl::{Ctl, Kind, Stat, Symmetry};

use super::appearance::{self, Choice, PartSelection};
use super::facegen::{self, Coords};
use crate::dialogue::PLAYER_BASE;

const NPC_: FourCC = FourCC::new(b"NPC_");
const HDPT: FourCC = FourCC::new(b"HDPT");
const ACBS: FourCC = FourCC::new(b"ACBS");
const RNAM: FourCC = FourCC::new(b"RNAM");
const HNAM: FourCC = FourCC::new(b"HNAM");
const LNAM: FourCC = FourCC::new(b"LNAM");
const ENAM: FourCC = FourCC::new(b"ENAM");
const PNAM: FourCC = FourCC::new(b"PNAM");
const HCLR: FourCC = FourCC::new(b"HCLR");
const FGGS: FourCC = FourCC::new(b"FGGS");
const FGGA: FourCC = FourCC::new(b"FGGA");
const FGTS: FourCC = FourCC::new(b"FGTS");
const TPLT: FourCC = FourCC::new(b"TPLT");
const FULL: FourCC = FourCC::new(b"FULL");
const OBND: FourCC = FourCC::new(b"OBND");
const DATA: FourCC = FourCC::new(b"DATA");
const DNAM: FourCC = FourCC::new(b"DNAM");

/// `ACBS` flags: female, and a face the menu offers as a preset.
const FEMALE: u32 = 1;
const PRESET: u32 = 4;

/// The FaceGen controls' file (`0058ee80`).
pub const CONTROLS_FILE: &str = "facegen\\si.ctl";

/// What the face menu has made of the player.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerFace {
    pub race: FormId,
    pub hair: Option<FormId>,
    pub hair_length: f32,
    pub eyes: Option<FormId>,
    pub head_parts: Vec<FormId>,
    pub hair_color: [u8; 3],
    /// Their own coordinates: the race's aren't in these.
    pub own: Coords,
}

fn le_u32(data: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(..4)?.try_into().ok()?))
}

fn floats(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

impl PlayerFace {
    /// A person's record as a face; `None` for anything else.
    pub fn of_npc(order: &LoadOrder, npc: FormId) -> Option<PlayerFace> {
        let rr = order.get(npc).filter(|r| r.entry.header.kind == NPC_)?;
        let record = rr.record().ok()?;
        let form = |kind: FourCC| {
            record
                .get(kind)
                .and_then(|s| le_u32(&s.data))
                .filter(|&raw| raw != 0)
                .map(|raw| rr.plugin.to_global(FormId(raw)))
        };
        let values = |kind: FourCC| {
            record
                .get(kind)
                .map(|s| floats(&s.data))
                .unwrap_or_default()
        };
        let mut hair_color = [0; 3];
        if let Some(s) = record.get(HCLR).filter(|s| s.data.len() >= 3) {
            hair_color.copy_from_slice(&s.data[..3]);
        }
        Some(PlayerFace {
            race: form(RNAM)?,
            hair: form(HNAM),
            hair_length: record
                .get(LNAM)
                .and_then(|s| le_u32(&s.data))
                .map_or(0.0, f32::from_bits),
            eyes: form(ENAM),
            head_parts: record
                .get_all(PNAM)
                .filter_map(|s| le_u32(&s.data))
                .filter(|&raw| raw != 0)
                .map(|raw| rr.plugin.to_global(FormId(raw)))
                .collect(),
            hair_color,
            own: Coords([
                [values(FGGS), values(FGGA)],
                [values(FGTS), Vec::new()],
            ]),
        })
    }

    /// The player as the game has them made: the menu's, else the record's.
    pub fn of_player(order: &LoadOrder, made: Option<&PlayerFace>) -> Option<PlayerFace> {
        made.cloned()
            .or_else(|| PlayerFace::of_npc(order, PLAYER_BASE))
    }
}

/// The race's coordinates for a sex: `FGGS`, `FGGA`, `FGTS` after the `MNAM`
/// or `FNAM` following its hair and eye lists (`race + 0x458`, `+ 0x3f8`).
pub fn race_coords(order: &LoadOrder, race: FormId, female: bool) -> Coords {
    crate::actor::race_face_values(order, race, female)
        .map(|[s, a, t]| Coords([[s, a], [t, Vec::new()]]))
        .unwrap_or_default()
}

fn combine(a: &[f32], b: &[f32], sign: f32) -> Vec<f32> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).copied().unwrap_or(0.0) + sign * b.get(i).copied().unwrap_or(0.0))
        .collect()
}

fn each(a: &Coords, b: &Coords, sign: f32) -> Coords {
    let mut out = Coords::default();
    for kind in 0..2 {
        for symmetry in 0..2 {
            out.0[kind][symmetry] = combine(&a.0[kind][symmetry], &b.0[kind][symmetry], sign);
        }
    }
    out
}

/// The race's coordinates plus the face's own (`00554d10`).
pub fn total(order: &LoadOrder, face: &PlayerFace, female: bool) -> Coords {
    each(&race_coords(order, face.race, female), &face.own, 1.0)
}

/// Stores coordinates back as the face's own: less the race's (`0058e510`).
fn store(order: &LoadOrder, face: &mut PlayerFace, female: bool, total: &Coords) {
    face.own = each(total, &race_coords(order, face.race, female), -1.0);
}

/// A slider's change (`0066d560`): the control set on the total.
pub fn set_control(
    order: &LoadOrder,
    ctl: &Ctl,
    face: &mut PlayerFace,
    female: bool,
    kind: Kind,
    index: usize,
    value: f32,
) {
    let mut all = total(order, face, female);
    facegen::set_control(ctl, &mut all, kind, Symmetry::Symmetric, index, value);
    store(order, face, female, &all);
}

/// Every symmetric control's value of a kind, by index (`0058f320`).
pub fn control_values(ctl: &Ctl, total: &Coords, kind: Kind) -> Vec<f32> {
    (0..ctl.controls(kind, Symmetry::Symmetric).len())
        .map(|i| facegen::control(ctl, total, kind, Symmetry::Symmetric, i))
        .collect()
}

/// The preset faces for a race and sex (`005595d0`).
pub fn presets(order: &LoadOrder, race: FormId, female: bool) -> Vec<FormId> {
    let mut named = Vec::new();
    let mut flagged = Vec::new();
    for rr in order.records_of_type(NPC_) {
        if rr.entry.header.is_deleted() || rr.form_id == PLAYER_BASE {
            continue;
        }
        let Ok(record) = rr.record() else {
            continue;
        };
        let their_race = record
            .get(RNAM)
            .and_then(|s| le_u32(&s.data))
            .map(|raw| rr.plugin.to_global(FormId(raw)));
        if their_race != Some(race) {
            continue;
        }
        let flags = record
            .get(ACBS)
            .and_then(|s| le_u32(&s.data))
            .unwrap_or(0);
        if (flags & FEMALE != 0) != female {
            continue;
        }
        let name = record.get(FULL).map(|s| s.zstring()).unwrap_or_default();
        if flags & PRESET != 0 {
            flagged.push((name.clone(), rr.form_id));
        }
        named.push((name, rr.form_id));
    }
    let mut list = if flagged.is_empty() { named } else { flagged };
    // `00555980`: a byte compare of their names; `qsort` leaves ties unsaid,
    // so they stay in the order found.
    list.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    list.into_iter().map(|(_, id)| id).collect()
}

/// A preset taken (`005579a0`).
pub fn apply_preset(order: &LoadOrder, face: &mut PlayerFace, preset: FormId, female: bool) {
    let Some(from) = PlayerFace::of_npc(order, preset) else {
        return;
    };
    let templated = order
        .get(preset)
        .and_then(|r| r.record().ok())
        .is_some_and(|r| r.get(TPLT).is_some());
    face.hair = if templated {
        race_default_hair(order, face.race, female)
    } else {
        from.hair
    };
    face.hair_length = from.hair_length;
    face.eyes = from.eyes;
    face.hair_color = from.hair_color;
    face.head_parts = from.head_parts;
    face.own = from.own;
}

/// A race's default hair for the sex (`0055f3f0`: `DNAM`).
fn race_default_hair(order: &LoadOrder, race: FormId, female: bool) -> Option<FormId> {
    let rr = order.get(race)?;
    let record = rr.record().ok()?;
    let data = record.get(DNAM)?;
    let raw = le_u32(data.data.get(usize::from(female) * 4..)?)?;
    (raw != 0).then(|| rr.plugin.to_global(FormId(raw)))
}

/// After a change of race or sex (`0066ff30`, `0066fbe0`).
fn after_change(order: &LoadOrder, face: &mut PlayerFace, female: bool) {
    if let Some(&first) = presets(order, face.race, female).first() {
        apply_preset(order, face, first, female);
    }
    if let Some(parts) = appearance::reconcile_parts(
        order,
        face.race,
        female,
        PartSelection {
            hair: face.hair,
            eyes: face.eyes,
        },
    ) {
        face.hair = parts.hair;
        face.eyes = parts.eyes;
    }
}

/// Another race (the click, kind 0x17).
pub fn change_race(order: &LoadOrder, face: &mut PlayerFace, race: FormId, female: bool) {
    face.race = race;
    after_change(order, face, female);
}

/// Another sex (the click, kinds 0x15, 0x16): 0x16 empties the head parts
/// first (`006c8220`).
pub fn change_sex(order: &LoadOrder, face: &mut PlayerFace, female: bool) {
    if female {
        face.head_parts.clear();
    }
    after_change(order, face, female);
}

/// A race's older or younger self (`ONAM`, `+0x4c4`; `YNAM`, `+0x4c8`: `AgeRace`
/// follows `+0x4c8` for a negative number, and CG02's `player.AgeRace -1` is
/// commented "change player to a child").
fn older_or_younger(order: &LoadOrder, race: FormId, kind: FourCC) -> Option<FormId> {
    let rr = order.get(race)?;
    let record = rr.record().ok()?;
    let raw = le_u32(&record.get(kind)?.data)?;
    (raw != 0).then(|| rr.plugin.to_global(FormId(raw)))
}

/// How far to follow a race's ages before giving up on a loop in the data.
const AGES: usize = 16;

/// `MatchRace` (`0052f490`): the other's race at this one's age. As many steps
/// from the youngest as this race is from its own youngest.
pub fn match_race(order: &LoadOrder, mine: FormId, theirs: FormId) -> FormId {
    let older = FourCC::new(b"ONAM");
    let younger = FourCC::new(b"YNAM");
    let mut steps = 0;
    let mut at = mine;
    while let Some(next) = older_or_younger(order, at, younger).filter(|_| steps < AGES) {
        steps += 1;
        at = next;
    }
    let mut result = theirs;
    for _ in 0..AGES {
        match older_or_younger(order, result, younger) {
            Some(next) => result = next,
            None => break,
        }
    }
    for _ in 0..steps {
        match older_or_younger(order, result, older) {
            Some(next) => result = next,
            None => break,
        }
    }
    result
}

/// `AgeRace` (`0052f3b0`): that many steps older, or younger for a negative
/// number, stopping where the race has no more.
pub fn age_race(order: &LoadOrder, race: FormId, steps: i32) -> FormId {
    let kind = FourCC::new(if steps < 0 { b"YNAM" } else { b"ONAM" });
    let mut at = race;
    for _ in 0..steps.unsigned_abs().min(AGES as u32) {
        match older_or_younger(order, at, kind) {
            Some(next) => at = next,
            None => break,
        }
    }
    at
}

/// Keeps a face for whoever's it is: the player's, or a person's by base.
pub fn put_face(state: &mut crate::scripting::GameState, base: FormId, face: PlayerFace) {
    if base == PLAYER_BASE {
        state.player_face = Some(face);
    } else {
        state.npc_faces.insert(base, face);
    }
}

/// The player's face as it's drawn: while young (`SetPCYoung`, `CG01`'s "default
/// hair") the race's default hair for the sex, whatever was chosen (`00560dd0`).
pub fn shown(order: &LoadOrder, state: &crate::scripting::GameState) -> Option<PlayerFace> {
    let mut face = PlayerFace::of_player(order, state.player_face.as_ref())?;
    if state.player_young {
        let female = state.player_female.unwrap_or(false);
        face.hair = race_default_hair(order, face.race, female);
    }
    Some(face)
}

/// Someone's race when it has been changed from their record's (`AgeRace`,
/// `MatchRace`, the face menu).
pub fn changed_race(
    order: &LoadOrder,
    state: &crate::scripting::GameState,
    who: FormId,
) -> Option<FormId> {
    let base = crate::scripting::base_of(order, who)?;
    if base == PLAYER_BASE {
        state.player_face.as_ref().map(|f| f.race)
    } else {
        state.npc_faces.get(&base).map(|f| f.race)
    }
}

/// The player's whole scale, as Fallout3.exe's `004e6d80` gives it ("GetScale
/// … with base", `Eval_GetScale` `0050f2a0`): the reference's own (`SetScale`,
/// `+0x38`) × the base's height, the player's race's for the sex. What jumps
/// (`MobileObject::vf95` `007668b0`) and the eye's height scale by.
pub fn player_scale(order: &LoadOrder, state: &crate::scripting::GameState) -> f32 {
    let own = state
        .scales
        .get(&crate::dialogue::PLAYER_REF)
        .copied()
        .filter(|s| *s > 0.0)
        .unwrap_or(1.0);
    let race = state
        .player_face
        .as_ref()
        .map(|f| f.race)
        .or_else(|| PlayerFace::of_player(order, None).map(|f| f.race));
    let female = state.player_female.unwrap_or(false);
    let height = race
        .and_then(|r| race_height(order, r, female))
        .filter(|h| *h > 0.0)
        .unwrap_or(1.0);
    own * height
}

/// A race's height for a sex (`DATA`, bytes 16 and 20: `race + 0x60`, `0x005556c0`).
pub fn race_height(order: &LoadOrder, race: FormId, female: bool) -> Option<f32> {
    let record = order.get(race)?.record().ok()?;
    let at = 16 + usize::from(female) * 4;
    let data = record.get(DATA)?;
    let bytes: [u8; 4] = data.data.get(at..at + 4)?.try_into().ok()?;
    Some(f32::from_le_bytes(bytes))
}

/// `MatchFaceGeometry` (`0053ff90` → `0053f550`): `percent` of the way from
/// this face to the other's (own values, not the races'), at its own age, its
/// texture staying its own, stored less the race's male values.
///
/// **Read, not compared:** the last step takes the race off values that never
/// had it on, as the decompile reads; which face each of its three inlined reads
/// is hasn't been checked in the instructions.
pub fn match_face(
    order: &LoadOrder,
    ctl: &Ctl,
    face: &mut PlayerFace,
    other: &PlayerFace,
    percent: f32,
) {
    let share = percent / 100.0;
    let age = facegen::stat(ctl, &face.own, Stat::Age, Kind::Shape);
    let mut part = each(&other.own, &face.own, -1.0);
    for by_symmetry in &mut part.0 {
        for values in by_symmetry {
            for v in values {
                *v *= share;
            }
        }
    }
    let mut moved = each(&face.own, &part, 1.0);
    facegen::set_stat(ctl, &mut moved, Stat::Age, Kind::Shape, age);
    let mut stored = each(&moved, &race_coords(order, face.race, false), -1.0);
    stored.0[1] = face.own.0[1].clone();
    face.own = stored;
}

/// The face a reference's person has: the player's, a person's matched one, or
/// their record's.
pub fn face_of(
    order: &LoadOrder,
    state: &crate::scripting::GameState,
    reference: FormId,
) -> Option<(FormId, PlayerFace)> {
    let base = crate::scripting::base_of(order, reference)?;
    let made = if base == PLAYER_BASE {
        state.player_face.clone()
    } else {
        state.npc_faces.get(&base).cloned()
    };
    Some((base, made.or_else(|| PlayerFace::of_npc(order, base))?))
}

/// Numbers Randomize draws (`0055a310`): a normal one (`00be54b0`, the polar
/// method) and a uniform one in 0..1 (`0086c4b0`).
pub trait Dice {
    fn normal(&mut self) -> f32;
    fn uniform(&mut self) -> f32;
}

/// How far Randomize goes from the race's face (`00f5cacc` × the 1.5 in
/// `00bdccc0`).
const RANDOM_SPREAD: f32 = 0.5 * 1.5;

/// Randomize (`0055a310(1, 0, 1)`): the race's symmetric coordinates plus
/// noise, nothing asymmetric, age as it comes out (kept within 15 to 65), a
/// new gender in -2..2 with the texture's moved by the same amount.
pub fn randomize(
    order: &LoadOrder,
    ctl: &Ctl,
    face: &mut PlayerFace,
    female: bool,
    dice: &mut dyn Dice,
) {
    let race = race_coords(order, face.race, female);
    let mut all = Coords::default();
    for kind in 0..2 {
        all.0[kind][0] = race.0[kind][0]
            .iter()
            .map(|v| v + dice.normal() * RANDOM_SPREAD)
            .collect();
        all.0[kind][1] = vec![0.0; race.0[kind][1].len()];
    }
    use facegen::{set_stat, stat};
    let (shape, texture) = (Kind::Shape, Kind::Texture);
    let age = stat(ctl, &all, Stat::Age, shape).clamp(15.0, 65.0);
    let age_texture = stat(ctl, &all, Stat::Age, texture).clamp(15.0, 65.0);
    set_stat(ctl, &mut all, Stat::Age, shape, age);
    set_stat(ctl, &mut all, Stat::Age, texture, age_texture);
    let gender = stat(ctl, &all, Stat::Gender, shape);
    let gender_texture = stat(ctl, &all, Stat::Gender, texture);
    let new = dice.uniform() * 4.0 - 2.0;
    let moved = gender_texture - (gender - new);
    set_stat(ctl, &mut all, Stat::Gender, shape, new.clamp(-2.0, 2.0));
    set_stat(
        ctl,
        &mut all,
        Stat::Gender,
        texture,
        moved.clamp(-2.0, 2.0),
    );
    store(order, face, female, &all);
}

/// The head parts offered (the setup at `0066ffc0`: the global list, those
/// playable, `00544680`: `HDPT`'s flag at `+0x44` bit 0).
pub fn head_parts(order: &LoadOrder) -> Vec<Choice> {
    order
        .records_of_type(HDPT)
        .filter(|rr| !rr.entry.header.is_deleted())
        .filter_map(|rr| {
            let record = rr.record().ok()?;
            let flags = *record.get(DATA)?.data.first()?;
            (flags & 1 != 0).then(|| Choice {
                form: rr.form_id,
                name: record.get(FULL).map(|s| s.zstring()),
            })
        })
        .collect()
}

/// How tall the player's model stands (`006f5de0`: the top of the bounds less
/// the bottom), which the menu's camera heights are shares of. `None` without
/// bounds.
pub fn height_of_bounds(order: &LoadOrder) -> Option<f32> {
    let record = order.get(PLAYER_BASE)?.record().ok()?;
    let data = record.get(OBND)?.data.clone();
    if data.len() < 12 {
        return None;
    }
    let i = |at: usize| f32::from(i16::from_le_bytes([data[at], data[at + 1]]));
    Some(i(10) - i(4))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed;
    impl Dice for Fixed {
        fn normal(&mut self) -> f32 {
            1.0
        }
        fn uniform(&mut self) -> f32 {
            0.75
        }
    }

    #[test]
    fn coordinates_add_and_store_back_less_the_first() {
        let mut a = Coords::default();
        a.0[0][0] = vec![1.0, 2.0];
        let mut b = Coords::default();
        b.0[0][0] = vec![0.5];
        let sum = each(&a, &b, 1.0);
        assert_eq!(sum.0[0][0], [1.5, 2.0]);
        assert_eq!(each(&sum, &a, -1.0).0[0][0], [0.5, 0.0]);
    }

    #[test]
    fn the_dice_reach_randomize_through_a_trait() {
        let mut d = Fixed;
        assert_eq!(d.normal() * RANDOM_SPREAD, 0.75);
        assert_eq!(d.uniform() * 4.0 - 2.0, 1.0);
    }
}
