//! A person's AI package locks and unlocks the doors around them
//! (`world::locks::package_doors`, Fallout3.exe 1.7.0.3 `006f71b0` as a
//! package starts, `00709630` as another takes its place, `0074b090` /
//! `0074b130` for each door, `004f3990` for whose it is), on a generated
//! plugin: a shopkeeper's own door (a load door out of their shop), a
//! neighbour's, one nobody owns, one out of reach and one with no lock; the
//! shop is the keeper's, and their package opens up at its start.

use esm::{ActivePlugins, FormId, LoadOrder};
use testdata::{f32s, group, record, sub, zstr};
use world::dialogue::PLAYER_REF;
use world::locks::{
    lock_now, package_door_changes, package_doors, package_flags::*, PackageMoment,
};
use world::scripting::GameState;

const KEEPER: u32 = 0x800;
const NEIGHBOUR: u32 = 0x801;
const DOOR: u32 = 0x802;
const SHOP: u32 = 0x900;
const KEEPER_REF: u32 = 0x910;
const NEIGHBOUR_REF: u32 = 0x911;
const OWN_DOOR: u32 = 0x912;
const OTHERS_DOOR: u32 = 0x913;
const FREE_DOOR: u32 = 0x914;
const FAR_DOOR: u32 = 0x915;
const UNLOCKED_DOOR: u32 = 0x916;
const OPEN_UP: u32 = 0x920;

fn named(kind: &[u8; 4], id: u32, rest: &[u8]) -> Vec<u8> {
    let mut d = sub(b"EDID", &zstr(&format!("Test{id:X}")));
    d.extend(rest);
    record(kind, id, &d)
}

fn placed(kind: &[u8; 4], id: u32, base: u32, x: f32, extra: &[u8]) -> Vec<u8> {
    let mut d = sub(b"NAME", &base.to_le_bytes());
    d.extend(extra);
    d.extend(sub(b"DATA", &f32s(&[x, 0.0, 0.0, 0.0, 0.0, 0.0])));
    record(kind, id, &d)
}

/// A lock of level 50 (leveled, as Moira's door is: flags 0x04).
fn lock() -> Vec<u8> {
    let mut l = vec![50u8, 0, 0, 0, 0, 0, 0, 0, 0x04, 0, 0, 0];
    l.extend([0u8; 8]);
    sub(b"XLOC", &l)
}

fn owned_by(owner: u32) -> Vec<u8> {
    let mut e = sub(b"XOWN", &owner.to_le_bytes());
    e.extend(lock());
    e
}

