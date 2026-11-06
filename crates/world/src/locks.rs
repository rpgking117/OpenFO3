//! Locked doors and containers. A placed reference's `XLOC`: the lock
//! level (u8: 0 very easy, 25 easy, 50 average, 75 hard, 100 very hard,
//! above that only its key opens it), three unused bytes, the key (`KEYM`,
//! 0 for none), flags (u8, 0x04 leveled) and padding; a reference with one
//! is locked when it loads (`0041b640` reads it, then `0041cd30(1)` sets
//! the lock's locked bit, which `GetLocked`, `0050f380`, reads: the file's
//! flags never hold it, none of Fallout3.esm's 854 locks has it). Scripts
//! lock and unlock them (`Lock`, `Unlock`), terminals do it for their linked
//! reference (`XLKR`), and people's AI packages do it for the doors around
//! them that they own ([`package_doors`]: Moira's shop door, locked while she
//! eats and sleeps, unlocked when she opens up).
//!
//! Getting in: with the key; else picking it. A lock's difficulty is the
//! first bracket its level doesn't pass (`00430b40`): `iLockLevelMax…`
//! very easy 0, easy 25, average 50, hard 75, very hard 100 (the exe's
//! defaults), above that key only; a lock broken by failed forcing needs
//! its key too (`005180b0` asks `00430ae0`: `sImpossibleLock`). Otherwise
//! the lockpicking game opens (`world::lockpick`), with Lockpick at least
//! the bracket's top (`0078db00`; else `sLockpickSkillTooLow`); picking it
//! gives `iXPRewardPickLock<bracket>` the first time (`0078eb50`).

use esm::{FormId, FourCC, LoadOrder};

use crate::cell::le_u32;
use crate::scripting::{Facts, GameState};

const XLOC: FourCC = FourCC::new(b"XLOC");
const XLKR: FourCC = FourCC::new(b"XLKR");

/// The Lockpick skill's actor value.
pub const LOCKPICK: u16 = 36;

/// A lock: its level and key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lock {
    pub level: u8,
    pub key: Option<FormId>,
}

impl Lock {
    /// Whether only the key opens it.
    pub fn key_only(&self) -> bool {
        self.level > 100
    }

    /// Its difficulty (0 very easy … 4 very hard; 5 key only) and the
    /// Lockpick skill that takes (`00430b40`, `0078db00`).
    pub fn difficulty(&self, order: &LoadOrder) -> (u8, u16) {
        for (i, word) in crate::experience::DIFFICULTIES.iter().enumerate() {
            let top = crate::scripting::game_setting(order, &format!("iLockLevelMax{word}"))
                .unwrap_or(25.0 * i as f32);
            if f32::from(self.level) <= top {
                return (i as u8, top.max(0.0) as u16);
            }
        }
        (5, u16::from(self.level))
    }
}

/// The lock a reference was placed with (`XLOC`), if any.
pub fn placed_lock(order: &LoadOrder, reference: FormId) -> Option<Lock> {
    let rr = order.get(reference)?;
    let record = rr.record_shared().ok()?;
    let s = record.get(XLOC).filter(|s| !s.data.is_empty())?;
    let key = (s.data.len() >= 8)
        .then(|| rr.plugin.to_global(FormId(le_u32(&s.data, 4))))
        .filter(|k| k.0 != 0);
    Some(Lock {
        level: s.data[0],
        key,
    })
}

/// The lock on a reference now: as placed, unless scripts or the player
/// changed it (`GameState::locks`: `None` unlocked, `Some(level)` locked).
pub fn lock_now(order: &LoadOrder, state: &GameState, reference: FormId) -> Option<Lock> {
    let placed = placed_lock(order, reference);
    match state.locks.get(&reference) {
        Some(None) => None,
        Some(Some(level)) => Some(Lock {
            level: *level,
            key: placed.and_then(|l| l.key),
        }),
        None => placed,
    }
}

