//! Which loading screens the main menu shows (`world::loading_screens`), on a
//! plugin built from scratch: the records flagged "Displays in Main Menu"
//! (`0x400`, Fallout3.exe `00658b60`), picked at random without repeats up to
//! the most it keeps (`00658050`).

use esm::{ActivePlugins, FormId, LoadOrder};
use testdata::{group, record, sub, zstr};
use world::loading_screens::{self, MAIN_MENU_FLAG};

fn screen(id: u32, flags: u32, icon: &str) -> Vec<u8> {
    let mut d = sub(b"EDID", &zstr(&format!("Screen{id:x}")));
    if !icon.is_empty() {
        d.extend(sub(b"ICON", &zstr(icon)));
    }
    d.extend(sub(b"DESC", &zstr("Words.")));
    let mut r = record(b"LSCR", id, &d);
    r[8..12].copy_from_slice(&flags.to_le_bytes());
    r
}

fn order(name: &str) -> (testdata::TempData, LoadOrder) {
    let mut hedr = 0.94f32.to_le_bytes().to_vec();
    hedr.extend([0; 8]);
    let mut plugin = record(b"TES4", 0, &sub(b"HEDR", &hedr));
    let mut screens = screen(0x100, MAIN_MENU_FLAG, "Interface\\Loading\\A.dds");
    screens.extend(screen(0x101, 0, "Interface\\Loading\\NotOnTheMenu.dds"));
    screens.extend(screen(0x102, MAIN_MENU_FLAG | 1, "Interface\\Loading\\B.dds"));
    screens.extend(screen(0x103, MAIN_MENU_FLAG, ""));
    // In the game: two for the clinic (one picture twice), one for anywhere.
    let mut at = |id: u32, icon: &str, place: Option<u32>| {
        let mut d = sub(b"EDID", &zstr(&format!("Screen{id:x}")));
        d.extend(sub(b"ICON", &zstr(icon)));
        d.extend(sub(b"DESC", &zstr("Here.")));
        if let Some(p) = place {
            let mut l = p.to_le_bytes().to_vec();
            l.extend([0; 8]);
            d.extend(sub(b"LNAM", &l));
        }
        screens.extend(record(b"LSCR", id, &d));
    };
    at(0x110, "Interface\\Loading\\Clinic1.dds", Some(0x200));
    at(0x111, "Interface\\Loading\\Clinic2.dds", Some(0x200));
    at(0x112, "Interface\\Loading\\Clinic1.dds", Some(0x200));
    plugin.extend(group(*b"LSCR", 0, &screens));
    let data = testdata::quests(name);
    data.write("Fallout3.esm", &plugin);
    let order = LoadOrder::from_data_dir(data.path(), &ActivePlugins::OfficialOnly).unwrap();
    (data, order)
}

#[test]
fn the_main_menu_takes_the_flagged_screens_with_pictures() {
    let (_data, order) = order("lscr-main");
    let screens = loading_screens::main_menu(&order);
    let forms: Vec<FormId> = screens.iter().map(|s| s.form).collect();
    assert_eq!(forms, [FormId(0x100), FormId(0x102)]);
    assert_eq!(screens[0].picture, "textures\\interface\\loading\\a.dds");
    assert_eq!(screens[0].text, "Words.");
}

#[test]
fn picking_takes_each_once_up_to_the_most() {
    let (_data, order) = order("lscr-pick");
    let screens = loading_screens::main_menu(&order);
    // Always the last of those left.
    let picked = loading_screens::pick(screens.clone(), 20, |n| n - 1);
    let forms: Vec<FormId> = picked.iter().map(|s| s.form).collect();
    assert_eq!(forms, [FormId(0x102), FormId(0x100)]);
    assert_eq!(loading_screens::pick(screens, 1, |_| 0).len(), 1);
}

#[test]
fn a_load_in_the_game_shows_its_places_screens_then_any() {
    use loading_screens::Destination;
    let (_data, order) = order("lscr-game");
    let forms = |v: Vec<loading_screens::LoadScreen>| -> Vec<u32> { v.iter().map(|s| s.form.0).collect() };
    // `00659400`: one for the clinic (`iNumLocationSpecificScreens`), then
    // those for anywhere, up to four.
    let shown = loading_screens::in_game(&order, Some(Destination::Cell(FormId(0x200))), false, |_| 0);
    assert_eq!(shown.len(), 4, "{:?}", forms(shown.clone()));
    assert!([0x110, 0x112].contains(&shown[0].form.0));
    assert!(shown[1..].iter().all(|s| ![0x110, 0x111, 0x112].contains(&s.form.0)));
    // `SetLocationSpecificLoadScreensOnly 1`: the clinic's alone, a picture
    // once (`00657ec0`).
    let only = forms(loading_screens::in_game(&order, Some(Destination::Cell(FormId(0x200))), true, |_| 0));
    assert_eq!(only.len(), 2, "{only:?}");
    assert!(only.contains(&0x111));
    // Outdoors no place's screens match: those for anywhere.
    let out = forms(loading_screens::in_game(&order, Some(Destination::Outdoors), false, |_| 0));
    assert!(out.iter().all(|f| ![0x110, 0x111, 0x112].contains(f)));
    assert!(loading_screens::in_game(&order, Some(Destination::Outdoors), true, |_| 0).is_empty());
}