fn world() -> (testdata::TempData, LoadOrder) {
    let data = testdata::TempData::empty("package-door-locks");
    let mut plugin = record(
        b"TES4",
        0,
        &sub(b"HEDR", &{
            let mut h = 0.94f32.to_le_bytes().to_vec();
            h.extend([0; 8]);
            h
        }),
    );
    let acbs = sub(b"ACBS", &[0u8; 24]);
    let mut keeper = acbs.clone();
    keeper.extend(sub(b"PKID", &OPEN_UP.to_le_bytes()));
    let mut npcs = named(b"NPC_", KEEPER, &keeper);
    npcs.extend(named(b"NPC_", NEIGHBOUR, &acbs));
    plugin.extend(group(*b"NPC_", 0, &npcs));
    // A sandbox (12) at any time that unlocks the doors as it starts, as
    // Moira's `MegMoiraOfferServices8x12` (flags 0x240).
    let mut pkdt = 0x240u32.to_le_bytes().to_vec();
    pkdt.extend([12, 0, 0, 0]);
    pkdt.extend([0; 4]);
    let mut package = sub(b"PKDT", &pkdt);
    package.extend(sub(b"PSDT", &[0xff, 0xff, 0, 0xff, 0, 0, 0, 0]));
    plugin.extend(group(*b"PACK", 0, &named(b"PACK", OPEN_UP, &package)));
    plugin.extend(group(*b"DOOR", 0, &named(b"DOOR", DOOR, &[])));
    let mut refs = placed(b"ACHR", KEEPER_REF, KEEPER, 0.0, &[]);
    refs.extend(placed(b"ACHR", NEIGHBOUR_REF, NEIGHBOUR, 0.0, &[]));
    // Its far side is the far door (in the shop too, for the test).
    let mut own = owned_by(KEEPER);
    let mut xtel = FAR_DOOR.to_le_bytes().to_vec();
    xtel.extend(f32s(&[0.0; 6]));
    xtel.extend([0u8; 4]);
    own.extend(sub(b"XTEL", &xtel));
    refs.extend(placed(b"REFR", OWN_DOOR, DOOR, 100.0, &own));
    refs.extend(placed(b"REFR", OTHERS_DOOR, DOOR, 200.0, &owned_by(NEIGHBOUR)));
    refs.extend(placed(b"REFR", FREE_DOOR, DOOR, 300.0, &lock()));
    refs.extend(placed(b"REFR", FAR_DOOR, DOOR, 6000.0, &lock()));
    refs.extend(placed(b"REFR", UNLOCKED_DOOR, DOOR, 50.0, &[]));
    let mut c = sub(b"EDID", &zstr("Shop"));
    c.extend(sub(b"DATA", &[1]));
    c.extend(sub(b"XOWN", &KEEPER.to_le_bytes()));
    let mut cell = record(b"CELL", SHOP, &c);
    cell.extend(group(SHOP.to_le_bytes(), 6, &group(SHOP.to_le_bytes(), 9, &refs)));
    plugin.extend(group(*b"CELL", 0, &group(0u32.to_le_bytes(), 2, &cell)));
    data.write("Fallout3.esm", &plugin);
    let order = LoadOrder::from_data_dir(data.path(), &ActivePlugins::OfficialOnly).unwrap();
    (data, order)
}

fn locked(order: &LoadOrder, state: &GameState, door: u32) -> bool {
    lock_now(order, state, FormId(door)).is_some()
}

#[test]
fn every_placed_lock_starts_locked_whatever_its_flags() {
    // `0041b640` → `0041cd30(1)`: the leveled flag (0x04) doesn't leave it
    // open.
    let (_data, order) = world();
    let state = GameState::default();
    for door in [OWN_DOOR, OTHERS_DOOR, FREE_DOOR, FAR_DOOR] {
        assert!(locked(&order, &state, door), "{door:X}");
    }
    assert!(!locked(&order, &state, UNLOCKED_DOOR));
}

#[test]
fn opening_up_unlocks_their_own_door_and_unowned_ones_in_reach() {
    let (_data, order) = world();
    let mut state = GameState::default();
    let keeper = FormId(KEEPER_REF);
    let mut changed = package_doors(&order, &mut state, keeper, 12, UNLOCK_AT_START, PackageMoment::Start);
    changed.sort();
    assert_eq!(changed, [FormId(OWN_DOOR), FormId(FREE_DOOR)]);
    assert!(!locked(&order, &state, OWN_DOOR));
    assert!(!locked(&order, &state, FREE_DOOR));
    // The neighbour's stays locked, and so does the one 6000 away.
    assert!(locked(&order, &state, OTHERS_DOOR));
    assert!(locked(&order, &state, FAR_DOOR));
    // One with no lock gets none.
    assert!(!state.locks.contains_key(&FormId(UNLOCKED_DOOR)));

    // Going to bed: locked again, at the lock's own level.
    package_doors(&order, &mut state, keeper, 4, LOCK_AT_START | LOCK_AT_LOCATION, PackageMoment::Start);
    assert_eq!(lock_now(&order, &state, FormId(OWN_DOOR)).map(|l| l.level), Some(50));
}

#[test]
fn the_shop_is_open_while_its_door_is() {
    // `00423d80`: the owned cell's flag 0x40 with the load door.
    let (_data, order) = world();
    let mut state = GameState::default();
    let shop = FormId(SHOP);
    state.player_cell = Some(shop);
    state.player_position = Some([0.0; 3]);
    assert!(world::crime::trespassing(&order, &state, shop));
    let keeper = FormId(KEEPER_REF);
    package_doors(&order, &mut state, keeper, 12, UNLOCK_AT_START, PackageMoment::Start);
    assert_eq!(state.set_by_scripts.opened_cells.get(&shop), Some(&true));
    assert!(!world::crime::trespassing(&order, &state, shop));
    package_doors(&order, &mut state, keeper, 4, LOCK_AT_START, PackageMoment::Start);
    assert!(world::crime::trespassing(&order, &state, shop));
    // Kept in a save.
    package_doors(&order, &mut state, keeper, 12, UNLOCK_AT_START, PackageMoment::Start);
    let (loaded, _) = world::save::load(&world::save::save(&state, None)).unwrap();
    assert_eq!(loaded.set_by_scripts.opened_cells.get(&shop), Some(&true));
}