/// Whether someone other than the player opens `reference`'s lock as its
/// owner (`004aa0f0` → `004a9f60`, Fallout3.exe 1.7.0.3): its own owner
/// (`SetOwnership`, else its `XOWN`) is their base, or a faction (form type
/// 8) they're in. The nursery's door is Dad's (`CG01PlayroomDoor`, owner
/// `CG01Dad`): he goes out and back through it while it's locked.
///
/// Not carried over: the game also wants their rank in an owning faction
/// at the reference's `XRNK` or above (`0044f6a0`; ranks aren't kept
/// here), and lets anyone through who carries the key (`006f6ff0`).
pub fn opens_as_owner(
    order: &LoadOrder,
    state: &GameState,
    reference: FormId,
    actor: FormId,
) -> bool {
    if actor == crate::dialogue::PLAYER_REF {
        return false;
    }
    let Some(owner) = crate::script_functions::own_owner(order, state, reference) else {
        return false;
    };
    crate::scripting::base_of(order, actor) == Some(owner)
        || crate::factions::factions_of(order, state, actor).contains(&owner)
}

/// A reference's linked reference (`XLKR`: a keyword then the reference,
/// or just the reference).
pub fn linked_ref(order: &LoadOrder, reference: FormId) -> Option<FormId> {
    let rr = order.get(reference)?;
    let record = rr.record().ok()?;
    let s = record.get(XLKR).filter(|s| s.data.len() >= 4)?;
    let at = if s.data.len() >= 8 { 4 } else { 0 };
    Some(rr.plugin.to_global(FormId(le_u32(&s.data, at)))).filter(|f| f.0 != 0)
}

/// A package's door flags (`PKDT`'s general flags, the package's `+0x1c`):
/// lock or unlock the doors around as it starts (`005a3b50`, `005a3b60`),
/// as another takes its place (`005a3b70`, `005a3b80`) and on reaching its
/// location (`005a3b90`, `005a3ba0`). Moira's
/// (`MegMoiraOfferServices8x12`) unlocks at its start; her eating, sleeping
/// and default ones lock at their start and at their location.
pub mod package_flags {
    pub const LOCK_AT_START: u32 = 0x08;
    pub const LOCK_AT_END: u32 = 0x10;
    pub const LOCK_AT_LOCATION: u32 = 0x20;
    pub const UNLOCK_AT_START: u32 = 0x40;
    pub const UNLOCK_AT_END: u32 = 0x80;
    pub const UNLOCK_AT_LOCATION: u32 = 0x100;
}

/// How far around the person the doors are (`006f71b0` passes 5000.0,
/// `0x459c4000`, to the cell's sweep `0043c140`).
pub const PACKAGE_DOOR_REACH: f32 = 5000.0;

/// When in a package's run its door flags are looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageMoment {
    /// It starts (`006f71b0`).
    Start,
    /// Another takes its place (`00709630`, on the one going).
    End,
}

/// What a package does to the doors at `moment` by its `flags`: `true` to
/// lock, `false` to unlock, in order. At its start the lock wins
/// (`006f71b0` asks it first, the unlock only without it); at its end both
/// can happen, the unlock first (`00709630`).
pub fn package_door_changes(flags: u32, moment: PackageMoment) -> Vec<bool> {
    use package_flags::*;
    match moment {
        PackageMoment::Start if flags & LOCK_AT_START != 0 => vec![true],
        PackageMoment::Start if flags & UNLOCK_AT_START != 0 => vec![false],
        PackageMoment::Start => Vec::new(),
        PackageMoment::End => {
            let mut out = Vec::new();
            if flags & UNLOCK_AT_END != 0 {
                out.push(false);
            }
            if flags & LOCK_AT_END != 0 {
                out.push(true);
            }
            out
        }
    }
}

