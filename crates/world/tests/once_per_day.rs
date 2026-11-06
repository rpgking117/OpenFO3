//! A once-per-day package (`PKDT` flag 0x400, Fallout3.exe `005a3bb0`): done,
//! the next in the person's list comes up until the day changes (`005ac410`).

use esm::{ActivePlugins, FormId, LoadOrder};
use testdata::{f32s, group, record, sub, zstr};
use world::ai;
use world::scripting::GameState;

const DAY: u32 = 0x700;
const ONCE: u32 = 0x800;
const AFTER: u32 = 0x801;
const WALKER: u32 = 0x810;
const ROOM: u32 = 0x900;
const WALKER_REF: u32 = 0x910;

fn world() -> (testdata::TempData, LoadOrder) {
    let data = testdata::TempData::empty("once-per-day");
    let mut plugin = record(
        b"TES4",
        0,
        &sub(b"HEDR", &{
            let mut h = 0.94f32.to_le_bytes().to_vec();
            h.extend([0; 8]);
            h
        }),
    );
    let mut day = sub(b"EDID", &zstr("GameDay"));
    day.extend(sub(b"FNAM", b"f"));
    day.extend(sub(b"FLTV", &1.0f32.to_le_bytes()));
    plugin.extend(group(*b"GLOB", 0, &record(b"GLOB", DAY, &day)));
    let package = |id: u32, name: &str, flags: u32| {
        let mut d = sub(b"EDID", &zstr(name));
        let mut pkdt = flags.to_le_bytes().to_vec();
        pkdt.extend([6, 0, 0, 0, 0, 0, 0, 0]);
        d.extend(sub(b"PKDT", &pkdt));
        d.extend(sub(b"PSDT", &[0xFF, 0xFF, 0, 0xFF, 0, 0, 0, 0]));
        record(b"PACK", id, &d)
    };
    let mut packs = package(ONCE, "TestToIntercom", 0x1404);
    packs.extend(package(AFTER, "TestTalkIntoIt", 0x1004));
    plugin.extend(group(*b"PACK", 0, &packs));
    let mut npc = sub(b"EDID", &zstr("TestDad"));
    npc.extend(sub(b"ACBS", &[0u8; 24]));
    npc.extend(sub(b"PKID", &ONCE.to_le_bytes()));
    npc.extend(sub(b"PKID", &AFTER.to_le_bytes()));
    plugin.extend(group(*b"NPC_", 0, &record(b"NPC_", WALKER, &npc)));
    let mut placed = sub(b"NAME", &WALKER.to_le_bytes());
    placed.extend(sub(b"DATA", &f32s(&[0.0; 6])));
    let refs = record(b"ACHR", WALKER_REF, &placed);
    let mut c = sub(b"EDID", &zstr("Diner"));
    c.extend(sub(b"DATA", &[1]));
    let mut cell = record(b"CELL", ROOM, &c);
    cell.extend(group(ROOM.to_le_bytes(), 6, &group(ROOM.to_le_bytes(), 9, &refs)));
    plugin.extend(group(*b"CELL", 0, &group(0u32.to_le_bytes(), 2, &cell)));
    data.write("Fallout3.esm", &plugin);
    let order = LoadOrder::from_data_dir(data.path(), &ActivePlugins::OfficialOnly).unwrap();
    (data, order)
}

#[test]
fn a_finished_once_per_day_package_gives_way_until_tomorrow() {
    let (_data, order) = world();
    let mut state = GameState::new(&order);
    let who = FormId(WALKER_REF);
    let picked = |state: &GameState| ai::current_package(&order, state, who).unwrap().form_id;
    assert_eq!(picked(&state), FormId(ONCE));
    ai::mark_done_today(&order, &mut state, who, FormId(ONCE));
    assert_eq!(picked(&state), FormId(AFTER));
    // The next day it's back.
    state.globals.insert(FormId(DAY), 2.0);
    assert_eq!(picked(&state), FormId(ONCE));
}