#[test]
fn arriving_finds_the_doors_as_the_keepers_package_left_them() {
    // The low process had started the keeper's package before the player
    // came (`006f71b0` from `LowProcess`): their door is open, the shop too.
    let (_data, order) = world();
    let mut state = GameState::default();
    let changed = world::locks::settle_package_doors(&order, &mut state, FormId(SHOP));
    assert!(changed.contains(&FormId(OWN_DOOR)));
    assert!(!locked(&order, &state, OWN_DOOR));
    assert!(locked(&order, &state, OTHERS_DOOR));
    assert!(!world::crime::trespassing(&order, &state, FormId(SHOP)));
    // A package already begun isn't started over: a door a script locked
    // since stays locked.
    state.package_begun.insert(FormId(KEEPER_REF), (FormId(OPEN_UP), false));
    state.locks.insert(FormId(OWN_DOOR), Some(50));
    world::locks::settle_package_doors(&order, &mut state, FormId(SHOP));
    assert!(locked(&order, &state, OWN_DOOR));
}

#[test]
fn the_neighbour_locks_only_their_own() {
    let (_data, order) = world();
    let mut state = GameState::default();
    state.locks.insert(FormId(OWN_DOOR), None);
    state.locks.insert(FormId(OTHERS_DOOR), None);
    let changed = package_doors(
        &order,
        &mut state,
        FormId(NEIGHBOUR_REF),
        3,
        LOCK_AT_START,
        PackageMoment::Start,
    );
    assert!(changed.contains(&FormId(OTHERS_DOOR)));
    assert!(locked(&order, &state, OTHERS_DOOR));
    assert!(!locked(&order, &state, OWN_DOOR));
}

#[test]
fn not_for_the_player_nor_the_engines_own_packages() {
    let (_data, order) = world();
    let mut state = GameState::default();
    state.player_cell = Some(FormId(SHOP));
    state.player_position = Some([0.0; 3]);
    assert!(package_doors(&order, &mut state, PLAYER_REF, 12, UNLOCK_AT_START, PackageMoment::Start).is_empty());
    // Type 0x12 and up (`005a6400`).
    assert!(package_doors(&order, &mut state, FormId(KEEPER_REF), 0x12, UNLOCK_AT_START, PackageMoment::Start).is_empty());
    assert!(locked(&order, &state, OWN_DOOR));
}

#[test]
fn which_flags_count_when() {
    // At the start the lock is asked first and wins (`006f71b0`).
    assert_eq!(package_door_changes(LOCK_AT_START | UNLOCK_AT_START, PackageMoment::Start), [true]);
    assert_eq!(package_door_changes(UNLOCK_AT_START, PackageMoment::Start), [false]);
    // The end flags aren't the start's.
    assert!(package_door_changes(LOCK_AT_END | UNLOCK_AT_END, PackageMoment::Start).is_empty());
    // At the end both run, the unlock first (`00709630`).
    assert_eq!(package_door_changes(LOCK_AT_END | UNLOCK_AT_END, PackageMoment::End), [false, true]);
    // The "at its location" ones aren't carried over.
    assert!(package_door_changes(LOCK_AT_LOCATION | UNLOCK_AT_LOCATION, PackageMoment::Start).is_empty());
    // Moira's: open up at 8, lock to eat and sleep.
    assert_eq!(package_door_changes(0x240, PackageMoment::Start), [false]);
    assert_eq!(package_door_changes(0x228, PackageMoment::Start), [true]);
    assert_eq!(package_door_changes(0x428, PackageMoment::Start), [true]);
}