/// A package's door flags carried out for `who` at `moment`
/// ([`package_door_changes`]): each door (`DOOR`) within
/// [`PACKAGE_DOOR_REACH`] of them with a lock (as placed, or one a script
/// gave it) that's theirs to lock ([`locks_for`]) is locked or unlocked
/// (`0074b090`, `0074b130` → `004ec600`, `004ec660`: the lock keeps its
/// level). Not for the player, nor for the engine's own packages (`005a6400`:
/// combat, dialogue and the like, types 0x12 and up). The doors changed.
///
/// A load door's own cell and its far side's, each if it has an owner
/// (`004daae0`), are opened or shut with it (`00423d80` → `004d3360`: the
/// cell's flag 0x40, which makes an owned cell public,
/// `script_functions::cell_is_public`).
///
/// Not carried over: the "at its location" flags (`005a5bd0` decides that).
pub fn package_doors(
    order: &LoadOrder,
    state: &mut GameState,
    who: FormId,
    package_kind: u8,
    flags: u32,
    moment: PackageMoment,
) -> Vec<FormId> {
    let changes = package_door_changes(flags, moment);
    if changes.is_empty() || who == crate::dialogue::PLAYER_REF || package_kind >= 0x12 {
        return Vec::new();
    }
    let Some((space, cell, at, _)) = state.place(order, who) else {
        return Vec::new();
    };
    let mut changed = Vec::new();
    for door in doors_around(order, space, cell, at) {
        let has_lock = placed_lock(order, door).is_some() || state.locks.contains_key(&door);
        if !has_lock || !locks_for(order, state, door, who) {
            continue;
        }
        for &lock in &changes {
            if lock {
                let level = match state.locks.get(&door) {
                    Some(Some(level)) => *level,
                    _ => placed_lock(order, door).map_or(0, |l| l.level),
                };
                state.locks.insert(door, Some(level));
            } else {
                state.locks.insert(door, None);
            }
            open_cells_beside(order, state, door, !lock);
        }
        changed.push(door);
    }
    changed
}

/// `00423d80`: a load door's cell and the far side's, those with an owner,
/// opened (flag 0x40 set) or shut.
fn open_cells_beside(order: &LoadOrder, state: &mut GameState, door: FormId, open: bool) {
    let Some(rr) = order.get(door) else {
        return;
    };
    let Some(teleport) = crate::placement::teleport_of(&rr) else {
        return;
    };
    let here = order.cell_of(&rr);
    let there = order.get(teleport.door).and_then(|far| order.cell_of(&far));
    for cell in [there, here].into_iter().flatten() {
        if crate::script_functions::cell_owner_now(order, state, cell).is_some() {
            state.set_by_scripts.opened_cells.insert(cell, open);
        }
    }
}

/// The people placed in `cell` (the player's arriving there) have been
/// following their packages all along (the game's low process starts them
/// while the player is elsewhere, `0074e8b0`, `007507f0` → `006f71b0`):
/// each one's current package's start is carried out for the doors
/// ([`package_doors`]) before anything asks about them. The doors changed.
pub fn settle_package_doors(order: &LoadOrder, state: &mut GameState, cell: FormId) -> Vec<FormId> {
    let people: Vec<FormId> = order
        .references_in_cell(cell)
        .into_iter()
        .filter(|r| {
            let k = r.entry.header.kind;
            (k == FourCC::new(b"ACHR") || k == FourCC::new(b"ACRE")) && !r.entry.header.is_deleted()
        })
        .map(|r| r.form_id)
        .filter(|r| !state.dead.contains(r) && crate::enabled_now(order, *r, &state.disabled))
        .collect();
    let mut changed = Vec::new();
    for who in people {
        let Some(p) = crate::ai::current_package(order, state, who) else {
            continue;
        };
        // Begun already (`world::ai::actions::begin`): its start was had.
        if state.package_begun.get(&who).map(|b| b.0) == Some(p.form_id) {
            continue;
        }
        changed.extend(package_doors(order, state, who, p.kind, p.flags, PackageMoment::Start));
    }
    changed
}

/// Whether `who` locks and unlocks `door` (`004f3990`): it has no owner of
/// its own (anyone does), or it's theirs, or a faction's they're in, when
/// it's a load door or the faction's second flags byte has 0x02 (its `+0x34`
/// bit 9). A door has no owner through its cell (`004f26c0` skips that for
/// doors).
pub fn locks_for(order: &LoadOrder, state: &GameState, door: FormId, who: FormId) -> bool {
    let Some(owner) = crate::script_functions::own_owner(order, state, door) else {
        return true;
    };
    if crate::scripting::base_of(order, who) == Some(owner) {
        return true;
    }
    let Some(faction) = order
        .get(owner)
        .filter(|r| r.entry.header.kind == FourCC::new(b"FACT"))
    else {
        return false;
    };
    let load_door = order
        .get(door)
        .is_some_and(|r| crate::placement::teleport_of(&r).is_some());
    let flag = faction
        .record()
        .ok()
        .and_then(|r| r.get(FourCC::new(b"DATA")).map(|s| s.data.clone()))
        .is_some_and(|d| d.get(1).is_some_and(|f| f & 0x02 != 0));
    (load_door || flag) && crate::factions::factions_of(order, state, who).contains(&owner)
}

