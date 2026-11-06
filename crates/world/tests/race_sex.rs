//! The face menu's rules (`world::chargen::race_sex`) on a plugin built from
//! scratch: what's read from `Fallout3.exe` 1.7.0.3 about presets, a change of
//! race, Randomize and a person matching the player.

use esm::{ActivePlugins, FormId, LoadOrder};
use nif::ctl::{Control, Ctl, Kind, Linear, Stat, Symmetry, GROUPS};
use testdata::{group, record, sub, zstr};
use world::chargen::facegen;
use world::chargen::race_sex::{self, Dice, PlayerFace};

const OLD: u32 = 0x100;
const ADULT: u32 = 0x101;
const CHILD: u32 = 0x102;
const OTHER_OLD: u32 = 0x103;
const OTHER_ADULT: u32 = 0x104;
const OTHER_CHILD: u32 = 0x105;
const HAIR_A: u32 = 0x200;
const HAIR_B: u32 = 0x201;
const EYES_A: u32 = 0x210;
const EYES_B: u32 = 0x211;
const BEARD: u32 = 0x220;
const PRESET_Z: u32 = 0x300;
const PRESET_A: u32 = 0x301;
const NOT_PRESET: u32 = 0x302;
const WOMAN: u32 = 0x303;
const TEMPLATED: u32 = 0x304;
const DAD: u32 = 0x305;

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn acbs(flags: u32) -> Vec<u8> {
    let mut d = flags.to_le_bytes().to_vec();
    d.extend([0; 20]);
    d
}

#[allow(clippy::too_many_arguments)]
fn person(
    id: u32,
    name: &str,
    flags: u32,
    race: u32,
    hair: u32,
    eyes: u32,
    shape: &[f32],
    tone: &[f32],
    template: bool,
) -> Vec<u8> {
    let mut d = sub(b"EDID", &zstr(&format!("Person{id:x}")));
    d.extend(sub(b"FULL", &zstr(name)));
    d.extend(sub(b"ACBS", &acbs(flags)));
    d.extend(sub(b"RNAM", &race.to_le_bytes()));
    d.extend(sub(b"HNAM", &hair.to_le_bytes()));
    d.extend(sub(b"ENAM", &eyes.to_le_bytes()));
    d.extend(sub(b"HCLR", &[10, 20, 30, 0]));
    d.extend(sub(b"FGGS", &floats(shape)));
    d.extend(sub(b"FGTS", &floats(tone)));
    if template {
        d.extend(sub(b"TPLT", &ADULT.to_le_bytes()));
    }
    record(b"NPC_", id, &d)
}

fn race(id: u32, name: &str, older: u32, younger: u32, male: f32) -> Vec<u8> {
    let mut d = sub(b"EDID", &zstr(name));
    d.extend(sub(b"FULL", &zstr(name)));
    let mut data = vec![0u8; 36];
    data[32] = 1;
    // The child race stands 0.8 tall, both sexes (as FO3's do).
    if id == CHILD {
        data[16..20].copy_from_slice(&0.8f32.to_le_bytes());
        data[20..24].copy_from_slice(&0.8f32.to_le_bytes());
    }
    d.extend(sub(b"DATA", &data));
    if older != 0 {
        d.extend(sub(b"ONAM", &older.to_le_bytes()));
    }
    if younger != 0 {
        d.extend(sub(b"YNAM", &younger.to_le_bytes()));
    }
    let mut dnam = HAIR_B.to_le_bytes().to_vec();
    dnam.extend(HAIR_B.to_le_bytes());
    d.extend(sub(b"DNAM", &dnam));
    d.extend(sub(b"HNAM", &floats_of_forms(&[HAIR_A, HAIR_B])));
    d.extend(sub(b"ENAM", &floats_of_forms(&[EYES_A, EYES_B])));
    d.extend(sub(b"MNAM", &[]));
    d.extend(sub(b"FGGS", &floats(&[male, 0.0])));
    d.extend(sub(b"FGTS", &floats(&[male, 0.0])));
    d.extend(sub(b"FNAM", &[]));
    d.extend(sub(b"FGGS", &floats(&[0.0, male])));
    d.extend(sub(b"FGTS", &floats(&[0.0, male])));
    record(b"RACE", id, &d)
}

