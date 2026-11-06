//! Who opens a locked door as its owner (`world::locks::opens_as_owner`,
//! Fallout3.exe 1.7.0.3 `004aa0f0` → `004a9f60`), on a generated plugin:
//! the nursery's door is Dad's, and he walks through it locked.

use esm::{ActivePlugins, FormId, LoadOrder};
use testdata::{f32s, group, record, sub, zstr};
use world::dialogue::PLAYER_REF;
use world::scripting::GameState;

const DAD: u32 = 0x800;
const NEIGHBOUR: u32 = 0x801;
const DOOR: u32 = 0x802;
const ROOM: u32 = 0x900;
const DAD_REF: u32 = 0x910;
const NEIGHBOUR_REF: u32 = 0x911;
const DOOR_REF: u32 = 0x912;

fn named(kind: &[u8; 4], id: u32, rest: &[u8]) -> Vec<u8> {
    let mut d = sub(b"EDID", &zstr(&format!("Test{id:X}")));
    d.extend(rest);
    record(kind, id, &d)
}

fn placed(kind: &[u8; 4], id: u32, base: u32, extra: &[u8]) -> Vec<u8> {
    let mut d = sub(b"NAME", &base.to_le_bytes());
    d.extend(extra);
    d.extend(sub(b"DATA", &f32s(&[0.0; 6])));
    record(kind, id, &d)
}

fn world() -> (testdata::TempData, LoadOrder) {
    let data = testdata::TempData::empty("door-owner");
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
    let mut npcs = named(b"NPC_", DAD, &acbs);
    npcs.extend(named(b"NPC_", NEIGHBOUR, &acbs));
    plugin.extend(group(*b"NPC_", 0, &npcs));
    plugin.extend(group(*b"DOOR", 0, &named(b"DOOR", DOOR, &[])));
    let mut lock = vec![255u8, 0, 0, 0];
    lock.extend([0u8; 8]);
    let mut extra = sub(b"XOWN", &DAD.to_le_bytes());
    extra.extend(sub(b"XLOC", &lock));
    let mut refs = placed(b"ACHR", DAD_REF, DAD, &[]);
    refs.extend(placed(b"ACHR", NEIGHBOUR_REF, NEIGHBOUR, &[]));
    refs.extend(placed(b"REFR", DOOR_REF, DOOR, &extra));
    let mut c = sub(b"EDID", &zstr("Nursery"));
    c.extend(sub(b"DATA", &[1]));
    let mut cell = record(b"CELL", ROOM, &c);
    cell.extend(group(ROOM.to_le_bytes(), 6, &group(ROOM.to_le_bytes(), 9, &refs)));
    plugin.extend(group(*b"CELL", 0, &group(0u32.to_le_bytes(), 2, &cell)));
    data.write("Fallout3.esm", &plugin);
    let order = LoadOrder::from_data_dir(data.path(), &ActivePlugins::OfficialOnly).unwrap();
    (data, order)
}

#[test]
fn the_owner_opens_his_locked_door_and_no_one_else_does() {
    let (_data, order) = world();
    let mut state = GameState::default();
    let door = FormId(DOOR_REF);
    assert!(world::locks::lock_now(&order, &state, door).is_some());
    assert!(world::locks::opens_as_owner(&order, &state, door, FormId(DAD_REF)));
    assert!(!world::locks::opens_as_owner(&order, &state, door, FormId(NEIGHBOUR_REF)));
    // The player picks or unlocks it (the lock menu), never as its owner here.
    assert!(!world::locks::opens_as_owner(&order, &state, door, PLAYER_REF));
    // `SetOwnership` hands it over.
    state.set_by_scripts.owners.insert(door, FormId(NEIGHBOUR));
    assert!(world::locks::opens_as_owner(&order, &state, door, FormId(NEIGHBOUR_REF)));
    assert!(!world::locks::opens_as_owner(&order, &state, door, FormId(DAD_REF)));
}