/// The doors (references of a `DOOR`) within [`PACKAGE_DOOR_REACH`] of
/// `at`: in an interior its cell's; outdoors the squares around and the
/// worldspace's persistent ones (the game looks through the loaded squares,
/// `00502290`).
fn doors_around(order: &LoadOrder, space: FormId, cell: FormId, at: [f32; 3]) -> Vec<FormId> {
    let mut cells = vec![cell];
    if order
        .get(space)
        .is_some_and(|r| r.entry.header.kind == FourCC::new(b"WRLD"))
    {
        cells.clear();
        if let Ok(grid) = crate::WorldGrid::load(order, space) {
            let (x, y) = crate::square_of(at);
            for dx in -2..=2 {
                for dy in -2..=2 {
                    if let Some(&c) = grid.cells.get(&(x + dx, y + dy)) {
                        cells.push(c);
                    }
                }
            }
            cells.extend(grid.persistent);
        }
    }
    let reach = PACKAGE_DOOR_REACH * PACKAGE_DOOR_REACH;
    let mut out = Vec::new();
    for c in cells {
        for rr in order.references_in_cell(c) {
            if rr.entry.header.kind != FourCC::new(b"REFR") || rr.entry.header.is_deleted() {
                continue;
            }
            let Some(p) = crate::placement_of(order, rr.form_id) else {
                continue;
            };
            if p.base_type != FourCC::new(b"DOOR") {
                continue;
            }
            let d: f32 = (0..3).map(|i| (p.position[i] - at[i]).powi(2)).sum();
            if d <= reach {
                out.push(rr.form_id);
            }
        }
    }
    out
}

/// How the player gets past a lock, or why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opening {
    Open,
    WithKey,
    /// The lockpicking game opens on it (`world::lockpick`), a lock of
    /// this difficulty (0..4).
    Pick(u8),
    NeedsKey,
    NeedsSkill(u16),
}

/// The player tries a locked reference (`005180b0`): open already; their
/// key (it's unlocked for good); a lock only its key opens, or broken by
/// failed forcing (`sImpossibleLock`); too little Lockpick
/// (`sLockpickSkillTooLow`, `0078db00`); else the lockpicking game, which
/// unlocks it if it's won (`world::lockpick::picked`).
pub fn try_open(order: &LoadOrder, state: &mut GameState, reference: FormId) -> Opening {
    let Some(lock) = lock_now(order, state, reference) else {
        return Opening::Open;
    };
    let player = crate::dialogue::PLAYER_REF;
    let opening = if lock
        .key
        .is_some_and(|k| state.item_count(order, player, k) > 0)
    {
        Opening::WithKey
    } else if lock.key_only()
        || lock.difficulty(order).0 > 4
        || crate::lockpick::is_broken(order, state, reference)
    {
        Opening::NeedsKey
    } else {
        let skill = Facts {
            order,
            state,
            speaker: None,
        }
        .current_actor_value(player, LOCKPICK)
        .unwrap_or(0.0)
        .floor();
        let (difficulty, needs) = lock.difficulty(order);
        if skill >= f64::from(needs) {
            Opening::Pick(difficulty)
        } else {
            Opening::NeedsSkill(needs)
        }
    };
    if opening == Opening::WithKey {
        state.locks.insert(reference, None);
        if let Some(key) = lock.key {
            unlocked_with(order, state, key);
        }
    }
    opening
}

/// Opened with the key (`005180b0` for doors, `00516dc0` for containers):
/// `UILockpickingUnlock`, and "Unlocked with <key>." (`sOpenWithKey`, the
/// key's name for its `%s`) with the key picture.
fn unlocked_with(order: &LoadOrder, state: &mut GameState, key: FormId) {
    use crate::scripting::Event;
    if let Some(sound) = order.form_by_editor_id("UILockpickingUnlock") {
        state.events.push(Event::Sound(sound));
    }
    let name = order
        .get(key)
        .and_then(|r| r.record().ok())
        .and_then(|r| r.full_name())
        .unwrap_or_default();
    let text = crate::scripting::game_setting_text(order, "sOpenWithKey")
        .unwrap_or_else(|| "Unlocked with %s.".to_string())
        .replacen("%s", &name, 1);
    state.events.push(Event::Message {
        title: None,
        text,
        buttons: Vec::new(),
        icon: Some(crate::message_icon::KEY.to_string()),
    });
}