fn floats_of_forms(ids: &[u32]) -> Vec<u8> {
    ids.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn plugin() -> Vec<u8> {
    let mut hedr = 0.94f32.to_le_bytes().to_vec();
    hedr.extend([0; 8]);
    let mut plugin = record(b"TES4", 0, &sub(b"HEDR", &hedr));

    let mut races = race(OLD, "Old", 0, ADULT, 0.0);
    races.extend(race(ADULT, "Adult", OLD, CHILD, 1.0));
    races.extend(race(CHILD, "Child", ADULT, 0, 0.0));
    races.extend(race(OTHER_OLD, "OtherOld", 0, OTHER_ADULT, 0.0));
    races.extend(race(OTHER_ADULT, "OtherAdult", OTHER_OLD, OTHER_CHILD, 2.0));
    races.extend(race(OTHER_CHILD, "OtherChild", OTHER_ADULT, 0, 0.0));
    plugin.extend(group(*b"RACE", 0, &races));

    let part = |id: u32, kind: &[u8; 4], flags: u8| {
        let mut d = sub(b"EDID", &zstr(&format!("Part{id:x}")));
        d.extend(sub(b"DATA", &[flags]));
        record(kind, id, &d)
    };
    let mut hair = part(HAIR_A, b"HAIR", 1);
    hair.extend(part(HAIR_B, b"HAIR", 1));
    plugin.extend(group(*b"HAIR", 0, &hair));
    let mut eyes = part(EYES_A, b"EYES", 1);
    eyes.extend(part(EYES_B, b"EYES", 1));
    plugin.extend(group(*b"EYES", 0, &eyes));
    plugin.extend(group(*b"HDPT", 0, &part(BEARD, b"HDPT", 1)));

    let mut people = person(0x7, "", 0, ADULT, HAIR_A, EYES_A, &[1.0, 2.0], &[0.5, 0.0], false);
    people.extend(person(PRESET_Z, "Zed", 4, ADULT, HAIR_B, EYES_B, &[3.0, 0.0], &[0.0, 0.0], false));
    people.extend(person(PRESET_A, "Abe", 4, ADULT, HAIR_A, EYES_A, &[4.0, 0.0], &[0.0, 0.0], false));
    people.extend(person(NOT_PRESET, "Carl", 0, ADULT, HAIR_A, EYES_A, &[5.0, 0.0], &[0.0, 0.0], false));
    people.extend(person(WOMAN, "Wendy", 1, ADULT, HAIR_A, EYES_A, &[6.0, 0.0], &[0.0, 0.0], false));
    people.extend(person(TEMPLATED, "Temp", 0, OTHER_ADULT, HAIR_A, EYES_A, &[7.0, 0.0], &[0.0, 0.0], true));
    people.extend(person(DAD, "Dad", 0, OTHER_ADULT, HAIR_A, EYES_A, &[8.0, 4.0], &[0.25, 0.75], false));
    plugin.extend(group(*b"NPC_", 0, &people));
    plugin
}

fn order(name: &str) -> (testdata::TempData, LoadOrder) {
    let data = testdata::quests(name);
    data.write("Fallout3.esm", &plugin());
    let order = LoadOrder::from_data_dir(data.path(), &ActivePlugins::OfficialOnly).unwrap();
    (data, order)
}

fn controls() -> Ctl {
    let linear = |coefficients: Vec<f32>, offset: f32| Linear {
        coefficients,
        offset,
    };
    let stats = [
        [linear(vec![1.0, 0.0], 30.0), linear(vec![1.0, 0.0], 30.0)],
        [linear(vec![0.0, 1.0], 0.0), linear(vec![0.0, 1.0], 0.0)],
    ];
    Ctl {
        basis_ids: [0, 0],
        sizes: [[2, 0], [2, 0]],
        controls: [
            [
                vec![Control {
                    label: "First".into(),
                    coefficients: vec![1.0, 0.0],
                }],
                vec![],
            ],
            [
                vec![Control {
                    label: "Tone".into(),
                    coefficients: vec![0.0, 1.0],
                }],
                vec![],
            ],
        ],
        stats: vec![stats; GROUPS],
        differences: vec![],
        distributions: vec![],
    }
}

#[test]
fn presets_are_those_flagged_sorted_by_name_and_all_when_none_is() {
    let (_data, order) = order("rsm-presets");
    // Zed and Abe are flagged, Carl isn't, Wendy is a woman; never the player.
    assert_eq!(
        race_sex::presets(&order, FormId(ADULT), false),
        [FormId(PRESET_A), FormId(PRESET_Z)]
    );
    // Nobody flagged among the women of the race: all of them.
    assert_eq!(race_sex::presets(&order, FormId(ADULT), true), [FormId(WOMAN)]);
    assert!(race_sex::presets(&order, FormId(CHILD), false).is_empty());
}

#[test]
fn a_preset_gives_its_parts_and_a_templated_one_the_races_hair() {
    let (_data, order) = order("rsm-preset");
    let mut face = PlayerFace::of_npc(&order, FormId(0x7)).unwrap();
    assert_eq!(face.hair, Some(FormId(HAIR_A)));
    assert_eq!(face.hair_color, [10, 20, 30]);

    race_sex::apply_preset(&order, &mut face, FormId(PRESET_Z), false);
    assert_eq!(face.hair, Some(FormId(HAIR_B)));
    assert_eq!(face.eyes, Some(FormId(EYES_B)));
    assert_eq!(face.own.0[0][0], [3.0, 0.0]);

    race_sex::apply_preset(&order, &mut face, FormId(TEMPLATED), false);
    assert_eq!(face.hair, Some(FormId(HAIR_B)), "the race's default, DNAM");
    assert_eq!(face.own.0[0][0], [7.0, 0.0]);
}

#[test]
fn the_race_changes_and_the_first_preset_comes_with_it() {
    let (_data, order) = order("rsm-race");
    let mut face = PlayerFace::of_npc(&order, FormId(0x7)).unwrap();
    race_sex::change_race(&order, &mut face, FormId(ADULT), false);
    assert_eq!(face.race, FormId(ADULT));
    assert_eq!(face.own.0[0][0], [4.0, 0.0], "Abe's, first by name");
    assert_eq!(face.eyes, Some(FormId(EYES_A)));
}

#[test]
fn female_empties_the_head_parts_and_male_keeps_them() {
    let (_data, order) = order("rsm-sex");
    let mut face = PlayerFace::of_npc(&order, FormId(0x7)).unwrap();
    face.head_parts = vec![FormId(BEARD)];
    race_sex::change_sex(&order, &mut face, true);
    assert_eq!(face.own.0[0][0], [6.0, 0.0], "Wendy's");
    assert!(face.head_parts.is_empty());
}

#[test]
fn a_slider_sets_the_total_and_stores_it_less_the_race() {
    let (_data, order) = order("rsm-slider");
    let ctl = controls();
    let mut face = PlayerFace::of_npc(&order, FormId(0x7)).unwrap();
    // Own 1 on the race's 1.
    let total = race_sex::total(&order, &face, false);
    assert_eq!(total.get(Kind::Shape, Symmetry::Symmetric), [2.0, 2.0]);
    race_sex::set_control(&order, &ctl, &mut face, false, Kind::Shape, 0, 5.0);
    assert_eq!(face.own.0[0][0], [4.0, 2.0], "5 less the race's 1");
    assert_eq!(
        race_sex::control_values(&ctl, &race_sex::total(&order, &face, false), Kind::Shape),
        [5.0]
    );
}

struct Fixed;

impl Dice for Fixed {
    fn normal(&mut self) -> f32 {
        1.0
    }
    fn uniform(&mut self) -> f32 {
        // A gender of 1.
        0.75
    }
}

#[test]
fn randomize_starts_from_the_race_and_sets_a_new_gender() {
    let (_data, order) = order("rsm-random");
    let ctl = controls();
    let mut face = PlayerFace::of_npc(&order, FormId(0x7)).unwrap();
    race_sex::randomize(&order, &ctl, &mut face, false, &mut Fixed);
    let total = race_sex::total(&order, &face, false);
    // Age (the first coordinate here) as it came: the race's 1 plus 0.75.
    assert_eq!(total.get(Kind::Shape, Symmetry::Symmetric)[0], 1.75);
    assert_eq!(facegen::stat(&ctl, &total, Stat::Gender, Kind::Shape), 1.0);
}

#[test]
fn matching_a_race_keeps_the_age() {
    let (_data, order) = order("rsm-match-race");
    // An adult stays an adult of the other race, and so on.
    assert_eq!(
        race_sex::match_race(&order, FormId(ADULT), FormId(OTHER_ADULT)),
        FormId(OTHER_ADULT)
    );
    assert_eq!(
        race_sex::match_race(&order, FormId(ADULT), FormId(OTHER_CHILD)),
        FormId(OTHER_ADULT)
    );
    assert_eq!(
        race_sex::match_race(&order, FormId(CHILD), FormId(OTHER_OLD)),
        FormId(OTHER_CHILD)
    );
    assert_eq!(
        race_sex::match_race(&order, FormId(OLD), FormId(OTHER_CHILD)),
        FormId(OTHER_OLD)
    );
}

#[test]
fn ageing_a_race_follows_its_links_and_stops_at_the_end() {
    let (_data, order) = order("rsm-age");
    // CG02's `player.AgeRace -1` is commented "change player to a child".
    assert_eq!(race_sex::age_race(&order, FormId(ADULT), -1), FormId(CHILD));
    assert_eq!(race_sex::age_race(&order, FormId(CHILD), 1), FormId(ADULT));
    assert_eq!(race_sex::age_race(&order, FormId(ADULT), 1), FormId(OLD));
    assert_eq!(race_sex::age_race(&order, FormId(ADULT), 0), FormId(ADULT));
    assert_eq!(race_sex::age_race(&order, FormId(CHILD), -5), FormId(CHILD));
}

#[test]
fn a_young_player_shows_the_races_default_hair_but_keeps_their_own() {
    let (_data, order) = order("rsm-young");
    let mut state = world::scripting::GameState::default();
    state.player_face = PlayerFace::of_npc(&order, FormId(0x7));
    assert_eq!(race_sex::shown(&order, &state).unwrap().hair, Some(FormId(HAIR_A)));
    state.player_young = true;
    assert_eq!(race_sex::shown(&order, &state).unwrap().hair, Some(FormId(HAIR_B)));
    assert_eq!(state.player_face.as_ref().unwrap().hair, Some(FormId(HAIR_A)));
    state.player_young = false;
    assert_eq!(race_sex::shown(&order, &state).unwrap().hair, Some(FormId(HAIR_A)));
}

/// The player's whole scale (Fallout3.exe `004e6d80`): the script's
/// `SetScale` × the race's height, as the nursery's toddler (0.4) and the
/// child race (0.8) make it; what the eye and the jump scale by.
#[test]
fn the_players_scale_is_the_scripts_times_the_races_height() {
    let (_data, order) = order("rsm-scale");
    let mut state = world::scripting::GameState::default();
    let mut face = PlayerFace::of_npc(&order, FormId(0x7)).unwrap();
    assert_eq!(race_sex::player_scale(&order, &state), 1.0);
    state.scales.insert(world::dialogue::PLAYER_REF, 0.4);
    assert!((race_sex::player_scale(&order, &state) - 0.4).abs() < 1e-6);
    face.race = FormId(CHILD);
    state.player_face = Some(face);
    assert!((race_sex::player_scale(&order, &state) - 0.32).abs() < 1e-6);
}

#[test]
fn matching_a_face_goes_part_way_keeping_age_and_skin() {
    let (_data, order) = order("rsm-match-face");
    let ctl = controls();
    let player = PlayerFace::of_npc(&order, FormId(0x7)).unwrap();
    let mut dad = PlayerFace::of_npc(&order, FormId(DAD)).unwrap();
    let own_tone = dad.own.0[1][0].clone();
    let own_age = facegen::stat(&ctl, &dad.own, Stat::Age, Kind::Shape);
    race_sex::match_face(&order, &ctl, &mut dad, &player, 50.0);
    assert_eq!(dad.own.0[1][0], own_tone, "the texture stays their own");
    // Halfway from (8, 4) to (1, 2) is (4.5, 3); the age, read from the first
    // coordinate here, back to 8; then less the race's male (2, 0).
    assert_eq!(dad.own.0[0][0], [6.0, 3.0]);
    assert_eq!(own_age, 38.0);
}

#[test]
fn the_face_is_saved_and_read_back() {
    let (_data, order) = order("rsm-save");
    let mut state = world::scripting::GameState::default();
    let mut face = PlayerFace::of_npc(&order, FormId(0x7)).unwrap();
    face.head_parts = vec![FormId(BEARD)];
    face.hair_length = 0.5;
    state.player_face = Some(face.clone());
    state.npc_faces.insert(FormId(DAD), face.clone());
    let text = world::save::save(&state, None);
    let (back, _) = world::save::load(&text).unwrap();
    assert_eq!(back.player_face.as_ref(), Some(&face));
    assert_eq!(back.npc_faces.get(&FormId(DAD)), Some(&face));
}
